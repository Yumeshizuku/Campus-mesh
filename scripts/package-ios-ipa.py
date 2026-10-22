#!/usr/bin/env python3
"""Package and verify an unsigned iPhone IPA, without Apple SDK dependencies."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import plistlib
import re
import stat
import struct
import tempfile
import zipfile


class InvalidIPA(ValueError):
    pass


SIGNING_SUFFIXES = {
    '.mobileprovision', '.provisionprofile', '.p12', '.pfx', '.pem',
    '.key', '.keystore', '.jks',
}
MAX_FILE_BYTES = 512 * 1024 * 1024
MAX_PAYLOAD_BYTES = 1024 * 1024 * 1024


def require(condition, message):
    if not condition:
        raise InvalidIPA(message)


def packed_version(value):
    return f'{value >> 16}.{(value >> 8) & 255}.{value & 255}'


def inspect_executable(data):
    """arm64 alone is insufficient: Apple Silicon simulators also use arm64."""
    require(len(data) >= 32 and data[:4] == b'\xcf\xfa\xed\xfe',
            'Executable must be a thin little-endian 64-bit Mach-O.')
    _, cpu, _, kind, count, command_bytes, _, _ = struct.unpack_from('<8I', data)
    require(cpu == 0x0100000C, 'Executable architecture must be arm64.')
    require(kind == 2, 'Mach-O must be an application executable (MH_EXECUTE).')
    end = 32 + command_bytes
    require(end <= len(data) and count <= command_bytes // 8,
            'Invalid Mach-O load-command bounds.')
    offset, platform, minimum, sdk = 32, None, None, None
    for _ in range(count):
        require(offset + 8 <= end, 'Truncated Mach-O load command.')
        command, size = struct.unpack_from('<2I', data, offset)
        require(size >= 8 and size % 8 == 0 and offset + size <= end,
                'Invalid Mach-O load-command size.')
        if command == 0x32:  # LC_BUILD_VERSION
            require(size >= 24 and platform is None, 'Invalid or duplicate LC_BUILD_VERSION.')
            _, _, platform, minimum, sdk, tools = struct.unpack_from('<6I', data, offset)
            require(24 + tools * 8 <= size, 'Truncated Mach-O build-tool metadata.')
        if command == 0x1D:  # LC_CODE_SIGNATURE, including ad-hoc signatures
            require(size >= 16, 'Invalid LC_CODE_SIGNATURE.')
            _, _, _, signature_size = struct.unpack_from('<4I', data, offset)
            require(signature_size == 0,
                    'Executable has a code signature; refusing to label it unsigned.')
        offset += size
    require(offset == end, 'Mach-O load-command size does not match its header.')
    require(platform != 7, 'Simulator executable rejected: platform is IOSSIMULATOR.')
    require(platform == 2, 'Executable must declare LC_BUILD_VERSION platform IOS (2).')
    return {
        'architecture': 'arm64', 'macho_platform': 'IOS',
        'macho_minimum_os_version': packed_version(minimum),
        'sdk_version': packed_version(sdk),
    }


def check_name(path):
    require('_CodeSignature' not in path.parts,
            'Signed bundle resources found; refusing to label the app unsigned.')
    require(path.suffix.lower() not in SIGNING_SUFFIXES,
            f'Signing profile or private-key material must not be packaged: {path}')
    require(path.name.lower() not in {'embedded.mobileprovision', 'embedded.provisionprofile'},
            f'Provisioning profile must not be packaged: {path}')


def check_private_key(data, name):
    require(not re.search(
        rb'-----BEGIN (?:[A-Z0-9]+ )?PRIVATE KEY-----[\r\n]', data),
        f'Private-key PEM material found in {name}.')


def inspect_plist(data):
    info = plistlib.loads(data)
    require(isinstance(info, dict), 'Info.plist must contain a dictionary.')
    require(info.get('CFBundlePackageType') == 'APPL', 'Bundle is not an application.')
    require(info.get('CFBundleSupportedPlatforms') == ['iPhoneOS'],
            'CFBundleSupportedPlatforms must be [iPhoneOS], not a simulator platform.')
    require(info.get('DTPlatformName', 'iphoneos') == 'iphoneos',
            'DTPlatformName must identify iphoneos.')
    executable = info.get('CFBundleExecutable', '')
    require(isinstance(executable, str) and executable not in {'', '.', '..'}
            and '/' not in executable and '\\' not in executable,
            'Invalid CFBundleExecutable.')
    version, build = info.get('CFBundleShortVersionString'), info.get('CFBundleVersion')
    require(isinstance(version, str) and re.fullmatch(r'\d+\.\d+\.\d+', version),
            'Marketing version must contain three numeric components.')
    require(isinstance(build, str) and re.fullmatch(r'\d+(?:\.\d+){0,2}', build),
            'Build version must contain one to three numeric components.')
    require(info.get('CMReleaseChannel') == 'alpha', 'Release channel must be alpha.')
    minimum = info.get('MinimumOSVersion')
    require(isinstance(minimum, str) and re.fullmatch(r'\d+(?:\.\d+){1,2}', minimum),
            'MinimumOSVersion must be numeric.')
    bundle_id = info.get('CFBundleIdentifier')
    require(isinstance(bundle_id, str) and re.fullmatch(r'[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+', bundle_id),
            'Invalid CFBundleIdentifier.')
    return executable, {
        'bundle_identifier': bundle_id, 'marketing_version': version,
        'build_version': build, 'release_channel': 'alpha',
        'release_name': f'alpha-{version}', 'minimum_os_version': minimum,
        'platform': 'iPhoneOS', 'signing': 'unsigned',
        'installable_without_signing': False, 'requires_signing': True,
    }


def verify(ipa_path):
    ipa_path = Path(ipa_path)
    with zipfile.ZipFile(ipa_path) as archive:
        entries = archive.infolist()
        require(entries, 'IPA is empty.')
        seen, files, apps, total = set(), {}, set(), 0
        for entry in entries:
            name = entry.filename
            require(name and not name.startswith('/') and '\\' not in name and '\0' not in name,
                    'Unsafe archive path.')
            parts = name.rstrip('/').split('/')
            require(all(part not in {'', '.', '..'} for part in parts), 'Unsafe archive path.')
            canonical_name = name.rstrip('/')
            require(canonical_name not in seen, f'Duplicate archive entry: {name}')
            seen.add(canonical_name)
            require(parts[0] == 'Payload', 'IPA must contain only a top-level Payload directory.')
            mode = entry.external_attr >> 16
            expected_kind = stat.S_IFDIR if entry.is_dir() else stat.S_IFREG
            require(entry.create_system == 3 and stat.S_IFMT(mode) == expected_kind,
                    f'Only regular files/directories with Unix modes are supported: {name}')
            require(not mode & 0o7000, f'Special permission bits are not supported: {name}')
            if len(parts) == 1:
                require(entry.is_dir(), 'Payload must be a directory.')
                continue
            require(parts[1].endswith('.app'), 'Payload must contain one .app bundle.')
            apps.add(parts[1])
            check_name(PurePosixPath(*parts[2:]))
            total += entry.file_size
            require(entry.file_size <= MAX_FILE_BYTES and total <= MAX_PAYLOAD_BYTES,
                    'IPA exceeds the verification size limit.')
            if not entry.is_dir():
                require(len(parts) >= 3, '.app bundle must be a directory.')
                data = archive.read(entry)  # Includes ZIP CRC verification.
                check_private_key(data, name)
                files[name] = entry
        require(len(apps) == 1, 'IPA must contain exactly one application bundle.')
        prefix = 'Payload/' + next(iter(apps))
        plist_name = prefix + '/Info.plist'
        require(plist_name in files, 'Application Info.plist is missing.')
        executable, metadata = inspect_plist(archive.read(plist_name))
        executable_name = prefix + '/' + executable
        require(executable_name in files, 'Application executable is missing.')
        require((files[executable_name].external_attr >> 16) & 0o111,
                'Application executable does not retain executable permission.')
        metadata.update(inspect_executable(archive.read(executable_name)))
        metadata.update({'app_bundle': prefix, 'executable': executable, 'files': len(files)})
    with ipa_path.open('rb') as source:
        hasher = hashlib.sha256()
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            hasher.update(chunk)
        digest = hasher.hexdigest()
    return dict(metadata, artifact=ipa_path.name, bytes=ipa_path.stat().st_size, sha256=digest)


def package(app_path, output_dir):
    app = Path(app_path)
    require(not app.is_symlink(), 'Symlink app bundles are not supported.')
    app = app.resolve(strict=True)
    require(app.is_dir() and app.suffix == '.app', '--app must be a .app directory.')
    output = Path(output_dir).resolve()
    require(output != app and app not in output.parents, 'Output directory must be outside the app.')
    source_entries = [app, *sorted(app.rglob('*'))]
    for path in source_entries:
        require(not path.is_symlink(), f'Symlinks are not supported: {path.relative_to(app.parent)}')
        mode = path.stat().st_mode
        require(stat.S_ISDIR(mode) or stat.S_ISREG(mode), f'Unsupported file type: {path}')
        check_name(path.relative_to(app))
    plist = app / 'Info.plist'
    require(plist.is_file(), 'Application Info.plist is missing.')
    _, metadata = inspect_plist(plist.read_bytes())
    output.mkdir(parents=True, exist_ok=True)
    destination = output / f'CampusMesh-{metadata["release_name"]}-unsigned.ipa'
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=output, prefix='.ipa-', suffix='.tmp', delete=False) as staging:
            temporary = Path(staging.name)
        with zipfile.ZipFile(temporary, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
            payload = zipfile.ZipInfo('Payload/')
            payload.create_system = 3
            payload.external_attr = (stat.S_IFDIR | 0o755) << 16 | 0x10
            archive.writestr(payload, b'')
            for path in source_entries:
                name = (PurePosixPath('Payload') / app.name / path.relative_to(app).as_posix()).as_posix()
                archive.write(path, name)
        result = verify(temporary)
        os.replace(temporary, destination)
        temporary = None
        result['artifact'] = destination.name
        checksum = destination.with_name(destination.name + '.sha256')
        metadata_path = destination.with_name(destination.name + '.json')
        result.update({'ipa_path': str(destination), 'checksum_path': str(checksum),
                       'metadata_path': str(metadata_path)})
        checksum.write_text(result['sha256'] + '  ' + destination.name + '\n')
        metadata_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
        return result
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    pack = commands.add_parser('package', help='Package a real device .app without signing it')
    pack.add_argument('--app', required=True)
    pack.add_argument('--output-dir', required=True)
    check = commands.add_parser('verify', help='Verify an unsigned IPA on macOS or Linux')
    check.add_argument('ipa')
    args = parser.parse_args()
    try:
        result = package(args.app, args.output_dir) if args.command == 'package' else verify(args.ipa)
    except (InvalidIPA, OSError, plistlib.InvalidFileException, zipfile.BadZipFile, struct.error) as error:
        parser.exit(1, f'IPA verification failed: {error}\n')
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Package only the APK produced by the current Gradle build, with provenance."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def package(root, output, env):
    def git(*args):
        return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()

    sha = git('rev-parse', 'HEAD')
    if env.get('GITHUB_SHA') and env['GITHUB_SHA'] != sha:
        raise ValueError('Checkout SHA differs from GITHUB_SHA')
    if git('status', '--porcelain', '--untracked-files=no'):
        raise ValueError('Tracked source files are modified; commit before packaging')
    source = (root / 'android/app/build.gradle.kts').read_text()
    version = re.search(r'versionName\s*=\s*"([^"]+)"', source).group(1)
    code = int(re.search(r'versionCode\s*=\s*(\d+)', source).group(1))
    if not re.fullmatch(r'alpha-\d+\.\d+\.\d+', version):
        raise ValueError('Expected alpha-<major>.<minor>.<patch>')
    tag = env.get('GITHUB_REF_NAME') if env.get('GITHUB_REF_TYPE') == 'tag' else None
    if tag and tag.startswith('android-') and tag != f'android-{version}':
        raise ValueError('Android tag must match the APK versionName')
    build = root / 'android/app/build/outputs/apk/debug'
    metadata = json.loads((build / 'output-metadata.json').read_text())
    elements = metadata['elements']
    if len(elements) != 1 or metadata['variantName'] != 'debug':
        raise ValueError('Expected exactly one debug APK')
    item = elements[0]
    if (item['versionName'], item['versionCode'], metadata['applicationId']) != (version, code, 'org.campusmesh'):
        raise ValueError('Gradle output metadata differs from source version/application ID')
    apk = build / item['outputFile']
    if apk.parent.resolve() != build.resolve() or apk.suffix != '.apk':
        raise ValueError('APK must be inside the current debug output directory')
    with zipfile.ZipFile(apk) as archive:
        if archive.testzip() is not None:
            raise ValueError('Corrupt APK')
        libs = {name.split('/')[1] for name in archive.namelist() if name.startswith('lib/') and name.endswith('.so')}
        if libs != {'arm64-v8a'} or 'lib/arm64-v8a/libcampus_mesh.so' not in archive.namelist():
            raise ValueError('Expected ARM64-only APK with Rust core')
    run = env.get('GITHUB_RUN_ID', 'local')
    attempt = env.get('GITHUB_RUN_ATTEMPT', '1')
    if not re.fullmatch(r'\d+|local', run) or not attempt.isdigit():
        raise ValueError('Invalid run identity')
    name = f'CampusMesh-{version}-vc{code}-debug-arm64-{sha}-run{run}-{attempt}'
    output.mkdir(parents=True, exist_ok=False)
    dest = output / f'{name}.apk'
    shutil.copyfile(apk, dest)
    digest = hashlib.sha256(dest.read_bytes()).hexdigest()
    (output / f'{dest.name}.sha256').write_text(f'{digest}  {dest.name}\n')
    repo = env.get('GITHUB_REPOSITORY')
    server = env.get('GITHUB_SERVER_URL', 'https://github.com')
    provenance = {
        'schemaVersion': 1, 'applicationId': metadata['applicationId'],
        'versionName': version, 'versionCode': code, 'variant': 'debug',
        'abi': 'arm64-v8a',
        'minSdk': int(re.search(r'minSdk\s*=\s*(\d+)', source).group(1)),
        'targetSdk': int(re.search(r'targetSdk\s*=\s*(\d+)', source).group(1)),
        'signing': 'development debug key; not a production release signature',
        'sourceCommit': sha, 'sourceRef': env.get('GITHUB_REF'),
        'repository': repo, 'runId': run, 'runAttempt': attempt,
        'runUrl': f'{server}/{repo}/actions/runs/{run}' if repo and run != 'local' else None,
        'workflow': env.get('GITHUB_WORKFLOW'),
        'apk': dest.name, 'apkSha256': digest,
        'validation': 'See workflow run for build, lint, signature, alignment and all job results; metadata alone is not an attestation',
    }
    (output / 'build-info.json').write_text(json.dumps(provenance, indent=2) + '\n')
    (output / 'SOURCE_COMMIT.txt').write_text(sha + '\n')
    (output / 'README.txt').write_text(
        f'Campus Mesh Android {version} (versionCode {code})\n'
        f'Commit: {sha}\nRun: {provenance["runUrl"]}\n'
        'ARM64, Android 8.0+. Development debug signature.\n'
        'Verify: sha256sum -c *.apk.sha256\n'
        'A build artifact does not imply every Verify job passed; check the run.\n'
        'Debug keys can differ between CI runs and historical local builds; Android may reject an in-place update.\n'
        'Back up needed data before uninstalling. Both test phones should install the SAME APK.\n'
        'Bluetooth/LAN/multi-hop behavior still requires real-device tests.\n')
    if env.get('GITHUB_OUTPUT'):
        with open(env['GITHUB_OUTPUT'], 'a') as stream:
            stream.write(f'artifact={name}\nversion={version}\n')
    return provenance


if __name__ == '__main__':
    print(json.dumps(package(ROOT, ROOT / 'android-delivery', os.environ), indent=2))

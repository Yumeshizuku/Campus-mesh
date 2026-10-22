"""Focused format checks using synthetic Mach-O fixtures, not an iOS build."""
import importlib.util
from pathlib import Path
import plistlib
import stat
import struct
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('ipa', Path(__file__).parents[1] / 'package-ios-ipa.py')
ipa = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ipa)


def macho(platform=2, signed=False):
    commands = struct.pack('<6I', 0x32, 24, platform, 16 << 16, 26 << 16, 0)
    if signed:
        commands += struct.pack('<4I', 0x1D, 16, 72, 4)
    header = struct.pack('<8I', 0xFEEDFACF, 0x0100000C, 0, 2, 1 + signed, len(commands), 0, 0)
    return header + commands + (b'sign' if signed else b'')


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.app = self.root / 'CampusMesh.app'
        self.app.mkdir()
        info = {
            'CFBundlePackageType': 'APPL', 'CFBundleSupportedPlatforms': ['iPhoneOS'],
            'DTPlatformName': 'iphoneos', 'CFBundleExecutable': 'CampusMesh',
            'CFBundleIdentifier': 'org.campusmesh.app', 'CFBundleShortVersionString': '0.4.0',
            'CFBundleVersion': '5', 'CMReleaseChannel': 'alpha', 'MinimumOSVersion': '16.0',
        }
        (self.app / 'Info.plist').write_bytes(plistlib.dumps(info, fmt=plistlib.FMT_BINARY))
        self.binary = self.app / 'CampusMesh'
        self.binary.write_bytes(macho())
        self.binary.chmod(0o755)

    def package(self):
        return ipa.package(self.app, self.root / 'output')

    def test_device_payload_and_modes_round_trip(self):
        result = self.package()
        self.assertEqual(result['artifact'], 'CampusMesh-alpha-0.4.0-unsigned.ipa')
        self.assertFalse(result['installable_without_signing'])
        self.assertEqual(result['signing'], 'unsigned')
        verified = ipa.verify(result['ipa_path'])
        self.assertEqual(verified['sha256'], result['sha256'])
        with zipfile.ZipFile(result['ipa_path']) as archive:
            entry = archive.getinfo('Payload/CampusMesh.app/CampusMesh')
            self.assertEqual(stat.S_IMODE(entry.external_attr >> 16), 0o755)
            self.assertEqual(archive.read(entry), macho())
        self.assertTrue(Path(result['checksum_path']).is_file())
        self.assertTrue(Path(result['metadata_path']).is_file())

    def test_arm64_simulator_is_rejected_despite_iphone_plist(self):
        self.binary.write_bytes(macho(platform=7))
        with self.assertRaisesRegex(ipa.InvalidIPA, 'Simulator'):
            self.package()
        self.assertEqual(list((self.root / 'output').glob('*.ipa')), [])

    def test_signature_cannot_be_relabelled_unsigned(self):
        self.binary.write_bytes(macho(signed=True))
        with self.assertRaisesRegex(ipa.InvalidIPA, 'code signature'):
            self.package()

    def test_profiles_and_private_keys_are_rejected(self):
        for name in ['embedded.mobileprovision', 'certificate.p12']:
            with self.subTest(name=name):
                path = self.app / name
                path.write_bytes(b'not-for-distribution')
                with self.assertRaises(ipa.InvalidIPA):
                    self.package()
                path.unlink()

    def test_symlink_is_rejected(self):
        (self.app / 'linked').symlink_to(self.binary)
        with self.assertRaisesRegex(ipa.InvalidIPA, 'Symlink'):
            self.package()

    def test_archive_payload_directory_cannot_disguise_a_symlink(self):
        result = self.package()
        tampered = self.root / 'symlink.ipa'
        with zipfile.ZipFile(result['ipa_path']) as source, zipfile.ZipFile(tampered, 'w') as target:
            for entry in source.infolist():
                if entry.filename == 'Payload/':
                    entry.external_attr = (stat.S_IFLNK | 0o755) << 16
                target.writestr(entry, source.read(entry))
        with self.assertRaisesRegex(ipa.InvalidIPA, 'regular files/directories'):
            ipa.verify(tampered)

    def test_missing_executable_permission_is_rejected(self):
        self.binary.chmod(0o644)
        with self.assertRaisesRegex(ipa.InvalidIPA, 'executable permission'):
            self.package()


if __name__ == '__main__':
    unittest.main()

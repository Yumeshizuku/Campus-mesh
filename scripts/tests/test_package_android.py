import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile


def load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).resolve().parents[1] / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


package = load('package-android').package
verify = load('verify-android-delivery').verify


class AndroidDeliveryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.build = self.root / 'android/app/build/outputs/apk/debug'
        self.build.mkdir(parents=True)
        (self.root / '.gitignore').write_text('/android/app/build/\n/delivery/\n')
        (self.root / 'android/app/build.gradle.kts').write_text('versionName = "alpha-0.3.0"\nversionCode = 4\nminSdk = 26\ntargetSdk = 35\n')
        self.git('init', '-q')
        self.git('add', '.')
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'fixture')
        self.sha = self.git('rev-parse', 'HEAD')
        self.metadata = {'variantName': 'debug', 'applicationId': 'org.campusmesh', 'elements': [
            {'versionName': 'alpha-0.3.0', 'versionCode': 4, 'outputFile': 'app-debug.apk'}]}
        self.write_metadata()
        with zipfile.ZipFile(self.build / 'app-debug.apk', 'w') as archive:
            archive.writestr('lib/arm64-v8a/libcampus_mesh.so', b'test fixture only')
        self.env = {'GITHUB_SHA': self.sha, 'GITHUB_REF': 'refs/tags/android-alpha-0.3.0',
                    'GITHUB_REF_TYPE': 'tag', 'GITHUB_REF_NAME': 'android-alpha-0.3.0',
                    'GITHUB_REPOSITORY': 'owner/repo', 'GITHUB_RUN_ID': '123',
                    'GITHUB_RUN_ATTEMPT': '1', 'RELEASE_TAG': 'android-alpha-0.3.0'}
        self.out = self.root / 'delivery'

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True).strip()

    def write_metadata(self):
        (self.build / 'output-metadata.json').write_text(json.dumps(self.metadata))

    def test_package_round_trip_and_tamper(self):
        info = package(self.root, self.out, self.env)
        self.assertIn(self.sha, info['apk'])
        self.assertEqual(verify(self.out, self.env), info)
        (self.out / info['apk']).write_bytes(b'tampered')
        with self.assertRaisesRegex(ValueError, 'digest mismatch'):
            verify(self.out, self.env)

    def test_stale_version_rejected(self):
        self.metadata['elements'][0]['versionCode'] = 3
        self.write_metadata()
        with self.assertRaisesRegex(ValueError, 'metadata differs'):
            package(self.root, self.out, self.env)

    def test_checkout_mismatch_rejected(self):
        self.env['GITHUB_SHA'] = '0' * 40
        with self.assertRaisesRegex(ValueError, 'Checkout SHA'):
            package(self.root, self.out, self.env)

    def test_wrong_tag_rejected(self):
        self.env['GITHUB_REF_NAME'] = 'android-alpha-0.4.0'
        with self.assertRaisesRegex(ValueError, 'tag must match'):
            package(self.root, self.out, self.env)

    def test_wrong_abi_rejected(self):
        with zipfile.ZipFile(self.build / 'app-debug.apk', 'w') as archive:
            archive.writestr('lib/x86_64/libcampus_mesh.so', b'wrong abi')
        with self.assertRaisesRegex(ValueError, 'ARM64-only'):
            package(self.root, self.out, self.env)

    def test_missing_apk_cannot_fall_back_to_history(self):
        (self.build / 'app-debug.apk').unlink()
        (self.root / 'artifacts').mkdir()
        (self.root / 'artifacts/old.apk').write_bytes(b'historical')
        with self.assertRaises(FileNotFoundError):
            package(self.root, self.out, self.env)

    def test_modified_source_rejected(self):
        with (self.root / 'android/app/build.gradle.kts').open('a') as stream:
            stream.write('// modification')
        with self.assertRaisesRegex(ValueError, 'modified'):
            package(self.root, self.out, self.env)

    def test_download_run_identity_and_extra_files_rejected(self):
        package(self.root, self.out, self.env)
        with self.assertRaisesRegex(ValueError, 'runId'):
            verify(self.out, dict(self.env, GITHUB_RUN_ID='124'))
        (self.out / 'old.apk').write_bytes(b'old')
        with self.assertRaisesRegex(ValueError, 'Unexpected'):
            verify(self.out, self.env)

    def test_existing_delivery_cannot_be_overwritten(self):
        package(self.root, self.out, self.env)
        with self.assertRaises(FileExistsError):
            package(self.root, self.out, self.env)


if __name__ == '__main__':
    unittest.main()

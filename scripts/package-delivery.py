#!/usr/bin/env python3
"""Package source, optionally with the current Android APK; exclude private state."""
from pathlib import Path
import argparse
import hashlib
import re
import zipfile
ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--ios-source', action='store_true', help='Package the iOS release source without an Android APK')
args = parser.parse_args()
if args.ios_source:
    version = re.search(r'MARKETING_VERSION:\s*([^\s]+)', (ROOT / 'ios/project.yml').read_text()).group(1)
    channel = re.search(r'CMReleaseChannel:\s*([^\s]+)', (ROOT / 'ios/project.yml').read_text()).group(1)
    version = f'{channel}-{version}'
    apk = None
    archive = ROOT.parent / f'Campus-Mesh-iOS-{version}-source.zip'
else:
    version = re.search(r'versionName\s*=\s*"([^"]+)"', (ROOT / 'android/app/build.gradle.kts').read_text()).group(1)
    apk = f'CampusMesh-{version}-debug-arm64.apk'
    assert (ROOT / 'artifacts' / apk).is_file(), 'Build and copy the current ARM64 APK first'
    archive = ROOT.parent / f'Campus-Mesh-{version}-delivery.zip'
if not re.fullmatch(r'alpha-\d+\.\d+\.\d+', version):
    parser.error('Test versions must use alpha-<major>.<minor>.<patch>, for example alpha-0.4.0')
excluded = {'target', '.gradle', '.kotlin', 'build', '.cxx', 'jniLibs', 'Libraries', '.git', '__pycache__'}
files = []
for path in sorted(ROOT.rglob('*')):
    if not path.is_file() or path.is_symlink():
        continue
    relative = path.relative_to(ROOT)
    if any(part in excluded for part in relative.parts):
        continue
    if relative.parts[0] == 'artifacts' and path.name != apk:
        continue
    if path.name in {'local.properties', 'SOURCE_MANIFEST.sha256'}:
        continue
    if path.suffix in {'.lock', '.tmp'} and path.name != 'Cargo.lock':
        continue
    files.append(path)
manifest = ROOT / 'SOURCE_MANIFEST.sha256'
manifest.write_text(''.join(hashlib.file_digest(path.open('rb'), 'sha256').hexdigest() + '  ' + path.relative_to(ROOT).as_posix() + '\n' for path in files))
files.append(manifest)
with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as output:
    for path in files:
        output.write(path, Path('campus-mesh') / path.relative_to(ROOT))
with zipfile.ZipFile(archive) as check:
    assert check.testzip() is None
print(f'{archive} ({len(files)} files, {archive.stat().st_size} bytes)')
print('SHA256 ' + hashlib.file_digest(archive.open('rb'), 'sha256').hexdigest())

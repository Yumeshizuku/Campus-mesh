#!/usr/bin/env python3
"""Validate downloaded Android delivery before uploading it to a tag release."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def verify(folder, env):
    info = json.loads((folder / 'build-info.json').read_text())
    sha = env['GITHUB_SHA']
    tag = env['RELEASE_TAG']
    expected = {
        'sourceCommit': sha, 'sourceRef': f'refs/tags/{tag}',
        'repository': env['GITHUB_REPOSITORY'], 'runId': env['GITHUB_RUN_ID'],
        'runAttempt': env['GITHUB_RUN_ATTEMPT'], 'variant': 'debug', 'abi': 'arm64-v8a',
    }
    for key, value in expected.items():
        if info[key] != value:
            raise ValueError(f'Provenance mismatch: {key}')
    if tag != f'android-{info["versionName"]}':
        raise ValueError('Tag/version mismatch')
    if (folder / 'SOURCE_COMMIT.txt').read_text().strip() != sha:
        raise ValueError('Commit file mismatch')
    apk = folder / info['apk']
    if apk.parent.resolve() != folder.resolve() or apk.suffix != '.apk':
        raise ValueError('Invalid APK filename')
    digest = hashlib.sha256(apk.read_bytes()).hexdigest()
    if digest != info['apkSha256']:
        raise ValueError('APK digest mismatch')
    if (folder / f'{apk.name}.sha256').read_text() != f'{digest}  {apk.name}\n':
        raise ValueError('Checksum file mismatch')
    allowed = {apk.name, apk.name + '.sha256', 'build-info.json', 'SOURCE_COMMIT.txt', 'README.txt'}
    if {p.name for p in folder.iterdir()} != allowed:
        raise ValueError('Unexpected or missing delivery files')
    return info


if __name__ == '__main__':
    info = verify(Path(sys.argv[1]), os.environ)
    # checkout resolves lightweight and annotated tags to the actual commit.
    commit = subprocess.check_output(['git', 'rev-parse', f'refs/tags/{os.environ["RELEASE_TAG"]}^{{commit}}'], text=True).strip()
    if commit != info['sourceCommit']:
        raise ValueError('Checked out tag differs from package commit')
    print('Verified APK SHA-256, version, source commit, tag, and workflow run identity')

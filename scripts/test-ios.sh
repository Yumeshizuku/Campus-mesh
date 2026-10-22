#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
./scripts/build-ios.sh --simulator
build_root="$PWD/ios/build"
xcrun simctl list devices available --json > "$build_root/simulators.json"
simulator_id=$(python3 - "$build_root/simulators.json" <<'PY'
import json, re, sys
devices = json.load(open(sys.argv[1]))['devices']
candidates = []
for runtime, entries in devices.items():
    match = re.search(r'iOS-(\d+)-(\d+)', runtime)
    if not match or int(match[1]) < 26:
        continue
    for device in entries:
        if device.get('isAvailable') and device['name'].startswith('iPhone'):
            candidates.append(((int(match[1]), int(match[2])), device['name'], device['udid']))
if not candidates:
    sys.exit('No available iOS 26+ iPhone simulator. Install a runtime in Xcode Settings > Components.')
print(max(candidates)[2])
PY
)
printf 'Testing on iPhone simulator %s\n' "$simulator_id"
# XCTest boots and installs on this simulator. Keep the result bundle for screenshots
# and genuine assertion results; avoid parallel runners sharing the fixed TCP port.
result_bundle="$build_root/PersonalUse-$(date -u +%Y%m%dT%H%M%SZ).xcresult"
xcodebuild -project ios/CampusMesh.xcodeproj -scheme CampusMesh \
  -sdk iphonesimulator -configuration Debug \
  -destination "platform=iOS Simulator,id=$simulator_id" -destination-timeout 180 \
  -derivedDataPath "$build_root/Simulator" -resultBundlePath "$result_bundle" \
  -parallel-testing-enabled NO -maximum-concurrent-test-simulator-destinations 1 \
  CODE_SIGNING_ALLOWED=NO test 2>&1 | tee "$build_root/personal-use-tests.log"

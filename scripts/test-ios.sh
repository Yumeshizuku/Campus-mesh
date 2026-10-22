#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if (( $# == 0 )); then
  ./scripts/build-ios.sh --simulator
elif (( $# != 1 )) || [[ "$1" != --skip-build ]]; then
  echo 'Usage: scripts/test-ios.sh [--skip-build]' >&2
  exit 2
fi
build_root="$PWD/ios/build"
xcrun simctl list devices available --json > "$build_root/simulators.json"
sdk_version=$(xcrun --sdk iphonesimulator --show-sdk-version)
simulator_id=$(python3 - "$build_root/simulators.json" "$sdk_version" <<'PY'
import json, re, sys
devices = json.load(open(sys.argv[1]))['devices']
sdk = tuple(map(int, sys.argv[2].split('.')[:2]))
candidates = []
for runtime, entries in devices.items():
    match = re.search(r'iOS-(\d+)-(\d+)', runtime)
    if not match or int(match[1]) < 26:
        continue
    version = (int(match[1]), int(match[2]))
    if version > sdk:
        continue
    for device in entries:
        if device.get('isAvailable') and device['name'].startswith('iPhone'):
            candidates.append((version, device['name'], device['udid']))
if not candidates:
    sys.exit('No available iOS 26+ iPhone simulator compatible with SDK ' + sys.argv[2] + '. Install a matching runtime in Xcode Settings > Components.')
print(max(candidates)[2])
PY
)
printf 'Testing on iPhone simulator %s\n' "$simulator_id"
python3 - "$simulator_id" "$build_root/simulators.json" <<'PY'
import json, subprocess, sys
device_id = sys.argv[1]
devices = json.load(open(sys.argv[2]))['devices']
device = next(item for items in devices.values() for item in items if item['udid'] == device_id)
if device['state'] != 'Booted':
    subprocess.run(['xcrun', 'simctl', 'boot', device_id], check=True, timeout=60)
subprocess.run(['xcrun', 'simctl', 'bootstatus', device_id, '-b'], check=True, timeout=300)
PY
# Keep the result bundle for screenshots and genuine assertion results; avoid
# parallel runners sharing the fixed TCP port. XCTest also has an execution limit.
result_bundle="$build_root/PersonalUse-$(date -u +%Y%m%dT%H%M%SZ).xcresult"
export_results() {
  result=$?
  trap - EXIT
  if [[ -d "$result_bundle" ]]; then
    xcrun xcresulttool get test-results summary --path "$result_bundle" > "$build_root/personal-use-summary.json" || true
    xcrun xcresulttool export attachments --path "$result_bundle" --output-path "$build_root/screenshots" || true
  fi
  exit "$result"
}
trap export_results EXIT
xcodebuild -project ios/CampusMesh.xcodeproj -scheme CampusMesh \
  -sdk iphonesimulator -configuration Debug \
  -destination "platform=iOS Simulator,id=$simulator_id" -destination-timeout 180 \
  -derivedDataPath "$build_root/Simulator" -resultBundlePath "$result_bundle" \
  -parallel-testing-enabled NO -maximum-concurrent-test-simulator-destinations 1 \
  -test-timeouts-enabled YES -default-test-execution-time-allowance 180 \
  -maximum-test-execution-time-allowance 300 \
  CODE_SIGNING_ALLOWED=NO test 2>&1 | tee "$build_root/personal-use-tests.log"

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then echo 'iOS builds require macOS + Xcode 26 or newer.' >&2; exit 1; fi
for tool in xcodebuild xcrun xcodegen rustup cargo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing required tool: $tool. Install Xcode 26+, XcodeGen and Rust before building." >&2
    exit 1
  fi
done
# Validate before compiling Rust: an old SDK would silently omit the native
# Liquid Glass branches guarded by compiler(>=6.2) in SwiftUI source.
xcode_version=$(xcodebuild -version)
printf '%s\n' "$xcode_version"
xcode_major=$(awk '/^Xcode / { split($2, version, "."); print version[1]; exit }' <<< "$xcode_version")
if [[ ! "$xcode_major" =~ ^[0-9]+$ ]] || (( xcode_major < 26 )); then
  echo 'Select Xcode 26+ using xcode-select or DEVELOPER_DIR; older Xcode cannot validate Liquid Glass.' >&2
  exit 1
fi
for sdk in iphoneos iphonesimulator; do
  sdk_version=$(xcrun --sdk "$sdk" --show-sdk-version)
  sdk_major=${sdk_version%%.*}
  if [[ ! "$sdk_major" =~ ^[0-9]+$ ]] || (( sdk_major < 26 )); then
    echo "The active $sdk SDK is $sdk_version; iOS 26+ SDKs are required (deployment remains iOS 16)." >&2
    exit 1
  fi
  printf '%s SDK: %s\n' "$sdk" "$sdk_version"
done
host_arch=$(uname -m)
if [[ "$host_arch" == arm64 ]]; then
  simulator_target=aarch64-apple-ios-sim
elif [[ "$host_arch" == x86_64 ]]; then
  simulator_target=x86_64-apple-ios
else
  echo "Unsupported Mac architecture: $host_arch" >&2
  exit 1
fi
rustup target add aarch64-apple-ios "$simulator_target"
cargo rustc --release --locked -p campus-mesh --lib --target aarch64-apple-ios --crate-type staticlib
cargo rustc --release --locked -p campus-mesh --lib --target "$simulator_target" --crate-type staticlib
mkdir -p ios/Libraries/iphoneos ios/Libraries/iphonesimulator
cp target/aarch64-apple-ios/release/libcampus_mesh.a ios/Libraries/iphoneos/
cp "target/$simulator_target/release/libcampus_mesh.a" ios/Libraries/iphonesimulator/
cd ios
xcodegen generate
xcodebuild -project CampusMesh.xcodeproj -scheme CampusMesh -sdk iphonesimulator -configuration Debug -destination "generic/platform=iOS Simulator" ARCHS="$host_arch" ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO build

#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to NDK 27.x}"
: "${ANDROID_HOME:?Set ANDROID_HOME to Android SDK}"
rustup target add aarch64-linux-android
command -v cargo-ndk >/dev/null || cargo install cargo-ndk --locked
cargo ndk -t arm64-v8a -p 26 -o android/app/src/main/jniLibs build --release --locked -p campus-mesh --lib
cd android
./gradlew assembleDebug

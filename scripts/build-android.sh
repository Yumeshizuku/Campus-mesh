#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to NDK 27.x}"
: "${ANDROID_HOME:?Set ANDROID_HOME to Android SDK}"
# Keep the runner's legacy NDK alias aligned with the explicitly selected NDK.
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
rustup target add aarch64-linux-android
command -v cargo-ndk >/dev/null || cargo install cargo-ndk --version 4.1.2 --locked
# Use the full platform flag: cargo-ndk 4 reserves lowercase -p for Cargo packages.
# Suppress cargo-ndk's environment dump if its panic hook is triggered.
CARGO_NDK_NO_PANIC_HOOK=1 cargo ndk -t arm64-v8a --platform 26 -o android/app/src/main/jniLibs build --release --locked -p campus-mesh --lib
cd android
# Never package a previous successful APK after a failed/current build.
rm -rf app/build/outputs/apk/debug
./gradlew assembleDebug

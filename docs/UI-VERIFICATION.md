# 0.2 界面升级验证

这份记录保留了 2026-10-22 早期 0.2 版本的检查结果，范围包括 Android 与 iOS 界面、外观和便捷操作。当时没有调整 Rust 通信协议，也没有新增无线验收结果。后续 iOS 已通过完整编译和基础操作检查，最新进展见[验证报告](VERIFICATION.md)。

| 项目 | 本轮结果 | 证据 |
|---|---|---|
| Android ARM64 0.2.0 `assembleDebug` | 通过，versionCode 2 | `reports/ui-android-build-lint.txt` |
| Android `lintDebug` | 通过，0 errors / 8 warnings | `reports/ui-android-lint.txt` |
| x86_64 模拟器版本构建 | 通过 | `reports/ui-android-x86-build.txt` |
| APK 签名 / 16 KB zip alignment | 通过 | `reports/ui-apk-signature.txt`、`reports/ui-apk-alignment.txt` |
| Swift 六个源文件语法解析 | 无解析错误；非类型检查 | `reports/ui-swift-syntax.txt` |
| 实际 UI 操作 / 截图 | 未完成，不能标为通过 | `reports/ui-emulator.json` |
| iOS App 类型检查 / 构建 / UI 运行 | 未执行，需要 macOS/Xcode | — |

Android Lint 警告保留为依赖更新提示、内联文本的本地化提示、较新权限属性与 ARM64 版本缺少 ChromeOS x86 ABI；没有屏蔽 Lint 错误。扫码兼容接口仍有 deprecation 编译提示。

当时搭建了 Android API 35 的 x86_64 软件模拟器，接入实际 Rust 库和原生桥接。由于环境缺少硬件虚拟化支持，系统服务始终没有完成冷启动，应用也未能成功安装，因此这一阶段的界面操作和截图没有通过记录。

已加入 `scripts/verify-ui-emulator.py`，供具备正常启动模拟器的环境执行。它计划检查草稿跨页面/进程恢复、深浅色切换及通过真实 TCP 的界面收发；读取模拟器私有状态时仅导出公钥身份，不输出私钥。脚本仅允许 `emulator-*` 设备，`--reset` 会清空专用模拟器内该 App 的数据，不能用于个人设备。脚本本轮未完成执行，故其断言不算已通过证据。

## 复现

正常 ARM64 手机：按 `scripts/build-android.sh` 构建，或安装 `artifacts/CampusMesh-0.2.0-debug-arm64.apk`。在拥有同一调试签名的旧安装上可升级，保留原身份文件和聊天记录；签名不一致时不要为强行安装而卸载，否则会丢失应用数据。

模拟器另需 x86_64 Rust 库：

```sh
rustup target add x86_64-linux-android
# Linux 示例；其他宿主系统需调整 NDK 的 host-toolchain 目录。
CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android26-clang" \
  cargo build --release --locked -p campus-mesh --lib --target x86_64-linux-android
mkdir -p android/app/src/main/jniLibs/x86_64
cp target/x86_64-linux-android/release/libcampus_mesh.so android/app/src/main/jniLibs/x86_64/
(cd android && ./gradlew assembleDebug -PmeshAbi=x86_64)
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
cargo build --locked --bin mesh-node
python3 scripts/verify-ui-emulator.py --serial emulator-5554 --reset
```

界面完整回归步骤见 `UI-UPGRADE.md`。特别需要真机或正常模拟器确认：最大系统字体、横屏与小屏键盘、主题对比度、TalkBack/VoiceOver、两位好友的消息隔离、群发部分失败重试，以及从旧安装升级后的身份/草稿保留。

恢复默认外观不会删除核心文件；UI 配置分别写入 Android `appearance` SharedPreferences 和 iOS `ui.*` UserDefaults。草稿、短语和群发成员是本机应用沙盒数据，未添加云同步，也未修改任何校园电脑或网络策略。

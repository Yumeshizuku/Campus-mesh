# Campus Mesh

跨平台校园离线通信开发版本：Rust DTN 核心、Android Kotlin 应用、iOS SwiftUI 应用。实际链路使用原生 BLE GATT、Android Wi-Fi Direct 和普通局域网 TCP，不接入消息服务器，不要求互联网或手机流量。

**验收状态：开发原型，尚未完成双手机、跨平台及无线多跳真机验收。** Linux 真实 TCP 三跳和断线恢复已验证；Android APK 构建和 iOS 状态以 [验证报告](docs/VERIFICATION.md) 为准。10–300 个虚拟节点测试不代表同等规模的无线网络容量。不得直接用于需要可靠应急通信的场景。

## iOS alpha-0.4.0：加入 Liquid Glass

这次 iOS 更新在 Telegram 风格的聊天界面中加入了 Apple 的 Liquid Glass。输入栏、快捷短语和发送等按钮会在 iOS 26 及以上系统显示原生玻璃效果，iOS 16–18 则使用兼容材质。你可以在设置中预览或关闭通透效果；开启系统的降低透明度或增强对比度后，这些控件会使用实色背景。

模拟器编译和 iPhone 设备版归档已在 GitHub 的 Xcode 26.2 环境通过，未签名 IPA 也已发布。安装需要有效的 Apple 签名，目前还没有 iPhone 实际运行的测试结果。具体变化见 [Liquid Glass 更新说明](docs/LIQUID-GLASS.md)。Android 当前版本仍为 `alpha-0.3.0`。

测试版统一使用 **`alpha-版本号`** 命名，应用和下载文件都会标明对应版本。各次更新汇总在 [更新日志](CHANGELOG.md)，命名规则见 [版本命名约定](docs/RELEASE-NAMING.md)。

## 0.3 Telegram 风格界面

聊天首页改成了会话列表，可以直接看到头像、最近一条消息和本机未读提示。点开会话后，会进入独立聊天页，消息按日期分组，默认浅色主题使用浅绿发送气泡，输入栏也换上了纸飞机发送按钮。原有的外观设置、草稿和离线通信功能都可以继续使用。详见 [0.3 界面更新说明](docs/TELEGRAM-STYLE.md)。

## 0.2 界面更新

这次更新把聊天、附近、身份和自定义设置分成了四个页面。会话支持搜索和自动保存草稿，也可以保存常用好友、群发成员和自己的快捷短语。外观上增加了深浅色、四种主题色、字号和紧凑模式，方便按习惯调整。群发时如果只有部分成员提交失败，再次发送会只重试未完成的成员。

具体操作见 [界面升级说明](docs/UI-UPGRADE.md)，当时的构建和检查结果见 [界面验证记录](docs/UI-VERIFICATION.md)。无线通信的测试进展记录在 [验证报告](docs/VERIFICATION.md) 中。

## 项目结构

- `core/`：签名/加密、去重、TTL、回执、持久化、Gossip/DTN、真实 TCP、C ABI、CLI 和测试。
- `android/`：Kotlin UI、JNI、BLE 中心/外设双角色、Wi-Fi P2P、前台服务、二维码、通知。
- `ios/`：SwiftUI、CoreBluetooth 双角色、二维码、C ABI 桥接、XcodeGen 配置。
- `scripts/`：平台构建、四进程真实通信验证。
- `windows/`：可选授权中继入口；手机通信不依赖 Windows。
- [架构与协议](docs/ARCHITECTURE.md)、[运行说明](docs/RUNNING.md)、[限制与未完成项](docs/LIMITATIONS.md)、[真机验收步骤](docs/DEVICE-TESTS.md)。

## 先运行可复现的核心验证

安装 Rust 1.99.1、Python 3，然后：

```sh
cargo test --workspace --locked -- --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
python3 scripts/verify-processes.py
cargo run --release --locked --bin mesh-sim
```

四个独立的 CLI 进程只通过真实 loopback TCP 建立 A–B–C–D 链路。脚本先断开 C，确认 B 已暂存密文，重启 B、C，再验证 D 只收到一次且 A 收到端到端回执。测试不调用模拟传输。

## 构建手机应用

Android：JDK 17 或 21、Android SDK 35、Build Tools 35.0.0、NDK 27.2.12479018、CMake 3.22.1。设置 `ANDROID_HOME` 和 `ANDROID_NDK_HOME`，运行：

```sh
./scripts/build-android.sh
```

产物：`android/app/build/outputs/apk/debug/app-debug.apk`，当前默认 ARM64、Android 8.0+。完整 Kotlin、JNI 和 Rust 均参与构建。首次下载工具和依赖需要互联网；应用正常收发不需要。

iOS：需要 macOS、Xcode 26 或更新版本（含 iOS 26+ SDK）、XcodeGen、Rust。运行：

```sh
./scripts/build-ios.sh
```

随后打开 `ios/CampusMesh.xcodeproj`，选择自己的开发团队和设备。iOS 最低 16，构建脚本按 Mac 主机架构选择 Apple Silicon 或 Intel 模拟器库。普通版本不要求 multicast 特殊 entitlement；iOS 局域网使用手动地址连接，BLE 提供附近设备发现。

生成 iPhone 真机未签名 IPA：运行 `./scripts/build-ios.sh --device`，或在 GitHub Actions 运行 **Build iOS IPA**。云端自动准备 Xcode、Rust 和 XcodeGen，并输出 `CampusMesh-alpha-0.4.0-unsigned.ipa`、SHA-256 校验和及元数据。当前版本已成功构建、发布并下载复核：[下载未签名 IPA](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa)。安装前需要有效的 Apple 签名与描述文件。详见 [IPA 构建与下载](docs/IOS-IPA.md)。

## 无互联网双手机起步

1. 安装应用，允许蓝牙/附近设备及需要的相机、通知权限。两端在前台，互扫二维码并当面核对公钥。
2. 两端启用 BLE，等待 GATT 链路出现。关闭 Wi-Fi 和手机流量，发送短文本，确认对方收到、发送方变为 `delivered`。此步骤需要真机执行，尚未在当前环境验证。
3. 加速方式：连入自己的热点或允许设备互通的合法 Wi-Fi；在“附近”填写对方 `IPv4:37078`。Android 还可发起 Wi-Fi Direct，接受系统配对后客户端自动连接群主 TCP 地址。
4. Android 允许前台服务继续运行；iOS 必须保留前台，进入后台后暂停，回到前台继续同步。

群聊目前为逐成员加密群发会话：填写相同群标识并选择成员，每个成员必须预先交换公钥；不具备管理员、成员变更协议或自动群名册分发。详见限制文档。

学校白板电脑不能安装软件，因此本项目不在其上安装、运行或尝试绕过任何限制。校园网络仅在已授权并允许设备互通时用于加速。Windows 中继只供自有或明确授权设备使用。

# Campus Mesh

跨平台校园离线通信开发版本：Rust DTN 核心、Android Kotlin 应用、iOS SwiftUI 应用。实际链路使用原生 BLE GATT、Android Wi-Fi Direct 和普通局域网 TCP，不接入消息服务器，不要求互联网或手机流量。

**验收状态：开发原型，尚未完成双手机、跨平台及无线多跳真机验收。** Linux 真实 TCP 三跳和断线恢复已验证；Android APK 构建和 iOS 状态以 [验证报告](docs/VERIFICATION.md) 为准。10–300 个虚拟节点测试不代表同等规模的无线网络容量。不得直接用于需要可靠应急通信的场景。

## iOS 0.4 Liquid Glass

保留 Telegram 风格会话与聊天气泡，输入栏、快捷短语、发送及最新消息按钮适配 Apple 原生 Liquid Glass。新增通透控件开关与实时预览，支持辅助功能实色回退。iOS 26+ 使用原生玻璃，iOS 16–18 使用兼容材质。构建要求 Xcode 26+；GitHub macOS 的 Xcode 26.2 模拟器编译已通过，尚未完成 iPhone 运行验证。详见 [适配与验证说明](docs/LIQUID-GLASS.md)。Android 仍为 0.3.0。

测试版统一命名为 **`alpha-版本号`**：当前 Android 为 `alpha-0.3.0`，iOS 为 `alpha-0.4.0`。应用版本标识与新交付文件名遵循此规则，详见 [版本命名约定](docs/RELEASE-NAMING.md)。

## 0.3 Telegram 风格界面

会话列表、圆形头像、本机未读提示、独立聊天页、日期分隔、浅绿发送气泡与纸飞机输入栏。保留原有外观设置和离线通信功能。详见 [0.3 界面说明与验证边界](docs/TELEGRAM-STYLE.md)。

## 0.2 界面更新

新增四页导航、独立会话与搜索、自动保存草稿、可编辑快捷短语、常用好友/群发，以及深浅色、四种主题色、字号与紧凑模式。群发部分提交失败时只重试未完成成员。

操作与设置见 [界面升级说明](docs/UI-UPGRADE.md)，本轮构建与界面检查见 [界面验证记录](docs/UI-VERIFICATION.md)。通信能力与真机限制仍以原验证报告为准。

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

生成 iPhone 真机未签名 IPA：运行 `./scripts/build-ios.sh --device`，或在 GitHub Actions 运行 **Build iOS IPA**。云端自动准备 Xcode、Rust 和 XcodeGen，并输出 `CampusMesh-alpha-0.4.0-unsigned.ipa`、SHA-256 校验和及元数据。安装前需要有效的 Apple 签名与描述文件。详见 [IPA 构建与下载](docs/IOS-IPA.md)。

## 无互联网双手机起步

1. 安装应用，允许蓝牙/附近设备及需要的相机、通知权限。两端在前台，互扫二维码并当面核对公钥。
2. 两端启用 BLE，等待 GATT 链路出现。关闭 Wi-Fi 和手机流量，发送短文本，确认对方收到、发送方变为 `delivered`。此步骤需要真机执行，尚未在当前环境验证。
3. 加速方式：连入自己的热点或允许设备互通的合法 Wi-Fi；在“附近”填写对方 `IPv4:37078`。Android 还可发起 Wi-Fi Direct，接受系统配对后客户端自动连接群主 TCP 地址。
4. Android 允许前台服务继续运行；iOS 必须保留前台，进入后台后暂停，回到前台继续同步。

群聊目前为逐成员加密群发会话：填写相同群标识并选择成员，每个成员必须预先交换公钥；不具备管理员、成员变更协议或自动群名册分发。详见限制文档。

学校白板电脑不能安装软件，因此本项目不在其上安装、运行或尝试绕过任何限制。校园网络仅在已授权并允许设备互通时用于加速。Windows 中继只供自有或明确授权设备使用。

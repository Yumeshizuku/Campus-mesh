# Campus Mesh

Campus Mesh 是一款正在开发的校园离线通信应用，提供 Android 和 iOS 版本。手机通过蓝牙、Android Wi-Fi Direct 或同一局域网传递消息，正常收发不需要互联网或手机流量。

目前已完成 Android APK 与 iPhone IPA 构建、核心通信测试，以及 iOS 模拟器上的基础操作检查。双手机收发、跨平台无线连接和无线多跳仍待真机验证，当前版本适合开发体验。10–300 个虚拟节点的结果只适用于仿真，当前版本还不能作为可靠的应急通信工具使用。各项检查的范围与结果见[验证报告](docs/VERIFICATION.md)。

## iOS alpha-0.4.0：加入 Liquid Glass

这次 iOS 更新为 Telegram 风格的聊天界面加入了 Apple Liquid Glass。输入栏、快捷短语和发送等按钮，会在 iOS 26 及更新系统上呈现原生玻璃效果，较早的系统使用兼容材质。你可以在设置中预览或关闭通透效果；开启系统的降低透明度或增强对比度后，这些控件会改用实色背景。

IPA 已通过 GitHub 的 Xcode 26.2 环境构建并发布。个人使用可以在 Windows 或 Mac 上通过 Sideloadly，用自己的免费 Apple 账户签名安装，无需上架。下载文件尚未签名，安装步骤见 [iPhone 个人安装说明](docs/IOS-IPA.md)，外观变化见 [Liquid Glass 更新说明](docs/LIQUID-GLASS.md)。

iOS 26.2 模拟器上的页面切换、公钥二维码、昵称修改、通信暂停与恢复，以及昵称和自定义设置的重启保存均已通过检查。运行记录与实际截图已保存在[验证报告](docs/VERIFICATION.md)中，手机安装和无线收发仍需真机确认。

测试版统一使用 **`alpha-版本号`** 命名，应用和下载文件都会标明对应版本。各次更新汇总在 [更新日志](CHANGELOG.md)，命名规则见 [版本命名约定](docs/RELEASE-NAMING.md)。

## Android alpha-0.3.0：Telegram 风格的聊天布局

聊天首页改成了会话列表，可以查看头像、最近消息、草稿和本机未读提示，也能按“全部”“私聊”和“群发”筛选。进入会话后，消息按日期分组，默认浅色主题使用浅绿发送气泡，输入栏换上了纸飞机发送按钮。原有的外观设置、草稿和常用群发继续保留，详见[界面更新说明](docs/TELEGRAM-STYLE.md)。

## 0.2 界面更新

这次更新把聊天、附近、身份和自定义设置分成了四个页面。会话支持搜索和自动保存草稿，也可以保存常用好友、群发成员和自己的快捷短语。外观上增加了深浅色、四种主题色、字号和紧凑模式，方便按习惯调整。群发时如果只有部分成员提交失败，保持原文再次发送会只重试未完成的成员。

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

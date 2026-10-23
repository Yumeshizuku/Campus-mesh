# Campus Mesh

Campus Mesh 是一款正在开发的校园离线通信应用，提供 Android 和 iOS 版本。手机通过蓝牙、Android Wi-Fi Direct 或同一局域网传递消息，正常收发不需要互联网或手机流量。

目前已完成 Android APK 与 iPhone IPA 构建、核心通信测试，以及 iOS 模拟器上的基础操作检查。双手机收发、跨平台无线连接和无线多跳仍待真机验证，当前版本适合开发体验。10–300 个虚拟节点的结果只适用于仿真，当前版本还不能作为可靠的应急通信工具使用。各项检查的范围与结果见[验证报告](docs/VERIFICATION.md)。

## alpha-0.5.0：扫码好友与 Mesh 群聊

Android 和 iOS 统一升级到 alpha-0.5.0。用户可以互扫带签名的二维码添加好友，设备 ID、好友公钥和本机备注都会保存；修改昵称不会改变设备 ID。附近页面增加了跳数、有效期、同步间隔、批量、邻居和待传缓存设置，保存后会实际用于 Mesh 通信。

原来的“常用群发”已替换为有固定群 ID 和共享成员名单的群聊。群主可以管理最多 32 位成员、生成入群二维码和解散群聊，成员可以退群。群消息继续经 Mesh 多跳传递，分别加密给各成员，并合并显示送达进度。旧成员组合、相关记录和草稿会迁移，原有 Telegram 风格布局与 iOS Liquid Glass 控件继续保留。

使用方法、参数范围、升级行为和离线边界见[好友与群聊说明](docs/FRIENDS-GROUPS.md)。本轮核心、Android、iOS 编译和 3 项模拟器操作检查均已通过，结果和截图见[验证报告](docs/VERIFICATION.md)。[下载 APK](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911054046/artifacts/12482538184) · [下载 IPA](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911048392/artifacts/12482503417)（解压后个人签名安装）。测试版统一使用 **`alpha-版本号`** 命名，历史变化见[更新日志](CHANGELOG.md)。

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

生成 iPhone 真机未签名 IPA：运行 `./scripts/build-ios.sh --device`，或在 GitHub Actions 运行 **Build iOS IPA**。云端自动准备 Xcode、Rust 和 XcodeGen，并输出 `CampusMesh-alpha-0.5.0-unsigned.ipa`、SHA-256 校验和及元数据。此前发布的 alpha-0.4.0 安装包仍可在这里下载：[历史未签名 IPA](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa)。安装前需要有效的 Apple 签名与描述文件。详见 [IPA 构建与下载](docs/IOS-IPA.md)。

## 无互联网双手机起步

1. 安装应用，允许蓝牙/附近设备及需要的相机、通知权限。两端在前台，互扫二维码并当面核对公钥。
2. 两端启用 BLE，等待 GATT 链路出现。关闭 Wi-Fi 和手机流量，发送短文本，确认对方收到、发送方变为 `delivered`。此步骤需要真机执行，尚未在当前环境验证。
3. 加速方式：连入自己的热点或允许设备互通的合法 Wi-Fi；在“附近”填写对方 `IPv4:37078`。Android 还可发起 Wi-Fi Direct，接受系统配对后客户端自动连接群主 TCP 地址。
4. Android 允许前台服务继续运行；iOS 必须保留前台，进入后台后暂停，回到前台继续同步。

群聊可由群主选择好友或分享入群二维码创建。群成员之间不必逐一互加好友；群信息和消息都经 Mesh 同步。成员变更需要送达后才能生效，详见[群聊说明](docs/FRIENDS-GROUPS.md)。

学校白板电脑不能安装软件，因此本项目不在其上安装、运行或尝试绕过任何限制。校园网络仅在已授权并允许设备互通时用于加速。Windows 中继只供自有或明确授权设备使用。

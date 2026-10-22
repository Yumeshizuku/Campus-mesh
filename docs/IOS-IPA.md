# iPhone IPA 构建与下载

`Build iOS IPA` 工作流在 GitHub 的 macOS 环境使用 Xcode 26.2、Rust 1.99.1 和 XcodeGen，从 Rust 核心与 SwiftUI 源码编译 iPhone ARM64 应用。无需在 Linux 或用户电脑安装 Xcode 插件。

## 云端生成

打开仓库的 **Actions → Build iOS IPA → Run workflow**，选择 `main`。勾选 `publish_release` 会把成功构建的文件上传到版本对应的现有 alpha 预发布页面；不勾选则仅保存 Actions 产物。影响 iOS 构建的代码推送到 `main` 也会自动生成产物。

成功后，从运行页面的 Artifacts 下载 `CampusMesh-iOS-alpha-0.4.0-unsigned`，解压得到：

- `CampusMesh-alpha-0.4.0-unsigned.ipa`
- `CampusMesh-alpha-0.4.0-unsigned.ipa.sha256`
- `CampusMesh-alpha-0.4.0-unsigned.ipa.json`

Actions 产物保留 30 天；发布到 GitHub Release 的附件不受此保留期限影响。工作流只在用户手动选择发布时使用仓库写权限。

## 本地构建与检查

macOS 上安装 Xcode 26+、XcodeGen、Rust 和 Python 3 后运行：

```bash
./scripts/build-ios.sh --device
```

脚本编译 `aarch64-apple-ios` Rust 静态库，使用 `iphoneos` SDK 归档完整 SwiftUI 应用，再输出 IPA 到 `artifacts/`。默认的 `./scripts/build-ios.sh` 仍用于模拟器构建。

打包器只使用 Python 标准库，可以在 macOS 或 Linux 独立检查已生成的文件：

```bash
python3 scripts/package-ios-ipa.py verify artifacts/CampusMesh-alpha-0.4.0-unsigned.ipa
```

它检查 `Payload/*.app` 结构、Info.plist、数字内部版本、alpha 发布名称、执行权限及 Mach-O 的 ARM64 和 IOS 设备平台标记；拒绝模拟器程序和带签名材料的输入。JSON 元数据记录 SHA-256、大小、平台、版本及签名状态。

## 安装条件

本流程输出真实设备构建的**未签名 IPA**，不能直接安装到普通 iPhone。安装还需要由有效 Apple 开发者身份签名，并使用与应用标识及目标设备匹配的描述文件；App Store / TestFlight 需要对应分发签名和上传流程。打包器不会生成或假冒这些 Apple 凭证，也不把模拟器编译通过等同于真机运行验证。

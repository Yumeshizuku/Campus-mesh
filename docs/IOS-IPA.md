# iPhone IPA 构建与下载

`Build iOS IPA` 工作流在 GitHub 的 macOS 环境使用 Xcode 26.2、Rust 1.99.1 和 XcodeGen，从 Rust 核心与 SwiftUI 源码编译 iPhone ARM64 应用。无需在 Linux 或用户电脑安装 Xcode 插件。

## 已生成的 alpha-0.4.0

2026-10-22，[运行 39760676278](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39760676278) 的 `build` 与 `publish` 均成功。构建源码提交为 `48d71dbbd57454c4a0737f5d13413d675fe101c3`；Xcode 26.2 / iPhoneOS SDK 26.2 完成 Release 设备归档。

[下载未签名 IPA](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa) · [SHA-256](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.sha256) · [版本与平台元数据](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.json)

- 版本：`alpha-0.4.0` / build `5`；最低 iOS `16.0`。
- 大小：`651779` 字节；应用标识：`org.campusmesh.app`。
- SHA-256：`bdc17ee2d78032fbb753b619f0bf271f4f4fd8f63c1b80a7a3d453ba3e710f45`。
- 已从 Release 重新下载，检查 Payload、ARM64 / IOS Mach-O 平台、执行权限与校验和，结果通过。
- 打包器的 7 项回归测试通过，包括模拟器、签名输入、符号链接与缺失执行权限的拒绝检查。

这是未签名设备构建，尚未进行 iPhone 安装及运行验收。

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

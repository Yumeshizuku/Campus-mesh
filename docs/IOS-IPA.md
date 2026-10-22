# iPhone IPA 构建与下载

现在可以直接通过 GitHub 上的 `Build iOS IPA` 工作流生成 iPhone 安装包。它会在 macOS 环境中准备 Xcode 26.2、Rust 1.99.1 和 XcodeGen，再将 Rust 通信核心与 SwiftUI 界面一起编译成 iPhone ARM64 应用。使用这条云端流程时，你的电脑上无需安装 Xcode。

## 已生成的 alpha-0.4.0

`alpha-0.4.0` 的 IPA 已在 2026-10-22 成功生成并发布。你可以在 [运行 39760676278](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39760676278) 中查看构建和上传过程，两项任务均已通过。此次使用的源码提交是 `48d71dbbd57454c4a0737f5d13413d675fe101c3`，设备版归档由 Xcode 26.2 和 iPhoneOS SDK 26.2 完成，构建配置为 Release。

[下载未签名 IPA](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa) · [SHA-256](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.sha256) · [版本与平台元数据](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.json)

- 版本：`alpha-0.4.0` / build `5`；最低 iOS `16.0`。
- 大小：`651779` 字节；应用标识：`org.campusmesh.app`。
- SHA-256：`bdc17ee2d78032fbb753b619f0bf271f4f4fd8f63c1b80a7a3d453ba3e710f45`。

发布后，已重新下载 IPA，确认包内的 Payload 目录、ARM64 / IOS Mach-O 平台标记、执行权限和校验和都符合要求。打包器的 7 项测试也已通过，能拒绝模拟器程序、带签名的输入、符号链接和缺少执行权限的程序。

**这个 IPA 尚未签名，需要完成签名后才能安装到普通 iPhone；目前还没有手机安装和运行的测试结果。**

## 云端生成

打开仓库的 **Actions → Build iOS IPA → Run workflow**，选择 `main` 即可开始构建。如果希望同时发布下载文件，可以勾选 `publish_release`，构建成功后文件会上传到对应版本已有的 alpha 发布页。没有勾选时，文件仍可从 Actions 运行页面下载。影响 iOS 构建的代码推送到 `main` 后，也会自动运行这条流程。

成功后，从运行页面的 Artifacts 下载 `CampusMesh-iOS-alpha-0.4.0-unsigned`，解压得到：

- `CampusMesh-alpha-0.4.0-unsigned.ipa`
- `CampusMesh-alpha-0.4.0-unsigned.ipa.sha256`
- `CampusMesh-alpha-0.4.0-unsigned.ipa.json`

Actions 中的下载文件会保留 30 天，GitHub Release 上的附件则不受这个期限影响。只有手动勾选发布时，工作流才会使用仓库写权限上传附件。

## 本地构建与检查

macOS 上安装 Xcode 26+、XcodeGen、Rust 和 Python 3 后运行：

```bash
./scripts/build-ios.sh --device
```

脚本会先编译 `aarch64-apple-ios` 的 Rust 静态库，再用 `iphoneos` SDK 归档完整的 SwiftUI 应用，最后把 IPA 保存到 `artifacts/`。如果需要编译模拟器版本，运行不带参数的 `./scripts/build-ios.sh` 即可。

打包器只使用 Python 标准库，可以在 macOS 或 Linux 独立检查已生成的文件：

```bash
python3 scripts/package-ios-ipa.py verify artifacts/CampusMesh-alpha-0.4.0-unsigned.ipa
```

检查会确认 `Payload/*.app` 目录和 Info.plist 是否完整、内部版本号是否为数字、发布名称是否使用 alpha 格式，以及程序是否保留执行权限。它还会读取 Mach-O 中的 ARM64 和 IOS 平台标记，避免把模拟器程序当作设备版。带有签名材料的输入会被拒绝。随包提供的 JSON 文件记录了 SHA-256、文件大小、平台、版本和签名状态，方便核对。

## 安装条件

这里生成的是面向真实设备的**未签名 IPA**。安装前，需要用有效的 Apple 开发者身份签名，并提供与应用标识和目标设备匹配的描述文件。如果要通过 App Store 或 TestFlight 分发，还需要完成相应的分发签名和上传流程。打包工具无法代替 Apple 提供这些凭证；编译完成后，应用在手机上的实际表现也仍需测试。

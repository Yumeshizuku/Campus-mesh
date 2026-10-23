# iPhone 个人安装与 IPA 构建

如果你只想把 Campus Mesh 装到自己的 iPhone 上使用，可以下载下面的 IPA，在 Windows 或 Mac 上通过 [Sideloadly 官网](https://sideloadly.io/)提供的工具，用自己的 Apple 账户签名并安装。这个过程不需要上架 App Store，也不要求购买 Apple Developer Program 会员。**下载的 IPA 尚未签名，不能在 iPhone 的“文件”中点一下就安装；个人签名由安装工具在你的电脑上完成。**

## 安装到自己的 iPhone

1. 在电脑上[下载 alpha-0.5.0 安装包](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911048392/artifacts/12482503417)，解压取得 `CampusMesh-alpha-0.5.0-unsigned.ipa`，并从 [Sideloadly 官网](https://sideloadly.io/)安装对应 Windows 或 macOS 的版本。Windows 按官网下载提示安装 Apple 网页版 iTunes 和 iCloud；如果已有对应的 Microsoft Store 版本，按官方说明更换。
2. 用数据线连接 iPhone，解锁手机并选择信任这台电脑。在 Sideloadly 中选中手机，将下载的 IPA 拖入窗口，填写你自己的 Apple 账户，然后点击 Start。登录和双重认证只在你本机的安装工具中完成，不要把密码、验证码或签名私钥发到聊天或 GitHub。
3. 安装后按系统提示，在“设置 → 通用 → VPN 与设备管理”中信任对应的开发者。在 iOS 16 及以上版本，还需按系统提示开启“设置 → 隐私与安全性 → 开发者模式”，重启并确认。
4. 打开桌面上的 **Campus Mesh alpha**。聊天页出现“本机已就绪”，身份页显示你的昵称和公钥二维码，说明通信核心已启动。可以先调整外观、修改昵称，再开始添加好友。

使用普通 Apple ID 安装模式即可。应用不需要 tweak、Cydia Substrate 或 Substitute 注入，保持这些选项关闭。Sideloadly 的当前版本也不需要安装旧教程中的 Mail 插件。以上路线已按官方资料和 IPA 结构核对，尚未用你的账户和手机完成实际重签安装。

免费账户的个人签名通常有效 7 天，需要在到期前使用同一账户、同一应用标识续签。Sideloadly 提供自动刷新功能，仍需要满足它的电脑、连接和运行条件。到期后先续签，避免通过删除应用来处理；删除应用会删除本机身份和聊天数据。免费账户可同时使用的侧载应用数量也有限，具体以工具和 Apple 的当前规则为准。

如果手边只有 iPhone，而且没有可用的签名证书、描述文件或其他已配置的合法安装途径，目前不能仅凭这个下载文件完成安装。有与设备匹配的签名资料时，可以另行进行个人设备签名；不要把这些资料提交到公开仓库。

### 装好后怎样聊天

两台手机都打开应用，在“身份”页展示公钥二维码，用“附近 → 扫描好友或群邀请二维码”互相添加，并当面核对公钥。保持应用在前台，可以在“附近”启用蓝牙，也可以连到允许设备互通的同一 Wi-Fi 或个人热点，填写对方的 `IPv4:37078` 地址。聊天页选择好友后发送一条短消息，检查接收方的内容和发送方的送达状态。

iPhone 进入后台后会暂停通信，回到前台再继续。个人签名解决的是安装许可；蓝牙、手机局域网和跨平台无线通信仍需按[真机验收步骤](DEVICE-TESTS.md)在实际设备上确认。

安装依据：[Apple 免费与付费会员能力说明](https://developer.apple.com/support/compare-memberships/)、[Apple 开发者模式说明](https://developer.apple.com/documentation/xcode/enabling-developer-mode-on-a-device)、[Sideloadly 官方说明](https://sideloadly.io/)。

## 已有的构建工具

GitHub 上的 `Build iOS IPA` 工作流会自动准备 Xcode 26.2、Rust 1.99.1 和 XcodeGen，将通信核心与界面编译成 iPhone ARM64 真机应用。使用云端构建时，你的电脑上无需安装 Xcode。

构建环境固定为 `macos-15-intel`，产物下载校验和发布任务使用 `ubuntu-24.04`。这些固定标签可以避免默认系统版本迁移带来的变化；使用 Intel 构建机时，生成的 iPhone 应用仍然是 ARM64 版本。工作流所用的 JavaScript Actions 已更新为支持 Node.js 24 的版本。

这条流程已在[alpha-0.5.0 IPA 检查](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911048392)中通过：Mac 完成构建和上传，Ubuntu 重新下载文件，并检查包结构与校验和。这次没有开启发布，下面的 Release 下载文件和校验和仍对应原始发布记录。较早的流程升级结果也保存在[历史检查](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39783000531)中。

## alpha-0.5.0 安装包

[下载本轮 IPA 与校验文件](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911048392/artifacts/12482503417)。应用版本为 alpha-0.5.0 / build 6，最低 iOS 16，架构为 iPhone ARM64。包大小为 840608 字节，SHA-256 为 `139f1d6d7dea46ffa5e39cad8eb47b583c06f3f54dae5293d13dd9a4821b8326`。源码提交为 `49bbdcaa951736ea79ab5cff510368fbe7a46a71`，包结构和下载摘要已复核。

设备 ID、好友备注、群聊与 Mesh 参数的保存已通过 iOS 26.2 模拟器操作检查，结果见[验证报告](VERIFICATION.md)。下载包仍未签名，按上面的个人安装步骤处理。Actions 文件保留 30 天，需要登录 GitHub 下载；下方历史 Release 链接继续保留。

## 已生成的 alpha-0.4.0

`alpha-0.4.0` 的 IPA 已在 2026-10-22 成功生成并发布。你可以在 [运行 39760676278](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39760676278) 中查看构建和上传过程，两项任务均已通过。此次使用的源码提交是 `48d71dbbd57454c4a0737f5d13413d675fe101c3`，设备版归档由 Xcode 26.2 和 iPhoneOS SDK 26.2 完成，构建配置为 Release。

[下载未签名 IPA](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa) · [SHA-256](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.sha256) · [版本与平台元数据](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.json)

- 版本：`alpha-0.4.0` / build `5`；最低 iOS `16.0`。
- 大小：`651779` 字节；应用标识：`org.campusmesh.app`。
- SHA-256：`bdc17ee2d78032fbb753b619f0bf271f4f4fd8f63c1b80a7a3d453ba3e710f45`。

发布后的 IPA 已重新下载，确认应用目录、真机架构、执行权限和校验和完整。打包工具的 7 项测试也已通过，可以识别并拒绝模拟器程序、带有签名材料的输入、符号链接，以及缺少执行权限的程序。详细平台信息保存在随包提供的 JSON 文件中。

**这个 IPA 尚未签名，需要完成签名后才能安装到普通 iPhone；目前还没有手机安装和运行的测试结果。**

## 云端生成

打开仓库的 **Actions → Build iOS IPA → Run workflow**，选择 `main` 即可开始构建。如果希望同时发布下载文件，可以勾选 `publish_release`，构建成功后文件会上传到对应版本已有的 alpha 发布页。没有勾选时，文件仍可从 Actions 运行页面下载。影响 iOS 构建的代码推送到 `main` 后，也会自动运行这条流程。

每次运行都会先在 Mac 上生成并上传 IPA，再由独立的 Ubuntu 任务重新下载，检查 IPA 结构和 SHA-256 校验和。只有构建与下载校验都通过，手动选择的发布任务才会继续。

成功后，从运行页面的 Artifacts 下载 `CampusMesh-iOS-alpha-0.5.0-unsigned`，解压得到：

- `CampusMesh-alpha-0.5.0-unsigned.ipa`
- `CampusMesh-alpha-0.5.0-unsigned.ipa.sha256`
- `CampusMesh-alpha-0.5.0-unsigned.ipa.json`

Actions 中的下载文件会保留 30 天，GitHub Release 上的附件则不受这个期限影响。只有手动勾选发布时，工作流才会使用仓库写权限上传附件。

## 本地构建与检查

macOS 上安装 Xcode 26+、XcodeGen、Rust 和 Python 3 后运行：

```bash
./scripts/build-ios.sh --device
```

脚本会先编译 `aarch64-apple-ios` 的 Rust 静态库，再用 `iphoneos` SDK 归档完整的 SwiftUI 应用，最后把 IPA 保存到 `artifacts/`。如果需要编译模拟器版本，运行不带参数的 `./scripts/build-ios.sh` 即可。

打包器只使用 Python 标准库，可以在 macOS 或 Linux 独立检查已生成的文件：

```bash
python3 scripts/package-ios-ipa.py verify artifacts/CampusMesh-alpha-0.5.0-unsigned.ipa
```

检查会确认 `Payload/*.app` 目录和 Info.plist 是否完整、内部版本号是否为数字、发布名称是否使用 alpha 格式，以及程序是否保留执行权限。它还会读取 Mach-O 中的 ARM64 和 IOS 平台标记，避免把模拟器程序当作设备版。带有签名材料的输入会被拒绝。随包提供的 JSON 文件记录了 SHA-256、文件大小、平台、版本和签名状态，方便核对。

## 启动与操作检查

macOS 上运行 `./scripts/test-ios.sh`，会编译应用并在已安装的 iOS 26 或更新版本的 iPhone 模拟器中真正启动它。检查覆盖通信核心初始化、公钥二维码、四个主页面、昵称和设备 ID 的重启保存、暂停与恢复通信、自定义偏好、好友备注、群聊邀请及 Mesh 参数保存。测试调用实际应用和 Rust 核心，没有用模拟数据替代身份。

`Verify` 工作流会执行相同检查，并保存结果、页面截图和日志。可以下载 `ios-personal-use-report` 查看简明报告，也可以下载 `ios-personal-use-tests` 查看完整测试文件。个人签名、手机安装和无线链路仍需要在真实设备上确认。

2026-10-23，这套检查已在[运行 39877716098](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39877716098)中通过。实际使用的是 iOS 26.2、ARM64 的 iPhone SE 模拟器，完整操作用例 1 项通过、0 项失败。核心、Android 和 Intel iOS 编译任务也全部通过；[同一提交的 IPA 流程](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39877716031)完成了设备编译、上传、下载及校验。

操作测试使用 `macos-15` 的 Apple Silicon 环境，先完成测试程序编译，再启动模拟器。所用的界面和通信核心源码与上方已发布 IPA 一致。详细结果见[结构化报告](reports/ios-personal-use.json)和[XCTest 日志](reports/ios-personal-use-tests.txt)，实际截图包括[聊天](reports/ios-personal-use/01-chat.png)、[附近](reports/ios-personal-use/02-nearby.png)、[身份](reports/ios-personal-use/03-identity.png)和[自定义](reports/ios-personal-use/04-customization.png)。

# iOS 0.4 Liquid Glass 适配

保留 Telegram 式会话列表、头像和消息气泡，将 Apple Liquid Glass 用于悬浮的操作控件。当前 iOS 测试版本名为 alpha-0.4.0 / build 5，最低运行系统保持 iOS 16。Android 测试版本为 alpha-0.3.0。

## 界面与自定义

- iOS 26+ 的聊天输入框、快捷短语及“最新消息”按钮使用原生 `glassEffect(.regular, in:)`。纸飞机发送按钮使用 `.glassProminent`，跟随已保存的主题色。
- 相邻输入控件由一个 `GlassEffectContainer` 管理。移除原来的整条不透明输入栏背景，保留三个独立的圆形/圆角操作区域；文本框支持 1–5 行，键盘与安全区仍由系统布局管理。
- 原生 `NavigationStack`、`TabView`、菜单与系统表单通过 iOS 26 SDK 获得系统外观，不额外覆盖导航栏玻璃。消息气泡、二维码和头像不使用 Liquid Glass。
- “自定义 → Liquid Glass → 通透控件”默认开启，偏好保存在本机；关闭后自定义控件采用实色。该开关不覆盖系统导航栏与标签栏外观。设置中提供与实际材质共用组件的输入栏预览。
- “恢复默认外观”会重置通透控件开关，已有深浅色、主题色、字号、紧凑模式、草稿、短语和常用群发逻辑继续保留。

## 兼容与辅助功能

| 条件 | 自定义控件表现 |
| --- | --- |
| Xcode 26+ 构建，iOS 26+，通透控件开启 | 原生 Liquid Glass，发送按钮使用原生玻璃按钮样式 |
| iOS 16–18，通透控件开启 | 系统 regular Material 兼容背景，发送按钮使用实色强调色 |
| 通透控件关闭 | 不透明背景及分隔描边 |
| 系统“降低透明度”开启 | 自定义控件使用不透明背景 |
| 系统“增强对比度”开启 | 自定义控件使用不透明背景，实色发送按钮使用黑白对比 |
| 系统“减少动态效果”开启 | 关闭自定义玻璃交互效果与程序触发的平滑滚动；原生控件遵循系统设置 |

所有新的玻璃 API 同时使用 `#if compiler(>=6.2)` 编译保护和 `if #available(iOS 26.0, *)` 运行保护。旧编译器排除新 API，旧系统执行兼容分支；正式构建脚本要求 Xcode 26+，避免把兼容分支误当作原生玻璃验证结果。

快捷短语、发送和最新消息按钮提供至少 44 pt 的布局空间与可访问名称；输入框使用系统正文动态字号。预览不接收点击，也不向 VoiceOver 暴露可操作的假按钮。通信核心、私钥文件格式与无线权限没有改动。

## 构建与验证状态

在 Mac 上使用 Xcode 26+、设备与模拟器 SDK 26+、XcodeGen、Rust：

```sh
./scripts/build-ios.sh
```

脚本先检查工具与 SDK，再构建设备及与主机架构匹配的模拟器 Rust 库，生成 Xcode 工程并编译模拟器 App。CI 配置选择 `macos-15` 与 Xcode 26.2；没有推送仓库或触发远程 CI。

本轮当前 Linux 环境仅执行 Swift 语法解析、Bash/Python/YAML 检查、构建脚本的隔离模拟验证和源码压缩包完整性校验。记录见 `reports/liquid-glass-checks.txt`。**没有 Swift 类型检查、Xcode App 编译、iOS 模拟器或 iPhone 实际运行结果，也没有签名 IPA 或运行截图。** 语法解析不能确认 SwiftUI 类型、运行时布局或玻璃渲染效果。

在 Mac/设备上仍需完成：

1. 使用 Xcode 26+ 构建，在 iOS 26 模拟器检查列表、原生导航栏/标签栏、聊天页浮动输入栏与键盘升降；在旧版 iOS 模拟器检查兼容材质。
2. 检查浅色/深色、四种主题色、最窄支持屏幕、最大辅助字号、长消息 1–5 行、VoiceOver、实色开关和三项辅助功能设置。
3. 在两个会话之间切换确认草稿隔离；快捷短语只填入草稿；空白消息与通信暂停时发送按钮不可用；群发失败说明仍可读。
4. 发送后和“最新消息”按钮滚动不遮挡最后一条消息；减少动态效果时没有程序触发的平滑滚动。

导出本轮源码（不混入旧 Android APK）：

```sh
python3 scripts/package-delivery.py --ios-source
```

生成 `Campus-Mesh-iOS-alpha-0.4.0-source.zip`，包含工程源码、文档和校验清单，不包含 Xcode 缓存、私有聊天状态或预编译库。

## 官方 API 依据

- [Applying Liquid Glass to custom views](https://developer.apple.com/documentation/swiftui/applying-liquid-glass-to-custom-views)
- [glassEffect(_:in:)](https://developer.apple.com/documentation/swiftui/view/glasseffect(_:in:))
- [GlassEffectContainer](https://developer.apple.com/documentation/swiftui/glasseffectcontainer)
- [Adopting Liquid Glass](https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass)
- [Human Interface Guidelines — Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [GitHub macOS 15 runner inventory](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-Readme.md)

# iOS 0.4 Liquid Glass 更新

iOS 的 alpha-0.4.0 为聊天输入区和悬浮操作按钮加入了 Apple Liquid Glass 效果，继续使用 Telegram 风格的会话列表、头像和消息气泡。这个版本的内部构建号为 build 5，最低支持 iOS 16；Android 的测试版本仍为 alpha-0.3.0。

## 新的外观和设置

- 在 iOS 26 及更新系统上，聊天输入框、快捷短语按钮和“最新消息”按钮使用原生 `glassEffect(.regular, in:)`。纸飞机发送按钮使用 `.glassProminent`，颜色会跟随已保存的主题设置。
- 输入区由三个独立的圆形或圆角区域组成，原来的整条不透明背景已经移除。相邻控件通过同一个 `GlassEffectContainer` 管理，文本框可显示 1–5 行文字，键盘避让和安全区仍由系统处理。
- 原生 `NavigationStack`、`TabView`、菜单和系统表单会呈现 iOS 26 SDK 对应的系统外观，导航栏没有额外叠加玻璃效果。消息气泡、二维码和头像保持原有材质。
- 在“自定义 → Liquid Glass”中可以找到默认开启的“通透控件”开关。关闭后，自定义控件会改用实色背景，系统导航栏和标签栏仍遵循系统外观。偏好保存在本机，设置页面也提供了与实际输入栏共用组件的材质预览。
- “恢复默认外观”会一并重置通透控件开关。原有的深浅色、主题色、字号、紧凑模式设置，以及草稿、快捷短语和常用群发功能仍然可用。

## 不同系统和辅助功能下的显示效果

| 使用条件 | 自定义控件的显示效果 |
| --- | --- |
| 使用 Xcode 26+ 构建，在 iOS 26+ 开启通透控件 | 使用原生 Liquid Glass，发送按钮也采用原生玻璃样式 |
| 在 iOS 16–18 开启通透控件 | 使用系统 regular Material 背景，发送按钮使用实色主题色 |
| 关闭通透控件 | 使用不透明背景和分隔描边 |
| 开启系统“降低透明度” | 使用不透明背景 |
| 开启系统“增强对比度” | 使用不透明背景，实色发送按钮采用黑白对比 |
| 开启系统“减少动态效果” | 关闭自定义玻璃交互效果和应用触发的平滑滚动；原生控件遵循系统设置 |

新的玻璃 API 同时通过 `#if compiler(>=6.2)` 和 `if #available(iOS 26.0, *)` 判断编译器与系统版本。旧编译器会跳过这些 API，旧系统会使用兼容实现。正式构建脚本要求 Xcode 26+，以确保构建时包含原生 Liquid Glass 的实现。

快捷短语、发送和“最新消息”按钮都提供至少 44 pt 的布局空间，并带有可访问名称。输入框使用系统正文动态字号。设置中的预览只展示效果，不接收点击，也不会被 VoiceOver 读作可操作按钮。通信核心、私钥文件格式和无线权限没有调整。

## 构建和测试进展

在 Mac 上准备好 Xcode 26+、设备与模拟器 SDK 26+、XcodeGen 和 Rust 后，可以运行：

```sh
./scripts/build-ios.sh
```

脚本会先检查工具和 SDK，然后构建设备 Rust 库及与 Mac 架构匹配的模拟器 Rust 库，生成 Xcode 工程，最后编译模拟器 App。CI 目前固定使用 `macos-15-intel` 和 Xcode 26.2，避免默认运行器架构变化影响构建。[早期 GitHub Actions 运行 39758596555](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39758596555) 已完成完整的模拟器 App 编译，其中 `ios` 和 `core` 任务均通过。

最初在 Linux 上完成的是 Swift 语法解析、Bash/Python/YAML 检查、构建脚本的隔离模拟检查，以及源码压缩包的完整性校验。之后的 GitHub macOS 构建补上了 Swift 类型检查和链接验证。**目前还没有完成模拟器交互检查或 iPhone 真机运行测试，也没有实际运行截图。** 界面布局和玻璃渲染效果仍需要运行应用后确认。设备 IPA 的构建结果、签名状态和下载入口见 [IPA 构建与下载](IOS-IPA.md)。

接下来需要在 Mac 或设备上检查这些场景：

1. 使用 Xcode 26+ 构建后，在 iOS 26 模拟器中查看会话列表、原生导航栏和标签栏、聊天页悬浮输入区，以及键盘弹出和收起时的布局；再用旧版 iOS 模拟器检查兼容材质。
2. 分别检查浅色、深色、四种主题色、最窄支持屏幕、最大辅助字号、1–5 行长消息、VoiceOver、实色开关，以及上表中的三项辅助功能设置。
3. 在两个会话之间切换，检查各自的草稿是否保存正确；选择快捷短语时应只填入草稿，空白消息或通信暂停时应无法点击发送，群发失败的说明也应清楚可读。
4. 发送消息或点击“最新消息”后，最后一条消息应完整显示；开启“减少动态效果”时，应用不应触发平滑滚动。

需要导出这个版本的 iOS 源码时，运行：

```sh
python3 scripts/package-delivery.py --ios-source
```

生成的 `Campus-Mesh-iOS-alpha-0.4.0-source.zip` 包含工程源码、文档和校验清单，不包含旧 Android APK、Xcode 缓存、私有聊天状态或预编译库。

## 参考文档

- [Applying Liquid Glass to custom views](https://developer.apple.com/documentation/swiftui/applying-liquid-glass-to-custom-views)
- [glassEffect(_:in:)](https://developer.apple.com/documentation/swiftui/view/glasseffect(_:in:))
- [GlassEffectContainer](https://developer.apple.com/documentation/swiftui/glasseffectcontainer)
- [Adopting Liquid Glass](https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass)
- [Human Interface Guidelines — Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [GitHub macOS 15 runner inventory](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-Readme.md)

这次 iOS 更新在 Telegram 风格的聊天界面中加入了 Apple 的 Liquid Glass。输入栏、快捷短语、发送和“最新消息”按钮在 iOS 26 及以上系统会显示原生玻璃效果，发送按钮也会跟随你选择的主题色。应用仍支持 iOS 16，较早的系统会使用兼容材质。

你可以在“自定义”页面预览效果，也可以关闭通透控件。开启系统的降低透明度或增强对比度后，这些控件会使用实色背景；开启减少动态效果后，会关闭自定义玻璃交互效果和应用主动触发的平滑滚动。已有的会话草稿、快捷短语、主题和字号设置仍然可以使用。

这次也补齐了 IPA 的自动构建、打包校验和发布工具。Xcode 26.2 与 iPhoneOS SDK 26.2 已成功生成 iPhone ARM64 设备版，版本为 **alpha-0.4.0**，内部版本为 0.4.0 / build 5。发布后的 IPA 已重新下载，目录结构、设备平台标记和文件校验和均已检查通过。

**当前 IPA 未签名，安装前需要有效的 Apple 签名和匹配的描述文件。应用尚未完成 iPhone 实际运行测试。**

下载文件：

- [iPhone 未签名 IPA](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.sha256) · [版本与平台信息](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.json)
- [原始应用源码包](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-iOS-alpha-0.4.0-source.zip) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-iOS-alpha-0.4.0-source.zip.sha256)
- [包含 IPA 工具的构建源码](https://github.com/Yumeshizuku/Campus-mesh/tree/48d71dbbd57454c4a0737f5d13413d675fe101c3) · [构建与下载说明](https://github.com/Yumeshizuku/Campus-mesh/blob/main/docs/IOS-IPA.md)

IPA 大小为 651779 字节，SHA-256 为 `bdc17ee2d78032fbb753b619f0bf271f4f4fd8f63c1b80a7a3d453ba3e710f45`。

构建和发布过程可以在[这次运行记录](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39760676278)中查看，对应源码提交为 `48d71dbbd57454c4a0737f5d13413d675fe101c3`。[此前的检查](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39758596555)也已通过核心测试和 iOS 模拟器编译。

构建工具随后更新到了支持 Node.js 24 的 Actions，并固定使用 macOS 15 Intel 与 Ubuntu 24.04。[新版 IPA 流程测试](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39783000531)已通过设备版编译、上传、重新下载和校验，两个任务的 GitHub 注释均为空。这次测试没有开启发布，上方下载文件和校验和仍对应原先的发布版本。

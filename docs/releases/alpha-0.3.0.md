这次 Android 更新让聊天更接近 Telegram 的使用习惯：先在会话列表里找到同学，再进入独立聊天页。首页可以查看头像、最近消息、时间、草稿和本机未读提示，也能按“全部”“私聊”或“群发”筛选。

聊天页会按日期整理消息，使用带方向的圆角气泡。默认浅色主题搭配浅绿发送气泡和淡灰绿背景，深色主题使用深蓝色背景。输入栏换上了圆形纸飞机发送按钮，快捷短语可以从菜单中选择。

## 原有习惯可以继续保留

主题色、深浅色模式、字号和草稿都会继续使用。常用好友和群发成员可以保存在本机；群发时如果只有部分成员的消息提交失败，保持原文再次发送，会只重试未完成的成员。

未读数字只表示这台设备上有没有查看消息。消息显示“已送达”仍需要收到并验证对方的回执，进入聊天页不会自动给对方发送已读回执。

## 安装和验证进展

这个版本是 **alpha-0.3.0**，内部版本号为 4，适用于 Android 8.0 及以上的 ARM64 设备，使用开发调试签名。后续测试版继续采用 alpha-版本号命名。iOS 的 Liquid Glass 更新在独立的 [alpha-0.4.0 发布页](https://github.com/Yumeshizuku/Campus-mesh/releases/tag/alpha-0.4.0)提供。

提供下载的 APK 已通过本地构建、代码检查、签名和 16 KB 对齐检查。仓库的 Android 云端构建也已修复，并在[最新完整验证](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39877716098)中通过。蓝牙、局域网无线收发和多跳通信仍需要手机实测。

## 下载文件

- [Android 安装包](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/CampusMesh-alpha-0.3.0-debug-arm64.apk) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/CampusMesh-alpha-0.3.0-debug-arm64.apk.sha256)
- [完整交付包](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-alpha-0.3.0-delivery.zip) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-alpha-0.3.0-delivery.zip.sha256)

## 构建流程的后续修复

早期 Android 云端任务曾因下载已经废弃的 SDK 工具而中断。后续更新移除了这个下载步骤，修正了新版 Rust Android 构建工具的参数，并统一了 NDK 路径。[修复后的检查](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39763817340)已经通过 APK 构建、代码检查、核心测试和 iOS 编译。

构建组件随后升级到支持 Node.js 24 的版本，并固定了运行环境。相关变更和各阶段结果见[更新日志](https://github.com/Yumeshizuku/Campus-mesh/blob/main/CHANGELOG.md)与[验证报告](https://github.com/Yumeshizuku/Campus-mesh/blob/main/docs/VERIFICATION.md)。上方 APK 仍是原先发布并完成检查的文件。

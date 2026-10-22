这次 Android 更新把聊天界面调整成了更接近 Telegram 的布局。首页可以查看各个会话的头像、最近消息、时间和本机未读提示，也可以按“全部”“私聊”“群发”筛选。点开会话后，会进入独立聊天页，消息按日期分组，默认浅色主题使用浅绿发送气泡，输入栏则换上了圆形纸飞机发送按钮。

原有的主题色、深浅色模式、字号和草稿都可以继续使用。常用好友和群发成员也能保存在本机；群发时如果只有部分成员提交失败，保持原文再次发送会只重试未完成的成员。未读数字只记录这台设备上的查看情况，消息显示“已送达”仍需要收到并验证对方的送达回执。

测试版现在统一使用 `alpha-版本号` 命名。这个安装包的版本是 **alpha-0.3.0**，内部版本号为 4，适用于 Android 8.0 及以上的 ARM64 设备，使用开发调试签名。iOS 的 Liquid Glass 更新在独立的 iOS 发布版本中提供。

下载文件：

- [Android 安装包](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/CampusMesh-alpha-0.3.0-debug-arm64.apk) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/CampusMesh-alpha-0.3.0-debug-arm64.apk.sha256)
- [完整交付包](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-alpha-0.3.0-delivery.zip) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-alpha-0.3.0-delivery.zip.sha256)

提供下载的 APK 来自已通过本地构建和 Lint 检查的版本，签名与 16 KB 对齐检查也已通过。无线收发和多跳通信还需要在手机上测试。

在[早期 GitHub Actions 检查](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39758596555)中，Android 云端任务曾因尝试下载已废弃的 `tools` 包而中断。这个问题现已修复，同时修正了新版 Rust Android 构建工具的参数，并统一了 NDK 路径。[修复后的完整检查](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39763817340)已通过 Android APK 构建和 Lint，核心测试与 iOS 编译也全部成功。上方提供下载的 APK 仍是此前通过本地检查的文件。

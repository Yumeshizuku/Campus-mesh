# 测试版命名约定

按用户要求，测试版统一采用 `alpha-<版本号>`，例如 `alpha-0.4.0`。后续测试版继续沿用此规则，除非用户另行指定发布阶段。

| 位置 | 当前规则或示例 |
| --- | --- |
| 应用显示名称 | Campus Mesh alpha |
| Android 版本标识 | alpha-0.3.0；内部 versionCode 为 4 |
| iOS 身份页测试版本 | alpha-0.4.0；内部 build 为 5 |
| Android 安装包 | CampusMesh-alpha-0.3.0-debug-arm64.apk |
| Android 完整交付包 | Campus-Mesh-alpha-0.3.0-delivery.zip |
| iOS 源码包 | Campus-Mesh-iOS-alpha-0.4.0-source.zip |

iOS 的 `CFBundleShortVersionString` 保持苹果要求的纯数字 `0.4.0`，`CFBundleVersion` 保持数字构建号；`CMReleaseChannel` 保存 `alpha`，界面和交付包组合显示完整测试版名称。Android `versionName` 直接使用完整测试版名称。两个平台各自维护实际功能版本，不因更改命名而混用版本号。

Android `versionCode` 与 iOS 构建号随重新发布递增；保留原 applicationId 和 bundle identifier，允许按原应用身份更新。发布脚本自动从平台配置读取版本名称。旧归档及旧验证记录保留原始名称，作为历史证据。

# 测试版命名约定

项目测试版统一使用 `alpha-版本号`，例如 `alpha-0.4.0`。应用页面、安装包和 GitHub 发布页都会标明对应版本，方便查找和更新。

| 位置 | 当前规则或示例 |
| --- | --- |
| 应用显示名称 | Campus Mesh alpha |
| Android 版本标识 | alpha-0.3.0；内部 versionCode 为 4 |
| iOS 身份页测试版本 | alpha-0.4.0；内部 build 为 5 |
| Android 安装包 | CampusMesh-alpha-0.3.0-debug-arm64.apk |
| Android 完整交付包 | Campus-Mesh-alpha-0.3.0-delivery.zip |
| iOS 源码包 | Campus-Mesh-iOS-alpha-0.4.0-source.zip |
| iOS 未签名真机包 | CampusMesh-alpha-0.4.0-unsigned.ipa |

iOS 的 `CFBundleShortVersionString` 保持苹果要求的纯数字 `0.4.0`，`CFBundleVersion` 保持数字构建号；`CMReleaseChannel` 保存 `alpha`，界面和交付包组合显示完整测试版名称。Android `versionName` 直接使用完整测试版名称。两个平台各自维护实际功能版本，不因更改命名而混用版本号。

重新发布应用构建时，Android 内部版本号和 iOS 构建号会递增，应用标识保持一致，便于覆盖更新。发布脚本自动从平台配置读取版本名称。旧归档及旧验证记录保留原始名称，作为历史证据。

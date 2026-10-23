这次 iOS 更新在 Telegram 风格的聊天界面中加入了 Apple Liquid Glass。聊天输入栏、快捷短语、发送和“最新消息”按钮，在 iOS 26 及更新系统上会呈现原生玻璃效果，发送按钮的颜色也会跟随你选择的主题。应用最低支持 iOS 16，较早的系统使用兼容的半透明材质。

## 外观可以按习惯调整

你可以在“自定义”页面预览或关闭通透控件。开启系统的降低透明度或增强对比度后，这些控件会使用实色背景；开启减少动态效果后，会关闭自定义玻璃交互和平滑滚动。原有的主题、字号、会话草稿和快捷短语仍然可以使用。

## 安装到自己的 iPhone

当前版本为 **alpha-0.4.0**，内部版本为 0.4.0 / build 5，安装包使用 iPhone ARM64 真机架构。

**下载的 IPA 尚未签名，安装前需要完成个人签名。** 有 Windows 或 Mac 时，可以通过 [Sideloadly](https://sideloadly.io/)，用自己的免费 Apple 账户签名安装，无需上架或购买开发者会员。免费签名通常需要每 7 天续签，覆盖更新时应使用同一账户和应用标识，以保留本机身份与聊天记录。具体步骤见[个人安装说明](https://github.com/Yumeshizuku/Campus-mesh/blob/main/docs/IOS-IPA.md)。

## 已完成的检查

Xcode 26.2 已成功生成设备版 IPA。发布后重新下载的文件通过了包结构、真机架构、执行权限和校验和检查。

2026-10-23，应用也在 iOS 26.2 模拟器上通过了实际操作检查：启动通信核心、切换四个主页面、显示公钥二维码、修改昵称，以及暂停和恢复通信。重新打开应用后，昵称和自定义设置能够正常保留。手机签名安装和真实无线通信仍待真机确认。

[完整验证流程](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39877716098)与 [IPA 构建、上传及下载校验](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39877716031)均已通过。[测试结果和实际截图](https://github.com/Yumeshizuku/Campus-mesh/blob/main/docs/IOS-IPA.md#启动与操作检查)已保存到仓库。

## 下载文件

- [iPhone 未签名 IPA](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.sha256) · [版本与平台信息](https://github.com/Yumeshizuku/Campus-mesh/releases/download/alpha-0.4.0/CampusMesh-alpha-0.4.0-unsigned.ipa.json)
- [原始应用源码包](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-iOS-alpha-0.4.0-source.zip) · [SHA-256 校验文件](https://github.com/Yumeshizuku/Campus-mesh/raw/855e6a86eae23335fa701e091d8c30270f16219e/artifacts/Campus-Mesh-iOS-alpha-0.4.0-source.zip.sha256)
- [最初 IPA 对应的构建源码](https://github.com/Yumeshizuku/Campus-mesh/tree/48d71dbbd57454c4a0737f5d13413d675fe101c3) · [当前构建与安装说明](https://github.com/Yumeshizuku/Campus-mesh/blob/main/docs/IOS-IPA.md)

IPA 大小为 651779 字节，SHA-256 为 `bdc17ee2d78032fbb753b619f0bf271f4f4fd8f63c1b80a7a3d453ba3e710f45`。原始构建和发布记录见[这次运行](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39760676278)。后续检查没有覆盖这个下载文件；测试所用的界面与通信核心源码和发布包对应源码一致。

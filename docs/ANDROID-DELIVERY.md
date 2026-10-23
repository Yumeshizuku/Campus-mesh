# Android 自动交付

## 每次 Verify

Android job 复用 `scripts/build-android.sh` 构建，只在 `lintDebug`、APK 签名与 16 KB ZIP 对齐检查成功后打包。构建前删除 Gradle 的 debug APK 输出目录，打包脚本只读取本次 `android/app/build/outputs/apk/debug`，绝不回退到 `artifacts/` 中的历史安装包。

打开该次 **Actions → Verify → Summary → Artifacts**，下载名称为
`CampusMesh-<versionName>-vc<versionCode>-debug-arm64-<完整提交SHA>-run<运行ID>-<尝试次数>`
的归档。Android job 的摘要也提供直接下载链接。Actions 下载通常需要登录，保留 30 天（受仓库策略约束）；长期公开下载使用 Releases。

归档包含：

- 同名 `.apk`：ARM64，Android 8.0+，开发调试签名。
- `.apk.sha256`：APK 本体 SHA-256，不是 Actions 外层 ZIP 的摘要。
- `SOURCE_COMMIT.txt`：实际 checkout 的完整 Git 提交 SHA。
- `build-info.json`：Gradle 产物版本、内部版本号、应用 ID、ABI、提交、ref、运行 ID、尝试次数、运行链接和 APK 摘要。
- `README.txt`：安装、校验与验证边界。

解压后，在此目录执行 `sha256sum -c *.apk.sha256`。Windows 可使用
`Get-FileHash .\<文件名>.apk -Algorithm SHA256` 与校验文件对比。
核对 JSON 中的 `sourceCommit` 与对应运行实际构建的 SHA；PR 的默认 checkout 是测试合并提交，可能不同于 PR 分支头。JSON 不宣称所有 Verify jobs 已成功，须在运行页面查看完整结果。

调试签名在不同 CI 运行/本地构建间可能变化，Android 可能拒绝覆盖安装。保留所需数据后再决定是否卸载（卸载会删除本机数据）。双 Android 设备测试应安装同一份 APK。这里没有引入或上传生产签名密钥。

## 发布到 GitHub Releases

使用平台前缀标签 `android-<versionName>`，例如 `android-alpha-0.3.1`。这与既有 `alpha-0.3.0` 和 iOS 标签分开，旧 Release、历史 APK、下载链接均保留。

1. 发布新应用构建前，按 `docs/RELEASE-NAMING.md` 更新 Android `versionName`、递增 `versionCode` 并提交。单纯修复工作流不修改应用版本。
2. 将新的 `android-alpha-X.Y.Z` 标签指向包含这套交付流程的目标提交并推送。标签必须与该提交的 Android `versionName` 一致。
3. 标签 push 触发同一 Verify，Android APK 在这次运行中重新构建、校验并保存。
4. 仅当 core、android、ios、ios-ui 全部成功，`android-release` 才下载本次运行中明确命名的归档，再核对 APK SHA-256、版本、提交、标签、运行 ID 和尝试次数。
5. 使用该仓库的 `GITHUB_TOKEN`（仅此 job 的 `contents: write`）和 `gh release create --verify-tag --draft --prerelease` 上传完整交付文件，创建草稿预发布。维护者确认后发布草稿；当前是 alpha 调试包，不能当成生产签名的正式发行版。

workflow_dispatch 只验证并保存 APK，不发布；发布必须由匹配的 Android 标签 push 触发。发布 job 不重新编译，也不从其他运行挑选“最新”APK。`--verify-tag` 防止 CLI 自动创建错误标签，提交身份由解引用后的标签再次核对。已有同名 Release 一律报错，不使用 `--clobber`，不替换附件。重跑遇到已有草稿/部分上传时，先检查其来源并人工处理；正常的新构建应递增版本、使用新标签。若组织或仓库策略禁止 Actions 写入 Releases，草稿创建会失败，Actions 安装包仍可下载。

发布后的永久入口为对应 Release 的 Assets；从该页同时下载 APK、校验文件和 `build-info.json`。Actions 安装包与 Release 安装包应有相同 APK SHA-256。

## 现有记录与验证边界

alpha-0.5.0 已在[本轮 Verify](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911054046)完成完整构建、二维码单元测试、Lint、签名和对齐检查，并保存 APK。下载文件也已在本地核对摘要、版本和来源；实际打包的 PR 临时合并提交为 `71441d5efd36b93de5a2a2e5abc2e594ef70aab7`。原生 UI 和 IPA 的同期检查见[本轮验证记录](reports/alpha-0.5.0/validation.json)，以下条目保留较早阶段的状态。

- 旧运行 `39783000526`：提交 `a8bad7b687c039c031836795b92fe6dada3c6d7b`，成功但 artifacts 为 0，不会因本次修改而补出安装包。
- 检查时较新成功 Verify 为 `39877716098`，提交 `ba7b16bf28707e7979578ace7b78c3b746d62f13`；Android 当时仍无上传步骤。
- 历史 `855e6a86eae23335fa701e091d8c30270f16219e/artifacts/CampusMesh-alpha-0.3.0-debug-arm64.apk` 是原本本地验证的归档。该提交是文件的存储位置，不据此推断其精确源码构建 SHA，也不把它归为任何新 CI 运行的输出。
- 本地 `python3 -m unittest discover -s scripts/tests -p 'test_package_android.py'` 验证打包/篡改检测/来源匹配，使用合成 APK fixture，不代表真实 Android 构建。
- `bash -n scripts/build-android.sh` 与 `git diff --check` 检查脚本语法及补丁空白。
- 新流程的真实 Android 编译、签名/对齐、Actions 上传/下载、标签版本校验与草稿 Release 上传需由对应 CI 实跑确认；双机蓝牙、局域网和多跳通信仍需设备实测。

官方行为参考：
- https://github.com/actions/upload-artifact
- https://github.com/actions/download-artifact
- https://cli.github.com/manual/gh_release_create
- https://cli.github.com/manual/gh_release_upload

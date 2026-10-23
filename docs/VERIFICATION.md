# 验证报告

## alpha-0.5.0 好友与群聊验证

本轮实现了签名好友卡、持久化设备 ID、本机备注、可配置 Mesh 与群主签名的群聊协议。新增覆盖扫码入群、单次和过期邀请、群成员之间经中继收发、送达汇总、退群与解散、离线后群信息同步、容量不足时整条发送回滚，以及旧状态和群发组合迁移的测试。移动端接口还检查备注、参数和群聊的重启保存，以及保存失败后的内存回滚。

[本轮 Verify](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911054046) 已通过核心、Android、iOS 编译和 iOS 操作测试。[IPA 流程](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911048392)也已完成设备包构建、上传、重新下载及校验。应用源码为 `49bbdcaa951736ea79ab5cff510368fbe7a46a71`；Android 构建使用 PR 的临时合并提交 `71441d5efd36b93de5a2a2e5abc2e594ef70aab7`，这不表示 PR 已合并到 main。之后补充的验证文档不改变应用代码。

| 本轮检查 | 结果 |
|---|---|
| Rust 格式、Clippy 与核心测试 | 通过，26 项测试全部成功 |
| 真实 TCP 四进程三跳、分区与重启 | 通过，消息只记录一次且回执返回 |
| 10 / 30 / 100 / 300 虚拟节点仿真 | 通过；仅代表仿真场景 |
| Android 编译、二维码往返测试、Lint、签名与 16 KB 对齐 | 通过；二维码测试覆盖中文和表情 |
| Android 打包脚本测试 | 9 项通过 |
| Intel iOS 编译 | 通过 |
| iOS 26.2 / iPhone SE 模拟器 | 3 项操作测试通过，覆盖设备 ID、昵称、备注、群聊、邀请二维码和 Mesh 参数的重启保存 |
| iPhone ARM64 IPA | 构建与下载校验通过；仍需个人签名 |
| 本地下载复核 | APK 摘要与源码来源、IPA 摘要与设备包结构均通过 |

[下载本轮 APK](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911054046/artifacts/12482538184) · [下载本轮 IPA](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39911048392/artifacts/12482503417) · [结构化验证记录](reports/alpha-0.5.0/validation.json) · [原始操作测试摘要](reports/alpha-0.5.0/ios-ui-summary.json)

操作截图：[设备身份](reports/alpha-0.5.0/03-identity.png)、[Mesh 参数](reports/alpha-0.5.0/05-mesh-settings.png)、[群邀请](reports/alpha-0.5.0/06-group-invite.png)、[好友备注](reports/alpha-0.5.0/07-friend-remark.png)。模拟器结果不包含手机安装、相机实际扫码、蓝牙、无线多跳或熄屏验收。


更新至 2026-10-23（UTC）。Android APK 和 iPhone IPA 均已完成构建，核心通信测试与 iOS 模拟器基础操作检查已通过。iPhone 下载包仍需个人签名，手机安装、跨平台无线收发和无线多跳还需要真机验证。下方按阶段保留检查范围、结果和原始记录。

## 环境与工具

初始 `/workspace` 无项目，只有 Java 运行环境，未安装 Rust、Android SDK、Swift 或 Xcode。通过 cloud-environment-runtime 技能检查网络与环境后，在工作区创建真实工程。未访问学校设备、未修改校园网络、未绕过任何权限或安装限制。

本轮实际使用：Linux x86_64、Rust 1.99.1、Temurin JDK 21.0.12.1、Gradle 8.10.2、AGP 8.7.3、Kotlin 2.1.0、Android Platform 35、Build Tools 35.0.0、NDK 27.2.12479018、CMake 3.22.1、Python 3。依赖由 Cargo.lock 和 Gradle 配置指定；Gradle distribution 有 SHA-256 校验。工具安装在本次临时环境，不将本机绝对 SDK 路径写入项目配置。

最初的 Linux 验证环境无 macOS/Xcode，`adb devices -l` 未发现任何设备，未获得手机蓝牙射频访问。后续已接入 GitHub macOS/Xcode 云端编译；手机安装运行与真实无线容量仍需设备验收。

## 已执行结果

| 验证 | 结果 | 原始证据 |
|---|---|---|
| Rust 格式检查 | 通过 | `reports/rust-format.txt` |
| Rust 集成测试 | 9/9 通过 | `reports/rust-tests.txt` |
| 全工作区 Clippy，warnings-as-errors | 通过 | `reports/rust-clippy.txt` |
| Rust CLI / 库构建 | 通过 | `reports/rust-build.txt` |
| 四独立进程，真实 TCP 三跳及恢复 | 通过，本次约 25.76 秒 | `reports/real-tcp-processes.json` |
| 10/30/100/300 节点移动接触仿真 | 本次全部消息、ACK 交付 | `reports/simulation.jsonl` |
| Android ARM64 Rust 交叉编译 | 通过 | `reports/android-rust.txt` |
| Android `assembleDebug` + `lintDebug` | 通过，Lint 0 errors / 10 warnings | `reports/android-build-lint.txt`、`reports/android-lint.txt` |
| Android GitHub 云端完整构建与 Lint | SDK 初始化、Rust/JNI/Kotlin 编译、`assembleDebug` 与 `lintDebug` 全部通过 | [运行 39763817340 的 android 任务](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39763817340/job/119774211823) |
| 工作流升级后的完整验证 | 核心测试、Android APK 与 Lint、iOS 模拟器编译全部通过；三个任务的 GitHub 注释均为空 | [Verify 运行 39783000526](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39783000526) |
| 工作流升级后的 IPA 构建与传输 | Intel Mac 成功构建 ARM64 iPhone IPA，上传后在 Ubuntu 重新下载并完成结构与校验和检查；两个任务的 GitHub 注释均为空 | [IPA 运行 39783000531](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39783000531) |
| APK 签名与 16 KB zip alignment | 通过，v2 调试签名 | `reports/apk-signature.txt`、`reports/apk-alignment.txt` |
| iOS arm64 设备 Rust 静态库 | 编译通过 | `reports/ios-rust-device.txt` |
| iOS arm64 模拟器 Rust 静态库 | 编译通过 | `reports/ios-rust-simulator.txt` |
| Swift 源码解析 | 无语法解析错误；不等于类型检查/构建 | `reports/swift-syntax.txt` |
| iOS SwiftUI/CoreBluetooth App 模拟器编译 | GitHub macOS / Xcode 26.2 编译通过 | [运行 39758596555 的 ios 任务](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39758596555) |
| iPhone ARM64 Release 归档及 IPA 发布 | 成功；未签名 IPA 下载后的结构、平台与 SHA-256 检查通过 | [运行 39760676278](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39760676278)、[IPA 记录](IOS-IPA.md) |
| iOS 26.2 模拟器实际操作 | 完整操作用例 1/1 通过：启动、公钥二维码、四个页面、昵称修改、通信启停，以及昵称与外观偏好的重启保存 | [运行 39877716098](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39877716098)、[报告与截图记录](reports/ios-personal-use.json) |
| BLE / Wi-Fi Direct / 手机 LAN / 无线多跳 | **未执行，需要真机** | `DEVICE-TESTS.md` 待执行清单 |

Android Lint 警告主要为内联界面字符串、较新版本权限属性、依赖更新提示与缺少 x86_64 ChromeOS ABI。未忽略 Lint 错误。为兼容旧版 Android 保留了一些已弃用 GATT API，构建有对应 deprecation warnings。Rust ELF LOAD segment 已设为 0x4000 对齐，JNI CMake 也开启 flexible page sizes；APK alignment 检查不代表 16 KB 真机运行已经通过。

## 9 项核心测试的边界

1. 三跳加密封装、目的解密、反向端到端 ACK，中继历史无明文。
2. 分区后携带、状态序列化重启、重接触、去重、24 小时 TTL 与关闭中继。
3. 篡改签名字段、密文、跳数超限与未知发送者拒绝。
4. 10/30/100/300 节点固定接触图的三跳交付。
5. 真实 TCP 双向交换和 ACK；使用真实 socket，无 mock。
6. 逐成员群发标签与文本长度限制。
7. 磁盘持久化、Unix 0600 权限、消息过期状态、轮转批次覆盖全部排队消息。
8. C ABI 中文/emoji、状态 API 不暴露私钥、独占锁、坏状态文件拒绝启动且不覆盖。
9. 多字节昵称不会超过协议身份长度限制。

## 四进程真实 TCP 验证

运行 `cargo build --workspace --locked && python3 scripts/verify-processes.py`。

拓扑只配置 A→B、B→C、C→D 的邻居，各 TCP 连接本身双向同步；没有 A→C/A→D 捷径。先停止 C，再从 A 发送中文消息，确认 B 获得密文而 D 尚无记录。随后重启 B、C，要求它们从磁盘恢复身份、队列与邻居。最终 D 只出现一条相同消息，A 状态为 `delivered`，B/C 的聊天历史为空。结束时清理含私钥的临时状态文件。

这证明真实网络字节、共享核心、磁盘恢复及三跳协议协作。四个进程仍在同一 Linux 主机上，不能作为四台手机或三个射频链路的验收证据。

## 移动接触 / 丢包 / 分区仿真

运行 `cargo run --release --locked --bin mesh-sim`。

生产传输不依赖这个程序。仿真调用真实 Engine 处理签名、加密、去重、TTL、ACK；接触与丢包由测试调度器控制。每轮 10 虚拟秒、总计 400 秒；前 120 秒分为两个不互通的分区，再合并为随机接触网络。每次尝试有 10% 丢弃概率，并对两个节点执行状态重启。消息跨分区发送；接触时用已见 ID 避免重复传递。该场景设置消息接收率、ACK 返回率均至少 90% 的回归门槛，不是现实校园网络 SLA。

| 虚拟节点 | 消息 | 收到 | ACK | 本次传输尝试 | 本次丢弃 |
|---:|---:|---:|---:|---:|---:|
| 10 | 10 | 10 | 10 | 204 | 24 |
| 30 | 30 | 30 | 30 | 1938 | 198 |
| 100 | 30 | 30 | 30 | 6595 | 655 |
| 300 | 30 | 30 | 30 | 19955 | 2015 |

接触/丢包使用指定种子，但密钥和消息 ID 仍使用操作系统安全随机源，缓存排序会影响后续事件，重复运行的精确计数可能不同。这里保留本次原始 JSON 结果。100/300 节点只发送 30 条消息，未模拟 300 人同时连续群聊、蓝牙带宽、无线干扰、CPU 降频或电池消耗。全数交付仅适用于本次场景。

## 可复现构建与剩余验收

普通机器遵循 README 和 `RUNNING.md`。本环境网络使用提供的 HTTP 代理和系统 CA 信任库；TLS 校验保持开启。JDK 必须包含编译器，单独 JRE 不能构建 APK。Android 脚本先编译 Rust，再编译原生桥接及 Kotlin；iOS 脚本构建设备与匹配主机架构的模拟器库后调用 XcodeGen/Xcodebuild。设备 IPA 的流程与状态见 [IPA 构建与下载](IOS-IPA.md)。

早期 Android 云端构建因下载已废弃的 `tools` 包而中断。修复后，初始化明确安装 `platform-tools`，构建脚本也改用新版 cargo-ndk 的 `--platform 26` 参数，并将 NDK 路径统一到指定版本。2026-10-22，[运行 39763817340](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39763817340) 的 Android、core 和 iOS 任务全部通过，对应代码提交为 `f49bf1778609b840fd89f9dbb85fe596cdb297ed`。其中 Android 完成了完整 APK 构建和 Lint 检查；此前的本地构建记录作为独立结果保留。

随后又将 JavaScript Actions 更新到支持 Node.js 24 的版本，并固定使用 `ubuntu-24.04` 与 `macos-15-intel`。提交 `a8bad7b687c039c031836795b92fe6dada3c6d7b` 已完成上表中的两条工作流测试，5 个实际执行任务的注释合计为 0 错误、0 警告、0 通知。这里统计的是 GitHub 检查注释；应用编译日志中的一般提示仍按各自日志记录。IPA 的自动构建、上传、下载和校验均已实测，手动发布选项未开启，因此没有覆盖现有 Release 安装包。结构化记录见 [工作流测试结果](reports/workflow-modernization.json)。

下一阶段先运行 `DEVICE-TESTS.md` 的双 Android / Android-iPhone BLE 和 LAN 用例，再验证无旁路的四手机三跳与混合链路；修复原生兼容性后逐级增加节点与负载。大规模群聊和离线成员变更验收、链路能耗评分、硬件密钥保护和生产级洪泛控制仍属未完成项，见 `LIMITATIONS.md`。

GitHub macOS 环境已补齐 Xcode 编译能力。剩余真机验收需要真实 Android/iOS 设备，普通 iPhone 安装还需要有效 Apple 签名与匹配的描述文件。

2026-10-23，新增的运行检查已在 Apple Silicon 上的 iPhone SE / iOS 26.2 模拟器通过。四项 Verify 任务和两项 IPA 构建、校验任务均成功；GitHub 注释合计为 0 错误、0 警告、1 条通知。该通知是 GitHub 对 macOS ARM64 运行器排队容量的提示，本次任务已实际完成。设备 IPA 构建与 Intel 模拟器编译仍使用 `macos-15-intel`。个人安装可使用免费 Apple 账户，步骤见[个人安装说明](IOS-IPA.md)；当前没有手机签名安装或无线通信的真机验收结果。

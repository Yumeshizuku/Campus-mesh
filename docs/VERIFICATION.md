# 验证报告

日期：2026-10-22（UTC）。结论：**可构建的 Android 开发原型与跨平台源码已交付；完整手机无线验收受当前环境阻塞，未完成最终产品验收。**

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
| APK 签名与 16 KB zip alignment | 通过，v2 调试签名 | `reports/apk-signature.txt`、`reports/apk-alignment.txt` |
| iOS arm64 设备 Rust 静态库 | 编译通过 | `reports/ios-rust-device.txt` |
| iOS arm64 模拟器 Rust 静态库 | 编译通过 | `reports/ios-rust-simulator.txt` |
| Swift 源码解析 | 无语法解析错误；不等于类型检查/构建 | `reports/swift-syntax.txt` |
| iOS SwiftUI/CoreBluetooth App 模拟器编译 | GitHub macOS / Xcode 26.2 编译通过 | [运行 39758596555 的 ios 任务](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39758596555) |
| iPhone ARM64 Release 归档及 IPA 发布 | 成功；未签名 IPA 下载后的结构、平台与 SHA-256 检查通过 | [运行 39760676278](https://github.com/Yumeshizuku/Campus-mesh/actions/runs/39760676278)、[IPA 记录](IOS-IPA.md) |
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

普通机器遵循 README 和 `RUNNING.md`。本环境网络使用提供的 HTTP 代理和系统 CA 信任库；TLS 校验保持开启。JDK 必须包含编译器，单独 JRE 不能构建 APK。Android 脚本先编译 Rust，再编译原生桥接及 Kotlin；iOS 脚本构建设备与匹配主机架构的模拟器库后调用 XcodeGen/Xcodebuild。仓库已推送并执行远程 CI：上述运行中的 core 与 ios 通过，Android 云端任务因 SDK 初始化下载已废弃的 `tools` 包失败；此前本地 Android APK 与 Lint 结果保持为独立证据。设备 IPA 的流程与状态见 [IPA 构建与下载](IOS-IPA.md)。

下一阶段先运行 `DEVICE-TESTS.md` 的双 Android / Android-iPhone BLE 和 LAN 用例，再验证无旁路的四手机三跳与混合链路；修复原生兼容性后逐级增加节点与负载。完整群成员协议、链路能耗评分、硬件密钥保护和生产级洪泛控制仍属未完成项，见 `LIMITATIONS.md`。

GitHub macOS 环境已补齐 Xcode 编译能力。剩余真机验收需要真实 Android/iOS 设备，普通 iPhone 安装还需要有效 Apple 签名与匹配的描述文件。

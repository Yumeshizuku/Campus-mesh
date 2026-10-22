# 运行与排障

## Linux / macOS / 自有 Windows CLI

```sh
cargo build --release --locked --bin mesh-node
./target/release/mesh-node ./alice.json Alice 37078 --discover
```

每行输入一个 JSON 命令，输出一行 JSON。`quit` 优雅停止。不同节点使用独立状态文件、独立地址/端口，不可共享状态文件。`--discover` 可省略，手动连接更便于限制测试拓扑。状态文件包含私钥与明文历史，不要发给他人、提交仓库或放进公共目录。

```json
{"op":"state"}
{"op":"peer","address":"192.168.1.25:37078"}
{"op":"contact","contact":{"nickname":"Bob","sign":"对方64位十六进制Ed25519公钥","exchange":"对方64位十六进制X25519公钥"}}
{"op":"send","to":"对方指纹（contact命令返回id）","text":"下课见"}
{"op":"send","to":"对方指纹","text":"小组讨论","group":"study-room-1"}
{"op":"relay","enabled":false}
```

好友双方均添加对方 identity。每份 `state.data.identity` 可直接作为对方 `contact` 内容。指纹由公钥计算，不能把昵称作为收件地址。中继无需添加通信双方好友，保持 `relay:true` 即可。

关闭中继后仍收发本人的文本和 ACK，但不继续广播第三方缓存。停止前台服务或 iOS 后台暂停可进一步省电。除 24 小时 TTL 外没有保证交付时限。

## Android

- 工具链：JDK 17/21（必须有 `javac`，JRE 不够），SDK 35、Build Tools 35.0.0、NDK 27.2.12479018、CMake 3.22.1、Gradle wrapper 8.10.2。
- `ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018 ./scripts/build-android.sh`。脚本用 cargo-ndk 编译真实 Rust ARM64 库，再编译 JNI 和 Kotlin；不复用占位库。
- `adb install -r android/app/build/outputs/apk/debug/app-debug.apk`。当前 APK 最低 Android 26，只打包 arm64-v8a；x86 模拟器需要修改 ABI 并另编译 Rust x86_64 Android 库。
- 首次设置昵称，公钥身份持久化。好友二维码支持中文，扫描库打包在应用内，不需在线下载扫码模型。
- BLE：附近→启用蓝牙，两端允许系统权限。Android 11 及以前的 BLE 扫描还需定位权限及系统定位开关；Android 12+ 使用蓝牙运行时权限。某些设备缺少外设广播能力，但仍可作为中心尝试连接。
- Wi-Fi Direct：双方发现设备，选择对方并接受系统配对。Android 12 及以前需要定位权限/开关；Android 13+ 使用附近 Wi-Fi 权限。未支持的设备会显示错误。客户端连接群主 IP 后复用同一 TCP 核心。
- 正常 LAN：两端同一 Wi-Fi/热点，在附近填对方实际 IPv4:37078；广播受限时手动连接仍可尝试。Android 前台通知展示运行状态，应用身份页可停止整个服务。

## iOS

- `brew install xcodegen`，选择 Xcode 26+ 并安装 iOS 26+ 设备与模拟器 SDK，再运行 `scripts/build-ios.sh`。脚本先检查工具链，再产出设备及匹配 Mac 主机架构的模拟器 Rust 库，生成原生工程并构建模拟器 App；最低运行系统仍为 iOS 16。
- iOS 26+ 使用原生 Liquid Glass；旧系统使用兼容半透明材质。在“自定义 → Liquid Glass”关闭通透控件可使用实色；降低透明度和增强对比度也会自动切换自定义控件。系统导航栏遵循系统设置。见 [适配说明](LIQUID-GLASS.md)。
- 在 Xcode 设置自己的 Team、签名与唯一 bundle identifier；真机需要正常开发签名。项目没有自动添加额外的后台模式或 multicast entitlement。
- 扫描二维码需要相机权限；蓝牙需要 Bluetooth 权限；LAN 需要本地网络许可。被拒绝后在系统设置恢复权限。
- 启用蓝牙会启动 CoreBluetooth 双角色发现。TCP 手动输入对方 IP。不要以 iOS 模拟器验证真实 BLE；模拟器通常不能代替物理蓝牙设备。
- 退后台将暂停通信，重新回前台恢复缓存及授权的手动邻居。现场中继实验要把应用保持在前台并控制自动锁屏。

## 常见现象

- 已发现但不能聊天：检查双方是否添加公钥，未信任发送者不会进入收件箱。
- 收到了但发送方仍 queued：反向 ACK 尚未回到源设备，允许中继/恢复接触后继续等待；不要把 GATT 写入成功当成送达。
- LAN 无连接：确认 IP、同一子网、端口、系统本地网络许可及 AP 客户端隔离。校园管理策略不能绕过。
- BLE 连接不出现：检查蓝牙权限、扫描/广播硬件支持、设备前台状态和旧版 Android 定位开关。降低同时参与设备数后重试并记录系统日志。
- 启动失败：端口已占用、文件锁冲突、状态文件损坏或存储不可写。不要直接删除状态文件，否则丢失身份；先保留私有备份诊断。
- 修改源码后 APK 看起来没变：重新构建 Rust `.so`，再重建 APK；只编译 Kotlin 不会重新编译 Rust。构建脚本负责两者。

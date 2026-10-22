# 可选 Windows 授权中继

核心已把 LAN 同步与平台分离；Windows 可复用同一 `mesh-node`，不解密非本机消息。该入口未在 Windows 上构建/运行验收。

仅在自己的测试设备或取得明确授权后：安装 Rust MSVC 工具链及 Visual Studio C++ Build Tools，在 PowerShell 执行 `./windows/Start-Relay.ps1 -Authorized`，再按运行文档添加授权手机的 IP:port。允许必要防火墙端口须通过设备正常管理流程完成，脚本不修改防火墙、不提权、不注册常驻服务、不进行网络扫描或远程安装。

手机可以作为 TCP 客户端把该电脑加入 peer；电脑可保存和转发经验证签名的密文。手机 BLE 传输和普通 LAN 通信不依赖这台电脑。学校现有白板无法安装软件，故完全不依赖或部署于其上。

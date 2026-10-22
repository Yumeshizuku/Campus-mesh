import SwiftUI
import CoreImage.CIFilterBuiltins
import UserNotifications
@main struct CampusMeshApp: App {
    @StateObject private var mesh = MeshStore()
    @StateObject private var appearance = AppearanceSettings()
    @Environment(\.scenePhase) private var phase
    var body: some Scene {
        WindowGroup { ContentView().environmentObject(mesh).environmentObject(appearance).tint(appearance.accent).preferredColorScheme(appearance.colorScheme).onAppear { mesh.start() }.onChange(of: phase) { value in
            if value == .active { mesh.start() } else if value == .background { mesh.stop() }
        } }
    }
}
@MainActor final class MeshStore: ObservableObject {
    @Published var isRunning = false
    @Published var state: [String:Any] = [:]
    @Published var error = ""
    @Published var bleStatus = "BLE 未启用"
    @Published var bleEnabled = false
    private var ble: BLEMesh?
    @Published var nickname = UserDefaults.standard.string(forKey:"nickname") ?? "同学"
    private var userStopped = false
    private var handle: UnsafeMutableRawPointer?
    private var timer: Timer?
    private var received = Set<String>()
    private var baseline = false
    var identity: [String:Any] { state["identity"] as? [String:Any] ?? [:] }
    var contacts: [String:[String:Any]] { state["contacts"] as? [String:[String:Any]] ?? [:] }
    var history: [[String:Any]] { state["history"] as? [[String:Any]] ?? [] }
    var peers: [[String:Any]] { state["peers"] as? [[String:Any]] ?? [] }
    func start(userInitiated: Bool = false) {
        if userInitiated { userStopped = false }
        guard !userStopped else { return }
        guard handle == nil else { return }
        let directory = FileManager.default.urls(for:.applicationSupportDirectory,in:.userDomainMask)[0]
        do { try FileManager.default.createDirectory(at:directory,withIntermediateDirectories:true)
            try (directory as NSURL).setResourceValue(true,forKey:.isExcludedFromBackupKey)
            try FileManager.default.setAttributes([.protectionKey:FileProtectionType.complete],ofItemAtPath:directory.path)
        } catch { self.error = error.localizedDescription; return }
        let path = directory.appendingPathComponent("mesh.json").path
        // Broadcast discovery requires Apple's multicast entitlement. Manual TCP
        // addressing works without that entitlement; keep discovery opt-in.
        let discovery = UserDefaults.standard.bool(forKey:"discovery")
        handle = path.withCString { path in nickname.withCString { cm_open(path,$0,37078,discovery) } }
        guard handle != nil else { error = "核心启动失败：检查状态文件与 TCP 端口"; return }
        isRunning = true
        refresh()
        if bleEnabled { startBLE() }
        timer = Timer.scheduledTimer(withTimeInterval:2,repeats:true) { [weak self] _ in Task { @MainActor in self?.refresh() } }
        UNUserNotificationCenter.current().requestAuthorization(options:[.alert,.sound]) { _,_ in }
    }
    func stop(userInitiated: Bool = false) { isRunning = false; if userInitiated { userStopped = true }; ble?.stop(); ble = nil; timer?.invalidate(); timer = nil; if let h = handle { cm_close(h); handle = nil } }
    func startBLE() {
        bleEnabled = true
        guard ble == nil, handle != nil else { return }
        let adapter = BLEMesh()
        adapter.call = { [weak self] in self?.call($0) }
        adapter.onStatus = { [weak self] in self?.bleStatus = $0 }
        ble = adapter; adapter.start()
    }
    func stopBLE() { bleEnabled = false; ble?.stop(); ble = nil }
    @discardableResult func call(_ command: [String:Any]) -> Any? {
        guard let h = handle else { error = "通信已停止，请返回前台或启动服务"; return nil }
        do {
            let input = try JSONSerialization.data(withJSONObject:command)
            let json = String(decoding:input,as:UTF8.self)
            guard let output = json.withCString({cm_call(h,$0)}) else { return nil }
            defer { cm_free(output) }
            let response = try JSONSerialization.jsonObject(with:Data(String(cString:output).utf8)) as? [String:Any] ?? [:]
            guard response["ok"] as? Bool == true else { error = response["error"] as? String ?? "请求失败"; return nil }
            return response["data"]
        } catch { self.error = error.localizedDescription; return nil }
    }
    func refresh() {
        guard let value = call(["op":"state"]) as? [String:Any] else { return }
        state = value
        for message in history where message["incoming"] as? Bool == true {
            guard let id = message["id"] as? String else { continue }
            if received.insert(id).inserted && baseline {
                let content = UNMutableNotificationContent(); content.title = "Campus Mesh"; content.body = "收到一条离线消息，打开应用查看"
                UNUserNotificationCenter.current().add(UNNotificationRequest(identifier:id,content:content,trigger:nil))
            }
        }
        baseline = true
    }
    func addContact(_ text:String) {
        do { let contact = try JSONSerialization.jsonObject(with:Data(text.utf8)); call(["op":"contact","contact":contact]); refresh() }
        catch { self.error = "无效的公钥二维码或 JSON" }
    }
    var publicJSON: String { guard let data = try? JSONSerialization.data(withJSONObject:identity,options:[.sortedKeys]) else { return "" }; return String(decoding:data,as:UTF8.self) }
}

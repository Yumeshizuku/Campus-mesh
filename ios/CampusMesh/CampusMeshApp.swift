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
        migrateLegacyGroups()
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
        adapter.syncInterval = { [weak self] in (self?.settings["sync_interval_secs"] as? NSNumber)?.doubleValue ?? 5 }
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
    var deviceID:String {state["device_id"] as? String ?? ""}
    var groups:[MeshGroup] {(state["groups"] as? [String:[String:Any]] ?? [:]).values.map(MeshGroup.init).sorted{$0.name<$1.name}}
    var settings:[String:Any] {state["settings"] as? [String:Any] ?? [:]}
    var pendingGroups:Int {(state["pending_groups"] as? [String:Any] ?? [:]).count}
    func displayName(_ id:String,fallback:String="同学")->String {contacts[id]?["display_name"] as? String ?? fallback}
    func groupName(_ id:String)->String {groups.first{$0.id==id}?.name ?? "旧群聊记录"}
    func addContact(_ text:String) {if call(["op":"scan","text":text]) != nil {refresh()}}
    private func migrateLegacyGroups() {
        let defaults=UserDefaults.standard
        guard let data=defaults.data(forKey:"ui.groups"),let presets=try? JSONDecoder().decode([LegacyGroupPreset].self,from:data) else{return}
        var remaining=[LegacyGroupPreset]()
        for preset in presets {
            guard let result=call(["op":"group_migrate","name":preset.name,"members":preset.members]) as? [String:Any],let id=result["id"] as? String else{remaining.append(preset);continue}
            let old="g:\(preset.name)",new="g:\(id)"
            // An unfinished old fanout stays a draft; it is never sent automatically.
            if let draft=defaults.string(forKey:"ui.draft.\(old)") {defaults.set(draft,forKey:"ui.draft.\(new)")}
            if let pending=defaults.data(forKey:"ui.pending.\(old)"),let raw=try? JSONSerialization.jsonObject(with:pending) as? [String:Any],let text=raw["text"] as? String,defaults.string(forKey:"ui.draft.\(new)")==nil {defaults.set(text,forKey:"ui.draft.\(new)")}
            defaults.removeObject(forKey:"ui.pending.\(old)");defaults.removeObject(forKey:"ui.draft.\(old)")
            let favorites=defaults.stringArray(forKey:"ui.favorites") ?? []
            defaults.set(favorites.map{$0==old ? new : $0},forKey:"ui.favorites")
        }
        if remaining.isEmpty {defaults.removeObject(forKey:"ui.groups")}
        else if let data=try? JSONEncoder().encode(remaining) {defaults.set(data,forKey:"ui.groups")}
    }
    var publicJSON:String {jsonText(state["contact_card"] ?? [:])}
}
func jsonText(_ value:Any)->String {
    guard let data=try? JSONSerialization.data(withJSONObject:value,options:[.sortedKeys]) else{return ""}
    return String(decoding:data,as:UTF8.self)
}

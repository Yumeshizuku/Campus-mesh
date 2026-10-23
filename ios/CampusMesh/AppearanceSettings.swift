import SwiftUI

struct LegacyGroupPreset: Codable { var name:String; var members:[String] }
struct MeshGroup: Identifiable {
    var id:String; var name:String; var ownerID:String; var members:[String:[String:Any]]
    var joined:Bool; var dissolved:Bool
    static var empty:MeshGroup {MeshGroup(id:"",name:"",ownerID:"",members:[:],joined:true,dissolved:false)}
    init(id:String,name:String,ownerID:String,members:[String:[String:Any]],joined:Bool,dissolved:Bool) {
        self.id=id;self.name=name;self.ownerID=ownerID;self.members=members;self.joined=joined;self.dissolved=dissolved
    }
    init(_ raw:[String:Any]) {
        id=raw["id"] as? String ?? "";name=raw["name"] as? String ?? "群聊"
        let owner=raw["owner"] as? [String:Any] ?? [:]
        let all=raw["members"] as? [String:[String:Any]] ?? [:]
        ownerID=all.first(where:{($0.value["sign"] as? String)==(owner["sign"] as? String)})?.key ?? ""
        members=all;joined=raw["joined"] as? Bool ?? false;dissolved=raw["dissolved"] as? Bool ?? false
    }
}

/// Local UI preferences never change identity, trust or transport permissions.
final class AppearanceSettings: ObservableObject {
    private let defaults = UserDefaults.standard
    @Published var theme = UserDefaults.standard.string(forKey:"ui.theme") ?? "system" {
        didSet { defaults.set(theme,forKey:"ui.theme") }
    }
    @Published var accentName = UserDefaults.standard.string(forKey:"ui.accent") ?? "blue" {
        didSet { defaults.set(accentName,forKey:"ui.accent") }
    }
    @Published var fontSize = UserDefaults.standard.object(forKey:"ui.font") as? Double ?? 16 {
        didSet { defaults.set(fontSize,forKey:"ui.font") }
    }
    @Published var compact = UserDefaults.standard.bool(forKey:"ui.compact") {
        didSet { defaults.set(compact,forKey:"ui.compact") }
    }
    @Published var timestamps = UserDefaults.standard.object(forKey:"ui.timestamps") as? Bool ?? true {
        didSet { defaults.set(timestamps,forKey:"ui.timestamps") }
    }
    @Published var glassControls = UserDefaults.standard.object(forKey:"ui.glassControls") as? Bool ?? true {
        didSet { defaults.set(glassControls,forKey:"ui.glassControls") }
    }
    @Published var phrases = UserDefaults.standard.stringArray(forKey:"ui.phrases") ?? ["我到了","在哪个教室？","下课见"] {
        didSet { defaults.set(phrases,forKey:"ui.phrases") }
    }
    @Published var favorites = Set(UserDefaults.standard.stringArray(forKey:"ui.favorites") ?? []) {
        didSet { defaults.set(Array(favorites),forKey:"ui.favorites") }
    }
    @Published var readIDs = Set(UserDefaults.standard.stringArray(forKey:"ui.readIDs") ?? []) {
        didSet { defaults.set(Array(readIDs),forKey:"ui.readIDs") }
    }
    func markRead(_ thread:String, messages:[DisplayMessage]) {
        let available=Set(messages.map(\.id))
        let updated=readIDs.intersection(available).union(messages.filter{$0.thread==thread && $0.incoming}.map(\.id))
        if updated != readIDs { readIDs=updated }
    }
    var colorScheme: ColorScheme? { theme == "dark" ? .dark : theme == "light" ? .light : nil }
    var accent: Color {
        switch accentName { case "blue": return .blue; case "violet": return .purple; case "amber": return .brown; default: return .teal }
    }
    func draft(_ key:String)->String { defaults.string(forKey:"ui.draft.\(key)") ?? "" }
    func saveDraft(_ text:String,for key:String) { defaults.set(text,forKey:"ui.draft.\(key)") }
    func restoreAppearance() { theme="system";accentName="blue";fontSize=16;compact=false;timestamps=true;glassControls=true }
}

struct DisplayMessage: Identifiable {
    let id:String
    let peer:String
    let text:String
    let group:String?
    let incoming:Bool
    let status:String
    let senderName:String
    let delivery:[String:String]
    let time:Date
    init(_ raw:[String:Any]) {
        id=raw["id"] as? String ?? "";peer=raw["peer"] as? String ?? ""
        text=raw["text"] as? String ?? "";group=raw["group"] as? String
        incoming=raw["incoming"] as? Bool ?? false
        status=raw["status"] as? String ?? "queued"
        senderName=raw["sender_name"] as? String ?? "同学"
        delivery=raw["delivery"] as? [String:String] ?? [:]
        time=Date(timeIntervalSince1970:(raw["time"] as? NSNumber)?.doubleValue ?? 0)
    }
    var statusLabel:String {
        switch status {case "delivered":return "已送达";case "partially_delivered":return "已送达 \(delivery.values.filter{$0=="delivered"}.count)/\(delivery.count)";case "received":return "已收到";case "expired":return "已过期";default:return "等待送达"}
    }
    var thread:String { if let group=group {return "g:\(group)"};return "p:\(peer)" }
}

struct ConversationRow:Identifiable {
    let id:String
    let last:DisplayMessage?
    let unread:Int
}

/// Build once per UI snapshot, rather than rescanning history for every row.
func makeConversationRows(history:[[String:Any]],contacts:[String:[String:Any]],groups:[MeshGroup],favorites:Set<String>,readIDs:Set<String>,query:String)->[ConversationRow] {
    let messages=history.map(DisplayMessage.init)
    var latest=[String:DisplayMessage](),unread=[String:Int]()
    for message in messages {
        latest[message.thread]=message
        if message.incoming && !readIDs.contains(message.id){unread[message.thread,default:0]+=1}
    }
    func title(_ key:String)->String {key.hasPrefix("g:") ? (groups.first{$0.id==String(key.dropFirst(2))}?.name ?? "旧群聊记录") : contacts[String(key.dropFirst(2))]?["display_name"] as? String ?? "同学"}
    let keys=Set(contacts.keys.map{"p:\($0)"}+groups.map{"g:\($0.id)"}+Array(latest.keys))
    return keys.filter{query.isEmpty || title($0).localizedCaseInsensitiveContains(query) || latest[$0]?.text.localizedCaseInsensitiveContains(query)==true}.sorted{a,b in
        let af=favorites.contains(a),bf=favorites.contains(b)
        if af != bf {return af}
        let at=latest[a]?.time ?? .distantPast,bt=latest[b]?.time ?? .distantPast
        if at != bt {return at>bt};return title(a)<title(b)
    }.map{ConversationRow(id:$0,last:latest[$0],unread:unread[$0] ?? 0)}
}

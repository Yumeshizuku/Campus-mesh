import SwiftUI
import CoreImage.CIFilterBuiltins
import UIKit

struct NearbyView: View {
    @Binding var scanning:Bool
    @EnvironmentObject private var mesh:MeshStore
    @State private var address=UserDefaults.standard.string(forKey:"ui.lastAddress") ?? ""
    @State private var contactJSON=""
    @State private var showImport=false
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Label("先认识，再连接",systemImage:"checkmark.shield.fill").font(.headline)
                    Text("两端互扫二维码，并当面核对公钥。无需互联网。").font(.subheadline).foregroundStyle(.secondary)
                    Button {scanning=true} label:{Label("扫描好友或群邀请二维码",systemImage:"qrcode.viewfinder")}
                    Button("粘贴二维码文本") {showImport=true}
                }
                Section("蓝牙直连") {
                    LabeledContent("当前状态",value:mesh.bleStatus)
                    Button(mesh.bleEnabled ? "停止蓝牙" : "启用蓝牙") {if mesh.bleEnabled {mesh.stopBLE()}else{mesh.startBLE()}}
                    Text("没有 Wi-Fi 也能尝试连接；iOS 需保持应用在前台。无线兼容性仍待真机验证。").font(.caption).foregroundStyle(.secondary)
                }
                Section("同一 Wi-Fi / 热点") {
                    ForEach(Array(mesh.peers.enumerated()),id:\.offset) {_,peer in
                        LabeledContent(peer["address"] as? String ?? "设备",value:peer["connected"] as? Bool == true ? "已连接" : "等待重连")
                    }
                    TextField("对方 IPv4:37078",text:$address).textInputAutocapitalization(.never).autocorrectionDisabled().keyboardType(.URL)
                    Button("连接设备") {if mesh.call(["op":"peer","address":address.trimmingCharacters(in:.whitespacesAndNewlines)]) != nil {UserDefaults.standard.set(address,forKey:"ui.lastAddress");mesh.refresh()}}
                        .disabled(address.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty || !mesh.isRunning)
                }
                Section("Mesh 多跳") {NavigationLink("详细传输参数") {MeshSettingsView()}}
                Section("已记住的好友") {ForEach(mesh.contacts.keys.sorted(),id:\.self) {id in NavigationLink(mesh.displayName(id)) {FriendDetailsView(id:id)}}}
                Section {Text("校园 Wi-Fi 若隔离设备，请使用自己的热点或蓝牙。iOS 返回后台后暂停通信，回到前台后继续同步。").font(.footnote).foregroundStyle(.secondary)}
            }
            .navigationTitle("连接同学")
            .sheet(isPresented:$showImport) {
                NavigationStack {Form {Section("好友或群邀请 JSON") {TextEditor(text:$contactJSON).frame(minHeight:180).textInputAutocapitalization(.never).autocorrectionDisabled()}}
                    .navigationTitle("添加好友").toolbar {
                        ToolbarItem(placement:.cancellationAction) {Button("取消"){showImport=false}}
                        ToolbarItem(placement:.confirmationAction) {Button("添加"){mesh.addContact(contactJSON);if mesh.error.isEmpty {showImport=false;contactJSON=""}}.disabled(contactJSON.isEmpty)}
                    }
                }
            }
        }
    }
}

struct IdentityView: View {
    @EnvironmentObject private var mesh:MeshStore
    @State private var editingNickname=false
    @State private var nickname=""
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    HStack {Image(systemName:"person.crop.circle.fill").font(.system(size:42)).foregroundStyle(.tint);VStack(alignment:.leading) {Text(mesh.identity["nickname"] as? String ?? "正在读取身份").font(.title3.bold());Text("本机公钥身份").font(.caption).foregroundStyle(.secondary)};Spacer();Button("编辑"){nickname=mesh.identity["nickname"] as? String ?? "";editingNickname=true}}
                    if !mesh.identity.isEmpty,let image=qr(mesh.publicJSON) {Image(uiImage:image).interpolation(.none).resizable().scaledToFit().frame(maxWidth:280).padding(16).background(.white,in:RoundedRectangle(cornerRadius:16)).frame(maxWidth:.infinity).accessibilityLabel("我的好友公钥二维码")}
                    Text("让好友扫描，并当面核对公钥。").font(.caption).foregroundStyle(.secondary)
                    ShareLink(item:mesh.publicJSON) {Label("分享公钥",systemImage:"square.and.arrow.up")}.disabled(mesh.identity.isEmpty)
                    LabeledContent("本机设备 ID") {Text(mesh.deviceID).font(.caption.monospaced()).textSelection(.enabled).accessibilityIdentifier("deviceID")}
                    Text("设备 ID 和好友保存在本机，修改昵称不会改变 ID。卸载或清除数据后会生成新身份。").font(.caption).foregroundStyle(.secondary)
                    DisclosureGroup("查看签名公钥") {Text(mesh.identity["sign"] as? String ?? "").font(.caption.monospaced()).textSelection(.enabled)}
                }
                Section("中继与电量") {
                    Toggle("参与消息中继",isOn:Binding(get:{mesh.state["relay"] as? Bool ?? true},set:{mesh.call(["op":"relay","enabled":$0]);mesh.refresh()})).disabled(!mesh.isRunning)
                    Text("中继只传递密文。关闭后仍能发送和接收本人的消息。").font(.caption).foregroundStyle(.secondary)
                    if mesh.isRunning {Button("暂停通信") {mesh.stop(userInitiated:true)}} else {Button("恢复通信") {mesh.start(userInitiated:true)}}
                }
                Section {Text("iOS 返回后台后暂停中继，回到前台继续同步。群成员变更需要通过 Mesh 送达后才会生效。").font(.footnote).foregroundStyle(.secondary)}
                Section {LabeledContent("测试版本",value:releaseName)}
            }
            .navigationTitle("我的身份")
            .alert("修改本机昵称",isPresented:$editingNickname) {
                TextField("昵称",text:$nickname)
                Button("保存") {if mesh.call(["op":"profile","nickname":nickname]) != nil {mesh.nickname=nickname;UserDefaults.standard.set(nickname,forKey:"nickname");mesh.refresh()}}
                Button("取消",role:.cancel) {}
            }
        }
    }
    private var releaseName:String {
        let info=Bundle.main.infoDictionary ?? [:]
        let channel=info["CMReleaseChannel"] as? String ?? "alpha"
        let version=info["CFBundleShortVersionString"] as? String ?? ""
        return "\(channel)-\(version)"
    }
    private func qr(_ value:String)->UIImage? {
        let filter=CIFilter.qrCodeGenerator();filter.message=Data(value.utf8)
        guard let output=filter.outputImage?.transformed(by:CGAffineTransform(scaleX:8,y:8)),let cg=CIContext().createCGImage(output,from:output.extent) else{return nil}
        return UIImage(cgImage:cg)
    }
}

struct CustomizationView: View {
    @EnvironmentObject private var mesh:MeshStore
    @EnvironmentObject private var appearance:AppearanceSettings
    @State private var editingPhrases=false
    @State private var phrases=""
    @State private var groupEditor:MeshGroup?
    var body: some View {
        NavigationStack {
            Form {
                Section {VStack(alignment:.leading,spacing:8) {Text("让界面更像你").font(.title2.bold());Text("偏好只保存在本机，不改变公钥和通信方式。").font(.subheadline).foregroundStyle(.secondary)}}
                Section("外观") {
                    Picker("深浅色",selection:$appearance.theme) {Text("跟随系统").tag("system");Text("浅色").tag("light");Text("深色").tag("dark")}
                    Picker("主题色",selection:$appearance.accentName) {Text("松绿").tag("teal");Text("晴蓝").tag("blue");Text("鸢紫").tag("violet");Text("暖杏").tag("amber")}
                    Picker("聊天字号",selection:$appearance.fontSize) {ForEach([14.0,16.0,18.0,20.0],id:\.self) {size in Text("\(Int(size))").tag(size)}}
                    Toggle("紧凑气泡",isOn:$appearance.compact)
                    Toggle("显示消息时间",isOn:$appearance.timestamps)
                    Text("下课后，图书馆见。").font(.system(size:appearance.fontSize)).padding(appearance.compact ? 8 : 16).frame(maxWidth:.infinity,alignment:.leading).background(appearance.accent.opacity(0.14),in:RoundedRectangle(cornerRadius:16))
                }
                Section {
                    Toggle("通透控件",isOn:$appearance.glassControls)
                    LiquidGlassPreview().listRowInsets(EdgeInsets(top:8,leading:12,bottom:8,trailing:12))
                } header:{Text("Liquid Glass")} footer:{
                    Text("用于聊天输入栏和悬浮按钮；关闭后使用实色，并始终遵循系统辅助功能设置。导航栏和标签栏跟随系统外观。" + (LiquidGlassSupport.isNativeAvailable ? "" : " 此系统使用兼容半透明材质。"))
                }
                Section("少点几次，更快表达") {
                    Button("编辑快捷短语") {phrases=appearance.phrases.joined(separator:"\n");editingPhrases=true}
                    Button("创建群聊") {groupEditor=MeshGroup.empty}
                    Text("草稿按会话保存。快捷短语只填入输入框，由你确认发送；向右轻扫会话可设置常用。").font(.caption).foregroundStyle(.secondary)
                    ForEach(mesh.groups) {group in NavigationLink {GroupDetailsView(id:group.id)} label:{LabeledContent(group.name,value:"\(group.members.count) 位成员")}}
                }
                Section {
                    Button("恢复默认外观") {appearance.restoreAppearance()}
                } footer:{Text("恢复主题、字号、密度、时间显示和通透控件；保留身份、聊天记录、草稿、短语和群聊。")}
            }
            .navigationTitle("按你的习惯")
            .sheet(item:$groupEditor) {GroupEditor(preset:$0)}
            .sheet(isPresented:$editingPhrases) {
                NavigationStack {Form {Section {TextEditor(text:$phrases).frame(minHeight:200)} header:{Text("每行一条，最多 8 条")} footer:{Text("每条最多 80 个字符。")}}
                    .navigationTitle("快捷短语").toolbar {
                        ToolbarItem(placement:.cancellationAction) {Button("取消"){editingPhrases=false}}
                        ToolbarItem(placement:.confirmationAction) {Button("保存") {appearance.phrases=Array(phrases.components(separatedBy:.newlines).map{$0.trimmingCharacters(in:.whitespacesAndNewlines)}.filter{!$0.isEmpty}.prefix(8)).map{String($0.prefix(80))};editingPhrases=false}}
                    }
                }
            }
        }
    }
}

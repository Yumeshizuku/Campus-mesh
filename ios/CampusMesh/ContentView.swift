import SwiftUI
import CoreImage.CIFilterBuiltins
import UIKit

struct ContentView: View {
    @EnvironmentObject private var mesh: MeshStore
    @EnvironmentObject private var appearance: AppearanceSettings
    @State private var scanning=false
    @AppStorage("ui.tab") private var selectedTab=0
    var body: some View {
        TabView(selection:$selectedTab) {
            ConversationsView().tag(0).tabItem {Label("聊天",systemImage:"bubble.left.and.bubble.right.fill")}
            NearbyView(scanning:$scanning).tag(1).tabItem {Label("附近",systemImage:"antenna.radiowaves.left.and.right")}
            IdentityView().tag(2).tabItem {Label("身份",systemImage:"person.crop.circle")}
            CustomizationView().tag(3).tabItem {Label("自定义",systemImage:"slider.horizontal.3")}
        }
        .sheet(isPresented:$scanning) {QRScanner { value in mesh.addContact(value);scanning=false }}
        .alert("Campus Mesh",isPresented:Binding(get:{!mesh.error.isEmpty},set:{if !$0 {mesh.error=""}})) {
            Button("好") {mesh.error=""}
        } message:{Text(mesh.error)}
    }
}

struct ConversationsView: View {
    @EnvironmentObject private var mesh:MeshStore
    @EnvironmentObject private var appearance:AppearanceSettings
    @State private var query=""
    @State private var groupEditor:MeshGroup?
    @State private var scanning=false
    private func title(_ key:String)->String {
        if key.hasPrefix("g:"){return mesh.groupName(String(key.dropFirst(2)))}
        return mesh.displayName(String(key.dropFirst(2)))
    }
    var body: some View {
        let snapshot=makeConversationRows(history:mesh.history,contacts:mesh.contacts,groups:mesh.groups,favorites:appearance.favorites,readIDs:appearance.readIDs,query:query)
        NavigationStack {
            List {
                Section {
                    Label(mesh.isRunning ? "本机已就绪 · \(mesh.contacts.count) 位好友" : "通信已暂停",systemImage:mesh.isRunning ? "dot.radiowaves.left.and.right" : "pause.circle")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if mesh.pendingGroups>0 {Section {Text("\(mesh.pendingGroups) 个入群申请正在等待与群主同步；邀请 10 分钟内有效。").font(.caption).foregroundStyle(.secondary)}}
                if snapshot.isEmpty {
                    Section {
                        VStack(alignment:.leading,spacing:12) {
                            Image(systemName:"person.2.wave.2").font(.largeTitle).foregroundStyle(appearance.accent)
                            Text(query.isEmpty ? "校园小网，从一位同学开始" : "没有匹配的会话").font(.title3.bold())
                            Text("互扫公钥二维码后开始聊天。没有网络时，消息可以等待下一次接触。").font(.subheadline).foregroundStyle(.secondary)
                            Button("扫码添加同学") {scanning=true}.buttonStyle(.borderedProminent)
                        }.padding(.vertical,16)
                    }
                }
                Section {
                    ForEach(snapshot) {row in
                        let key=row.id
                        NavigationLink {ChatRoom(thread:key)} label:{
                            HStack(spacing:12) {
                                TelegramAvatar(name:title(key),key:key)
                                VStack(alignment:.leading,spacing:5) {
                                    HStack {Text(title(key)).font(.headline);if appearance.favorites.contains(key){Image(systemName:"pin.fill").font(.caption).foregroundStyle(appearance.accent)}}
                                    Text(appearance.draft(key).isEmpty ? row.last?.text ?? "打声招呼吧" : "草稿 · \(appearance.draft(key))").font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
                                }
                                Spacer()
                                VStack(alignment:.trailing,spacing:8) {
                                    if let last=row.last{Text(last.time,style:.time).font(.caption2).foregroundStyle(.secondary)}
                                    let unread=row.unread
                                    if unread>0 {Text("\(unread)").font(.caption.bold()).foregroundStyle(.white).padding(.horizontal,7).padding(.vertical,3).background(appearance.accent,in:Capsule()).accessibilityLabel("\(unread) 条本机未读")}
                                }
                            }.padding(.vertical,appearance.compact ? 2 : 7)
                        }
                        .swipeActions(edge:.leading) {Button {if !appearance.favorites.insert(key).inserted {appearance.favorites.remove(key)}} label:{Label("常用",systemImage:"pin")}.tint(appearance.accent)}
                    }
                }
            }
            .listStyle(.plain)
            .navigationTitle("Campus Mesh").navigationBarTitleDisplayMode(.inline).searchable(text:$query,prompt:"搜索同学或最近消息")
            .toolbar {ToolbarItem(placement:.navigationBarTrailing) {Menu {
                Button {scanning=true} label:{Label("扫码添加好友",systemImage:"qrcode.viewfinder")}
                Button {groupEditor=MeshGroup.empty} label:{Label("创建群聊",systemImage:"person.3")}
            } label:{Image(systemName:"square.and.pencil")}.accessibilityLabel("新建会话")}}
            .sheet(item:$groupEditor) {GroupEditor(preset:$0)}
            .sheet(isPresented:$scanning) {QRScanner {value in mesh.addContact(value);scanning=false}}
        }
    }
}

struct ChatRoom: View {
    let thread:String
    @EnvironmentObject private var mesh:MeshStore
    @EnvironmentObject private var appearance:AppearanceSettings
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var text=""
    @State private var query=""
    @State private var showLatest=false
    @State private var feedback=""
    private var isGroup:Bool {thread.hasPrefix("g:")}
    private var target:String {String(thread.dropFirst(2))}
    private var title:String {isGroup ? mesh.groupName(target) : mesh.displayName(target)}
    private var preset:MeshGroup? {mesh.groups.first(where:{$0.id==target})}
    @State private var showDetails=false
    private var messages:[DisplayMessage] {Array(mesh.history.map(DisplayMessage.init).filter{$0.thread==thread && (query.isEmpty || $0.text.localizedCaseInsensitiveContains(query))}.suffix(200))}
    var body: some View {
        let visible=messages
        ScrollViewReader {proxy in
            ScrollView {
                LazyVStack(spacing:appearance.compact ? 7 : 14) {
                    if messages.isEmpty {
                        VStack(spacing:10) {Image(systemName:"bubble.left").font(.largeTitle);Text(query.isEmpty ? "还没有消息，打声招呼吧" : "没有匹配的消息");Text("消息会在重新接触后继续传递。").font(.caption)}.foregroundStyle(.secondary).padding(32)
                    }
                    ForEach(Array(visible.enumerated()),id:\.element.id) {index,message in
                        if index==0 || !Calendar.current.isDate(visible[index-1].time,inSameDayAs:message.time) {Text(message.time,format:.dateTime.month(.abbreviated).day()).font(.caption).foregroundStyle(.secondary).padding(.horizontal,12).padding(.vertical,5).background(.ultraThinMaterial,in:Capsule()).padding(.vertical,7)}
                        MessageBubble(message:message,showSender:isGroup)
                    }
                    Color.clear.frame(height:1).id("latest")
                }.padding()
            }
            .scrollDismissesKeyboard(.interactively)
            .background {TelegramWallpaper().ignoresSafeArea()}
            .overlay(alignment:.bottomTrailing) {
                if showLatest {
                    Button {scrollToLatest(proxy);showLatest=false} label:{
                        Label("最新消息",systemImage:"arrow.down").font(.subheadline.weight(.semibold))
                            .padding(.horizontal,16).frame(minHeight:44)
                            .campusGlass(in:Capsule(),interactive:true)
                    }.buttonStyle(.plain).padding()
                }
            }
            .onAppear {text=appearance.draft(thread);appearance.markRead(thread,messages:mesh.history.map(DisplayMessage.init));proxy.scrollTo("latest",anchor:.bottom)}
            .onChange(of:messages.last?.id) {_ in showLatest=true;appearance.markRead(thread,messages:mesh.history.map(DisplayMessage.init))}
            .safeAreaInset(edge:.bottom,spacing:0) {composer {send();scrollToLatest(proxy);showLatest=false}}
        }
        .navigationTitle("").navigationBarTitleDisplayMode(.inline)
        .toolbar(.hidden,for:.tabBar)
        .toolbar {ToolbarItem(placement:.principal) {
            HStack(spacing:9) {TelegramAvatar(name:title,key:thread,size:34);VStack(alignment:.leading,spacing:2) {Text(title).font(.headline);Text(mesh.isRunning ? "离线消息 · 自动同步" : "通信已暂停").font(.caption2).foregroundStyle(.secondary)}}
        }}
        .searchable(text:$query,prompt:"搜索当前会话")
        .toolbar {ToolbarItem(placement:.navigationBarTrailing) {Menu {
            Button {if !appearance.favorites.insert(thread).inserted {appearance.favorites.remove(thread)}} label:{Label(appearance.favorites.contains(thread) ? "取消常用" : "设为常用",systemImage:"pin")}
            Button(isGroup ? "群聊信息" : "好友信息与备注") {showDetails=true}
        } label:{Image(systemName:"ellipsis.circle")}.accessibilityLabel("会话操作")}}
        .sheet(isPresented:$showDetails) {
            if isGroup {GroupDetailsView(id:target)} else {NavigationStack {FriendDetailsView(id:target)}}
        }
        .onChange(of:text) {appearance.saveDraft($0,for:thread)}
    }
    private func composer(sendAction:@escaping ()->Void)->some View {
        VStack(alignment:.leading,spacing:8) {
            if isGroup || !feedback.isEmpty {
                VStack(alignment:.leading,spacing:4) {
                    if isGroup {Text(preset?.joined == true && preset?.dissolved == false ? "\(preset?.members.count ?? 0) 位成员 · Mesh 加密群聊" : "已退出或解散 · 保留聊天记录").font(.caption2)}
                    if !feedback.isEmpty {Text(feedback).font(.caption).accessibilityAddTraits(.updatesFrequently)}
                }.foregroundStyle(.secondary).padding(.horizontal,12).padding(.vertical,8)
                    .background(Color(uiColor:.secondarySystemGroupedBackground),in:RoundedRectangle(cornerRadius:16))
            }
            GlassControlGroup {
                HStack(alignment:.bottom,spacing:8) {
                    Menu {ForEach(Array(appearance.phrases.enumerated()),id:\.offset){_,phrase in Button(phrase){text=phrase}}} label:{
                        Image(systemName:"text.bubble").font(.title3).frame(width:44,height:44)
                            .campusGlass(in:Circle(),interactive:true)
                    }.buttonStyle(.plain).accessibilityLabel("快捷短语").disabled(appearance.phrases.isEmpty)
                    TextField("消息",text:$text,axis:.vertical).lineLimit(1...5)
                        .font(.body).padding(.horizontal,14).padding(.vertical,10).frame(minHeight:44)
                        .campusGlass(in:RoundedRectangle(cornerRadius:22),interactive:true)
                        .submitLabel(.send).onSubmit(sendAction).accessibilityLabel("消息")
                    GlassSendButton(enabled:!text.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty && mesh.isRunning && (!isGroup || (preset?.joined == true && preset?.dissolved == false)),action:sendAction)
                }
            }
        }.padding(.horizontal,12).padding(.vertical,8)
    }
    private func scrollToLatest(_ proxy:ScrollViewProxy) {
        if reduceMotion {proxy.scrollTo("latest",anchor:.bottom)}
        else {withAnimation(.easeOut(duration:0.2)) {proxy.scrollTo("latest",anchor:.bottom)}}
    }
    private func send() {
        guard !text.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty else{return}
        guard text.utf8.count<=8192 else{feedback="消息过长，请控制在 8192 字节以内。";return}
        let command:[String:Any]=isGroup ? ["op":"group_send","id":target,"text":text] : ["op":"send","to":target,"text":text]
        if mesh.call(command) != nil {text="";appearance.saveDraft("",for:thread);feedback=""}
        else {feedback="消息尚未提交，草稿已保留。"}
        mesh.refresh()
    }
}

struct MessageBubble: View {
    let message:DisplayMessage
    let showSender:Bool
    @EnvironmentObject private var mesh:MeshStore
    @EnvironmentObject private var appearance:AppearanceSettings
    @ScaledMetric(relativeTo:.body) private var baseSize=16.0
    @Environment(\.colorScheme) private var colorScheme
    private var bubble:Color {
        if message.incoming {return colorScheme == .dark ? Color(red:0.095,green:0.145,blue:0.20) : .white}
        if appearance.accentName=="blue" {return colorScheme == .dark ? Color(red:0.17,green:0.32,blue:0.47) : Color(red:0.88,green:1,blue:0.78)}
        return appearance.accent.opacity(colorScheme == .dark ? 0.28 : 0.14)
    }
    var body: some View {
        HStack {
            if !message.incoming {Spacer(minLength:40)}
            VStack(alignment:.leading,spacing:5) {
                if showSender && message.incoming {Text(mesh.displayName(message.peer,fallback:message.senderName)).font(.caption).foregroundStyle(.secondary)}
                Text(message.text).font(.system(size:baseSize * appearance.fontSize / 16)).textSelection(.enabled)
                HStack(spacing:5) {
                    if appearance.timestamps {Text(message.time,style:.time)}
                    if !message.incoming {Image(systemName:message.status=="delivered" ? "checkmark" : message.status=="expired" ? "exclamationmark.circle" : "clock")}
                    Text(message.statusLabel)
                }.font(.caption2).foregroundStyle(.secondary)
            }
            .padding(.horizontal,12).padding(.vertical,appearance.compact ? 7 : 10)
            .background {TelegramBubbleShape(incoming:message.incoming).fill(bubble)}
            .contextMenu {Button {UIPasteboard.general.string=message.text} label:{Label("复制消息",systemImage:"doc.on.doc")}}
            if message.incoming {Spacer(minLength:40)}
        }
    }
}

struct GroupEditor: View {
    let preset:MeshGroup
    @EnvironmentObject private var mesh:MeshStore
    @Environment(\.dismiss) private var dismiss
    @State private var name=""
    @State private var members=Set<String>()
    @State private var error=""
    private var available:[String] {Set(mesh.contacts.keys).union(preset.members.keys).subtracting([mesh.deviceID]).sorted()}
    var body: some View {
        NavigationStack {
            Form {
                Section("群聊名称") {TextField("例如：自习小组",text:$name);Text("群主可以管理成员，最多 32 人。也可以先建群，再分享入群二维码。").font(.caption).foregroundStyle(.secondary)}
                Section("选择成员") {ForEach(available,id:\.self) {id in Toggle(mesh.displayName(id,fallback:preset.members[id]?["nickname"] as? String ?? "同学"),isOn:Binding(get:{members.contains(id)},set:{if $0 {members.insert(id)} else {members.remove(id)}}))}}
                if !error.isEmpty {Text(error).foregroundStyle(.red)}
            }
            .navigationTitle(preset.id.isEmpty ? "创建群聊" : "编辑群聊").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement:.cancellationAction) {Button("取消"){dismiss()}}
                ToolbarItem(placement:.confirmationAction) {Button("保存") {
                    let cleaned=name.trimmingCharacters(in:.whitespacesAndNewlines)
                    guard !cleaned.isEmpty,cleaned.utf8.count<=96,members.count<32 else{error="请输入最多 96 字节的名称，群成员总数不能超过 32。";return}
                    var command:[String:Any]=["op":preset.id.isEmpty ? "group_create" : "group_edit","name":cleaned,"members":members.sorted()]
                    if !preset.id.isEmpty {command["id"]=preset.id}
                    if mesh.call(command) != nil {mesh.refresh();dismiss()}
                }}
            }.onAppear {name=preset.name;members=Set(preset.members.keys).subtracting([mesh.deviceID])}
        }
    }
}

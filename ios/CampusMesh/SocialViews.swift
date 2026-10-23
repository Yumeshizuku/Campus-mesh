import SwiftUI
import CoreImage.CIFilterBuiltins
import UIKit

struct FriendDetailsView:View {
    let id:String
    @EnvironmentObject private var mesh:MeshStore
    @State private var remark=""
    var body:some View {
        Form {
            Section("好友") {
                LabeledContent("昵称",value:mesh.contacts[id]?["nickname"] as? String ?? "同学")
                TextField("仅自己可见的备注",text:$remark)
                Button("保存备注") {if mesh.call(["op":"friend_remark","id":id,"remark":remark]) != nil {mesh.refresh()}}
                Text("留空后显示好友昵称。备注只保存在你的设备上。").font(.caption).foregroundStyle(.secondary)
            }
            Section("已记住的设备") {
                Text(id).font(.caption.monospaced()).textSelection(.enabled)
                ShareLink(item:id) {Label("分享设备 ID",systemImage:"square.and.arrow.up")}
                Text("设备 ID 来自签名公钥。重新打开应用或修改昵称后仍会识别为同一设备。").font(.caption).foregroundStyle(.secondary)
            }
        }.navigationTitle(mesh.displayName(id)).onAppear {remark=mesh.contacts[id]?["remark"] as? String ?? ""}
    }
}

struct GroupDetailsView:View {
    let id:String
    @EnvironmentObject private var mesh:MeshStore
    @State private var editing:MeshGroup?
    @State private var invitation=""
    @State private var showInvite=false
    @State private var confirmLeave=false
    private var group:MeshGroup? {mesh.groups.first{$0.id==id}}
    private var owner:Bool {group?.ownerID==mesh.deviceID}
    var body:some View {
        NavigationStack {
            Form {
                if let group=group {
                    Section(group.name) {
                        Text(group.dissolved ? "群聊已解散" : group.joined ? "已加入 · \(group.members.count) 位成员" : "已退出 · 保留聊天记录")
                        Text(id).font(.caption.monospaced()).textSelection(.enabled)
                    }
                    Section("成员") {ForEach(group.members.keys.sorted(),id:\.self) {member in
                        VStack(alignment:.leading,spacing:4) {
                            Text(mesh.displayName(member,fallback:group.members[member]?["nickname"] as? String ?? "同学")+(member==group.ownerID ? " · 群主" : ""))
                            Text(member).font(.caption2.monospaced()).foregroundStyle(.secondary).textSelection(.enabled)
                        }
                    }}
                    if group.joined && !group.dissolved {
                        Section {
                            if !owner {Button("同步群信息") {if mesh.call(["op":"group_sync","id":id]) != nil {mesh.refresh()}}}
                            if owner {
                                Button("修改群名和成员") {editing=group}
                                Button("生成入群二维码") {
                                    if let value=mesh.call(["op":"group_invite","id":id]) {invitation=jsonText(value);showInvite=true}
                                }
                            }
                            Button(owner ? "解散群聊" : "退出群聊",role:.destructive) {confirmLeave=true}
                        } footer:{Text("群主管理成员；成员变更在 Mesh 同步后生效。已送达的消息会保留在接收方设备上。")}
                    }
                } else {Text("这是升级前的群发记录，可以在聊天页创建新群聊。")}
            }.navigationTitle("群聊信息").navigationBarTitleDisplayMode(.inline)
            .sheet(item:$editing) {GroupEditor(preset:$0)}
            .sheet(isPresented:$showInvite) {QRCodeSheet(text:invitation)}
            .confirmationDialog(owner ? "确定解散群聊？" : "确定退出群聊？",isPresented:$confirmLeave,titleVisibility:.visible) {
                Button(owner ? "解散" : "退出",role:.destructive) {if mesh.call(["op":"group_leave","id":id]) != nil {mesh.refresh()}}
            } message:{Text("聊天记录会保留，变更将通过 Mesh 同步。")}
        }
    }
}

struct QRCodeSheet:View {
    let text:String
    @Environment(\.dismiss) private var dismiss
    private var image:UIImage? {
        let filter=CIFilter.qrCodeGenerator();filter.message=Data(text.utf8)
        guard let output=filter.outputImage?.transformed(by:CGAffineTransform(scaleX:8,y:8)),let cg=CIContext().createCGImage(output,from:output.extent) else{return nil}
        return UIImage(cgImage:cg)
    }
    var body:some View {
        NavigationStack {
            Form {
                Section {if let image=image {Image(uiImage:image).interpolation(.none).resizable().scaledToFit().padding().background(.white).accessibilityLabel("群邀请二维码")}}
                Section {Text("请当面分享。二维码 10 分钟内有效，仅可使用一次；重新生成会撤销上一个邀请。扫码后需在有效期内与群主通过 Mesh 完成同步。").font(.callout);ShareLink(item:text) {Label("分享邀请",systemImage:"square.and.arrow.up")}}
            }.navigationTitle("邀请入群").toolbar {ToolbarItem(placement:.confirmationAction) {Button("完成"){dismiss()}}}
        }
    }
}

struct MeshSettingsView:View {
    @EnvironmentObject private var mesh:MeshStore
    @State private var values=[String:String]()
    @State private var saved=false
    private let fields:[(String,String,String)]=[
        ("hop_limit","最大跳数","1–8"),("message_ttl_secs","消息有效期（秒）","60–86400"),
        ("sync_interval_secs","同步间隔（秒）","1–60"),("batch_size","TCP 每批封装数","1–24"),
        ("max_peers","TCP 邻居上限","1–300"),("cache_capacity","待传封装上限","64–4096")]
    var body:some View {
        Form {
            Section {
                ForEach(fields,id:\.0) {key,title,range in
                    HStack {Text(title);Spacer();TextField(range,text:Binding(get:{values[key] ?? ""},set:{values[key]=$0;saved=false})).keyboardType(.numberPad).multilineTextAlignment(.trailing).frame(width:120).accessibilityLabel(title)}
                }
            } footer:{Text("跳数和有效期用于新消息；同步间隔同时影响 TCP 和蓝牙。批量与邻居上限用于 TCP，蓝牙仍每轮发送一个封装，并保留各 4 条连接的限制。间隔越短、跳数越高，耗电与网络开销通常越大。")}
            Section {
                Button("保存 Mesh 参数") {
                    var settings=[String:Int]()
                    for (key,_,_) in fields {guard let number=Int(values[key] ?? "") else{mesh.error="请输入有效的整数";return};settings[key]=number}
                    if mesh.call(["op":"settings","settings":settings]) != nil {mesh.refresh();saved=true}
                }.disabled(!mesh.isRunning)
                Button("恢复默认参数") {load(["hop_limit":8,"message_ttl_secs":86400,"sync_interval_secs":5,"batch_size":24,"max_peers":300,"cache_capacity":4096]);saved=false}
                if saved {Text("参数已保存到本机").foregroundStyle(.secondary)}
            }
        }.navigationTitle("Mesh 详细参数").onAppear {load(mesh.settings)}
        .toolbar {ToolbarItemGroup(placement:.keyboard) {Spacer();Button("收起键盘") {UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder),to:nil,from:nil,for:nil)}}}
    }
    private func load(_ settings:[String:Any]) {for (key,_,_) in fields {values[key]=(settings[key] as? NSNumber)?.stringValue ?? ""}}
}

use campus_mesh::*;
use std::collections::BTreeMap;
const NOW: u64 = 100_000;
fn friend(a: &mut Engine, b: &Engine) {
    a.scan_card(
        &serde_json::to_string(&b.contact_card().unwrap()).unwrap(),
        NOW,
    )
    .unwrap();
}
fn exchange(a: &mut Engine, b: &mut Engine, now: u64) {
    for frame in a.outgoing(now) {
        let _ = b.receive(frame, now);
    }
    for frame in b.outgoing(now) {
        let _ = a.receive(frame, now);
    }
}
fn setup() -> (Engine, Engine, Engine, String) {
    let mut a = Engine::new("群主");
    let mut b = Engine::new("乙");
    let mut c = Engine::new("丙");
    friend(&mut a, &b);
    friend(&mut a, &c);
    friend(&mut b, &a);
    friend(&mut c, &a);
    let id = a
        .create_group("自习小组", &[b.identity.id(), c.identity.id()], None, NOW)
        .unwrap();
    exchange(&mut a, &mut b, NOW);
    exchange(&mut a, &mut c, NOW);
    (a, b, c, id)
}
#[test]
fn signed_qr_identity_remarks_and_legacy_upgrade() {
    let a = Engine::new("小明");
    let mut b = Engine::new("小红");
    friend(&mut b, &a);
    b.set_remark(&a.identity.id(), "同桌").unwrap();
    friend(&mut b, &a);
    assert_eq!(b.friends[&a.identity.id()].remark, "同桌");
    let mut bad = a.contact_card().unwrap();
    bad.contact.nickname = "假冒".into();
    assert!(b
        .scan_card(&serde_json::to_string(&bad).unwrap(), NOW)
        .is_err());
    bad = a.contact_card().unwrap();
    bad.device_id = b.identity.id();
    assert!(bad.validate().is_err());
    let path = std::env::temp_dir().join(format!("mesh-social-{}.json", b.identity.id()));
    b.save(&path).unwrap();
    let restored = Engine::load(&path).unwrap();
    assert_eq!(restored.identity.id(), b.identity.id());
    assert_eq!(restored.friends[&a.identity.id()].remark, "同桌");
    let mut old = serde_json::to_value(&b).unwrap();
    for key in [
        "friends",
        "settings",
        "groups",
        "group_invites",
        "pending_groups",
        "group_outbox",
        "legacy_groups",
    ] {
        old.as_object_mut().unwrap().remove(key);
    }
    std::fs::write(&path, old.to_string()).unwrap();
    let migrated = Engine::load(&path).unwrap();
    assert_eq!(migrated.identity, b.identity);
    assert_eq!(migrated.settings, MeshSettings::default());
    std::fs::remove_file(path).unwrap();
}
#[test]
fn configured_mesh_hops_ttl_capacity_and_restart() {
    let mut a = Engine::new("甲");
    let mut b = Engine::new("乙");
    let mut relay = Engine::new("中继");
    friend(&mut a, &b);
    friend(&mut b, &a);
    let settings = MeshSettings {
        hop_limit: 2,
        message_ttl_secs: 60,
        sync_interval_secs: 1,
        batch_size: 3,
        max_peers: 2,
        cache_capacity: 64,
    };
    a.update_settings(settings.clone()).unwrap();
    let id = a.send(&b.identity.id(), "跨两跳", None, NOW).unwrap();
    let frame = a.outgoing(NOW).pop().unwrap();
    assert_eq!(frame.envelope.hop_limit, 2);
    assert_eq!(frame.envelope.expires, NOW + 60);
    assert!(relay.receive(frame, NOW).unwrap());
    let relayed = relay.outgoing(NOW).pop().unwrap();
    assert!(b.receive(relayed, NOW).unwrap());
    assert_eq!(b.history[0].text, "跨两跳");
    assert!(!relay.outgoing(NOW + 60).iter().any(|f| f.envelope.id == id));
    for _ in 1..64 {
        a.send(&b.identity.id(), "缓存", None, NOW).unwrap();
    }
    assert!(a.send(&b.identity.id(), "已满", None, NOW).is_err());
    assert!(a
        .update_settings(MeshSettings {
            hop_limit: 9,
            ..settings.clone()
        })
        .is_err());
    assert_eq!(a.settings, settings);
    let path = std::env::temp_dir().join(format!("mesh-settings-{}.json", a.identity.id()));
    a.save(&path).unwrap();
    assert_eq!(Engine::load(&path).unwrap().settings, settings);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn group_messages_cross_relays_between_members_who_are_not_friends() {
    let (mut a, mut b, mut c, id) = setup();
    let mut relay = Engine::new("路过");
    assert!(!b.contacts.contains_key(&c.identity.id()));
    assert_eq!(b.groups[&id].definition.members.len(), 3);
    let logical = b.send_group(&id, "图书馆见", NOW + 1).unwrap();
    for _ in 0..4 {
        exchange(&mut b, &mut relay, NOW + 1);
        exchange(&mut relay, &mut c, NOW + 1);
        exchange(&mut relay, &mut a, NOW + 1);
    }
    exchange(&mut relay, &mut b, NOW + 1);
    let messages: Vec<_> = b.history.iter().filter(|m| m.id == logical).collect();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].status, "delivered");
    assert_eq!(messages[0].delivery.len(), 2);
    assert_eq!(c.history.iter().filter(|m| m.id == logical).count(), 1);
    assert!(relay.history.is_empty());
    assert!(!serde_json::to_string(&b.outgoing(NOW + 1))
        .unwrap()
        .contains("图书馆见"));
}
#[test]
fn group_owner_permissions_membership_removal_leave_and_dissolve() {
    let (mut a, mut b, mut c, id) = setup();
    assert!(b.edit_group(&id, "越权", &[], NOW).is_err());
    a.edit_group(&id, "复习小组", &[b.identity.id()], NOW + 1)
        .unwrap();
    exchange(&mut a, &mut b, NOW + 1);
    exchange(&mut a, &mut c, NOW + 1);
    assert_eq!(b.groups[&id].definition.name, "复习小组");
    assert!(!c.groups[&id].joined);
    assert!(c.send_group(&id, "退出后", NOW + 2).is_err());
    b.leave_group(&id, NOW + 2).unwrap();
    exchange(&mut b, &mut a, NOW + 2);
    exchange(&mut a, &mut b, NOW + 2);
    assert!(!b.groups[&id].joined);
    assert_eq!(a.groups[&id].definition.members.len(), 1);
    a.leave_group(&id, NOW + 3).unwrap();
    assert!(a.groups[&id].definition.dissolved);
    assert!(a.group_invite(&id, NOW + 4).is_err());
}
#[test]
fn group_qr_unknown_member_join_single_use_expiry_and_persistence() {
    let mut owner = Engine::new("群主");
    let mut guest = Engine::new("访客");
    let mut other = Engine::new("他人");
    let id = owner.create_group("扫码入群", &[], None, NOW).unwrap();
    let invite = owner.group_invite(&id, NOW).unwrap();
    let path = std::env::temp_dir().join(format!("mesh-invite-{}.json", owner.identity.id()));
    owner.save(&path).unwrap();
    owner = Engine::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let raw = serde_json::to_string(&invite).unwrap();
    guest.scan_card(&raw, NOW + 1).unwrap();
    for _ in 0..3 {
        exchange(&mut guest, &mut owner, NOW + 1);
    }
    assert!(guest.groups[&id].joined);
    assert_eq!(owner.groups[&id].definition.members.len(), 2);
    assert!(guest.contacts.is_empty());
    other.scan_card(&raw, NOW + 2).unwrap();
    exchange(&mut other, &mut owner, NOW + 2);
    assert_eq!(owner.groups[&id].definition.members.len(), 2);
    assert!(other.groups.is_empty());
    assert!(invite.validate(NOW + 600).is_err());
    let mut tampered = invite;
    tampered.group_name = "冒名".into();
    assert!(tampered.validate(NOW).is_err());
    guest.send_group(&id, "已经进群", NOW + 2).unwrap();
    exchange(&mut guest, &mut owner, NOW + 2);
    assert_eq!(owner.history.last().unwrap().text, "已经进群");
}
#[test]
fn group_send_is_atomic_when_cache_has_insufficient_room() {
    let (mut a, b, c, id) = setup();
    a.update_settings(MeshSettings {
        cache_capacity: 64,
        ..MeshSettings::default()
    })
    .unwrap();
    while a.outgoing(NOW).len() < 63 {
        a.send(&b.identity.id(), "占位", None, NOW).unwrap();
    }
    let before = serde_json::to_value(&a).unwrap();
    assert!(a.send_group(&id, "不能发一半", NOW).is_err());
    assert_eq!(serde_json::to_value(&a).unwrap(), before);
    assert!(a
        .edit_group(&id, "不能改一半", &[b.identity.id(), c.identity.id()], NOW)
        .is_err());
    assert_eq!(serde_json::to_value(&a).unwrap(), before);
}
#[test]
fn legacy_presets_migrate_once_preserving_history_and_identity() {
    let mut a = Engine::new("甲");
    let b = Engine::new("乙");
    friend(&mut a, &b);
    a.send(&b.identity.id(), "旧群发", Some("旧组合".into()), NOW)
        .unwrap();
    let original = a.identity.clone();
    let id = a
        .create_group("旧组合", &[b.identity.id()], Some("旧组合"), NOW)
        .unwrap();
    assert_eq!(a.history[0].group.as_deref(), Some(id.as_str()));
    assert_eq!(a.identity, original);
    assert_eq!(
        a.create_group("旧组合", &[], Some("旧组合"), NOW).unwrap(),
        id
    );
    assert_eq!(a.groups.len(), 1);
}
#[test]
fn remote_nickname_changes_do_not_replace_local_remarks() {
    let mut a = Engine::new("甲");
    let mut b = Engine::new("旧昵称");
    friend(&mut a, &b);
    friend(&mut b, &a);
    a.set_remark(&b.identity.id(), "实验搭档").unwrap();
    let stable = b.identity.id();
    b.identity.nickname = "新昵称".into();
    b.send(&a.identity.id(), "你好", None, NOW).unwrap();
    exchange(&mut b, &mut a, NOW);
    assert_eq!(b.identity.id(), stable);
    assert_eq!(a.contacts[&stable].nickname, "新昵称");
    assert_eq!(a.friends[&stable].remark, "实验搭档");
    assert_eq!(a.history[0].delivery, BTreeMap::new());
}

#[test]
fn group_sync_recovers_a_member_after_updates_have_expired() {
    let (mut a, mut b, _, id) = setup();
    a.edit_group(&id, "离线期间改名", &[b.identity.id()], NOW + 1)
        .unwrap();
    let later = NOW + 86402;
    a.prune(later);
    b.prune(later);
    assert_eq!(b.groups[&id].definition.name, "自习小组");
    b.sync_group(&id, later).unwrap();
    for _ in 0..3 {
        exchange(&mut b, &mut a, later);
    }
    assert_eq!(b.groups[&id].definition.name, "离线期间改名");
    // An old fanout label must never inject a message into a managed group.
    let forged = a
        .send(
            &b.identity.id(),
            "旧标签伪装群消息",
            Some(id.clone()),
            later,
        )
        .unwrap();
    let frame = a
        .outgoing(later)
        .into_iter()
        .find(|f| f.envelope.id == forged)
        .unwrap();
    assert!(b.receive(frame, later).is_err());
    assert!(b.history.is_empty());
}

#[test]
fn group_member_limit_rejects_the_whole_creation() {
    let mut owner = Engine::new("群主");
    let mut members = Vec::new();
    for index in 0..32 {
        let peer = Engine::new(&format!("同学{index}"));
        friend(&mut owner, &peer);
        members.push(peer.identity.id());
    }
    let before = serde_json::to_value(&owner).unwrap();
    assert!(owner.create_group("过大群聊", &members, None, NOW).is_err());
    assert_eq!(serde_json::to_value(&owner).unwrap(), before);
}

#[test]
fn migration_accepts_the_old_longer_group_name_limit() {
    let mut a = Engine::new("甲");
    let b = Engine::new("乙");
    friend(&mut a, &b);
    let old = "组".repeat(42);
    a.send(&b.identity.id(), "旧记录", Some(old.clone()), NOW)
        .unwrap();
    let id = a
        .create_group(&old, &[b.identity.id()], Some(&old), NOW)
        .unwrap();
    assert_eq!(a.groups[&id].definition.name.len(), 96);
    assert_eq!(a.history[0].group.as_deref(), Some(id.as_str()));
}

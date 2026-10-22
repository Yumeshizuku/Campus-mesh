use campus_mesh::network::{Runtime, TcpTransport, Transport};
use campus_mesh::*;
fn nodes(n: usize) -> Vec<Engine> {
    let mut nodes: Vec<_> = (0..n)
        .map(|i| Engine::new(&format!("student-{i}")))
        .collect();
    let a = nodes[0].identity.clone();
    let b = nodes[n - 1].identity.clone();
    nodes[0].add_contact(b).unwrap();
    nodes[n - 1].add_contact(a).unwrap();
    nodes
}
fn forward(nodes: &mut [Engine], a: usize, b: usize, t: u64) {
    for f in nodes[a].outgoing(t) {
        nodes[b].receive(f, t).unwrap();
    }
}
#[test]
fn encrypted_three_hops_and_receipt() {
    let mut ns = nodes(4);
    let dest = ns[3].identity.id();
    let id = ns[0]
        .send(&dest, "secret campus message", None, 100)
        .unwrap();
    let wire = serde_json::to_string(&ns[0].outgoing(100)).unwrap();
    assert!(!wire.contains("secret campus message"));
    for i in 0..3 {
        forward(&mut ns, i, i + 1, 101);
    }
    assert_eq!(ns[3].history[0].text, "secret campus message");
    assert!(ns[1].history.is_empty());
    for i in (1..4).rev() {
        forward(&mut ns, i, i - 1, 102);
    }
    assert_eq!(
        ns[0].history.iter().find(|m| m.id == id).unwrap().status,
        "delivered"
    );
}
#[test]
fn partition_restart_dedup_ttl_and_relay_optout() {
    let mut ns = nodes(4);
    let dest = ns[3].identity.id();
    ns[0].send(&dest, "carry me", None, 100).unwrap();
    forward(&mut ns, 0, 1, 101);
    ns[1] = serde_json::from_slice(&serde_json::to_vec(&ns[1]).unwrap()).unwrap();
    forward(&mut ns, 1, 2, 300);
    forward(&mut ns, 2, 3, 301);
    forward(&mut ns, 2, 3, 302);
    assert_eq!(ns[3].history.len(), 1);
    ns[2].relay = false;
    assert!(ns[2].outgoing(303).is_empty());
    assert!(ns[1].outgoing(86500).is_empty());
}
#[test]
fn signature_tamper_unknown_sender_and_hop_budget() {
    let mut ns = nodes(3);
    let dest = ns[2].identity.id();
    ns[0].send(&dest, "hello", None, 100).unwrap();
    let f = ns[0].outgoing(100).remove(0);
    let mut bad = f.clone();
    bad.envelope.recipient = ns[1].identity.id();
    assert!(ns[1].receive(bad, 101).is_err());
    let mut bad = f.clone();
    bad.envelope.ciphertext.push('0');
    assert!(ns[2].receive(bad, 101).is_err());
    let mut bad = f.clone();
    bad.hops = 8;
    assert!(ns[1].receive(bad, 101).is_err());
    ns[2].contacts.clear();
    assert!(ns[2].receive(f, 101).is_err());
}
#[test]
fn ten_to_three_hundred_virtual_nodes() {
    for count in [10, 30, 100, 300] {
        let mut ns = nodes(count);
        let to = ns[count - 1].identity.id();
        ns[0]
            .send(&to, "virtual wireless contact schedule", None, 100)
            .unwrap(); // star branches plus a three-hop backbone, not a 299-hop chain
        for i in 1..count - 2 {
            forward(&mut ns, 0, i, 101);
        }
        forward(&mut ns, 1, count - 2, 200);
        forward(&mut ns, count - 2, count - 1, 300);
        assert_eq!(ns[count - 1].history.len(), 1);
        forward(&mut ns, count - 1, count - 2, 301);
        forward(&mut ns, count - 2, 1, 302);
        forward(&mut ns, 1, 0, 303);
        assert_eq!(ns[0].history[0].status, "delivered");
        println!("virtual nodes={count}, partition delay=199s, delivery=1/1, hops=3");
    }
}
#[test]
fn real_tcp_delivery_and_ack() {
    let mut ns = nodes(2);
    let path = std::env::temp_dir().join(format!("cm-test-{}.json", ns[1].identity.id()));
    let to = ns[1].identity.id();
    let t = network::now();
    ns[0].send(&to, "real TCP bytes", None, t).unwrap();
    let r = Runtime::start(ns.remove(1), path.clone(), 0, false).unwrap();
    let mut transport = TcpTransport(format!("127.0.0.1:{}", r.port).parse().unwrap());
    transport.exchange(&mut ns[0]).unwrap();
    transport.exchange(&mut ns[0]).unwrap();
    assert_eq!(r.engine.lock().unwrap().history[0].text, "real TCP bytes");
    assert_eq!(ns[0].history[0].status, "delivered");
    drop(r);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn bounded_frame_and_group_fanout() {
    let mut ns = nodes(3);
    let c = ns[1].identity.clone();
    ns[0].add_contact(c.clone()).unwrap();
    let g = Some("study-group".to_string());
    ns[0].send(&c.id(), "group hi", g.clone(), 100).unwrap();
    {
        let to = ns[2].identity.id();
        ns[0].send(&to, "group hi", g, 100)
    }
    .unwrap();
    forward(&mut ns, 0, 2, 101);
    assert_eq!(ns[2].history[0].group.as_deref(), Some("study-group"));
    assert!(ns[0].send(&c.id(), &"x".repeat(8193), None, 100).is_err());
}

#[test]
fn persistence_permissions_expiry_and_batch_fairness() {
    let mut ns = nodes(2);
    let to = ns[1].identity.id();
    for i in 0..30 {
        ns[0].send(&to, &format!("queued {i}"), None, 100).unwrap();
    }
    let mut cursor = 0;
    let mut ids = std::collections::BTreeSet::new();
    for _ in 0..30 {
        ids.insert(
            ns[0].outgoing_batch(100, &mut cursor, 1)[0]
                .envelope
                .id
                .clone(),
        );
    }
    assert_eq!(ids.len(), 30);
    let dir = std::env::temp_dir().join(format!("campus-save-{}", ns[0].identity.id()));
    let path = dir.join("state.json");
    ns[0].save(&path).unwrap();
    let mut restored = Engine::load(&path).unwrap();
    assert_eq!(restored.identity, ns[0].identity);
    assert_eq!(restored.outgoing(101).len(), 30);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    restored.prune(86500);
    assert!(restored.outgoing(86500).is_empty());
    assert!(restored.history.iter().all(|m| m.status == "expired"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn ffi_unicode_lock_and_fail_closed_restore() {
    use campus_mesh::ffi::*;
    use std::ffi::{CStr, CString};
    let path = std::env::temp_dir().join(format!(
        "campus-ffi-{}.json",
        Engine::new("test").identity.id()
    ));
    let p = CString::new(path.to_str().unwrap()).unwrap();
    let name = CString::new("同学🦀").unwrap();
    unsafe {
        let h = cm_open(p.as_ptr(), name.as_ptr(), 0, false);
        assert!(!h.is_null());
        let duplicate = cm_open(p.as_ptr(), name.as_ptr(), 0, false);
        assert!(duplicate.is_null());
        let request = CString::new(r#"{"op":"state"}"#).unwrap();
        let result = cm_call(h, request.as_ptr());
        let response: serde_json::Value =
            serde_json::from_slice(CStr::from_ptr(result).to_bytes()).unwrap();
        assert_eq!(response["data"]["identity"]["nickname"], "同学🦀");
        assert!(response["data"].get("signing").is_none());
        cm_free(result);
        cm_close(h);
        std::fs::write(&path, b"corrupted disk state").unwrap();
        assert!(cm_open(p.as_ptr(), name.as_ptr(), 0, false).is_null());
        assert_eq!(std::fs::read(&path).unwrap(), b"corrupted disk state");
    }
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(format!("{}.lock", path.display())).unwrap();
}

#[test]
fn unicode_nickname_stays_within_wire_identity_limit() {
    let node = Engine::new(&"🦀".repeat(40));
    node.identity.validate().unwrap();
    assert!(node.identity.nickname.len() <= 96);
}

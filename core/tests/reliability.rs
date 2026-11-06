use base64::{engine::general_purpose::STANDARD, Engine as _};
use campus_mesh::*;
use std::io::Write;
fn pair() -> (Engine, Engine) {
    let mut a = Engine::new("甲");
    let mut b = Engine::new("乙");
    a.add_contact(b.identity.clone()).unwrap();
    b.add_contact(a.identity.clone()).unwrap();
    (a, b)
}
#[test]
fn encrypted_compression_long_unicode_roundtrip_and_tamper() {
    let (mut a, mut b) = pair();
    let text = "校园消息🔐".repeat(300);
    a.send(&b.identity.id(), &text, None, 100).unwrap();
    let f = a.outgoing(100).remove(0);
    assert_eq!(f.envelope.version, 2);
    let wire = wire::encode(&f).unwrap();
    assert!(STANDARD.decode(&wire).unwrap().len() < serde_json::to_vec(&f).unwrap().len());
    assert!(!serde_json::to_string(&f).unwrap().contains("校园"));
    let mut bad = wire::decode(&wire).unwrap();
    bad.envelope.nonce = "00".repeat(12);
    assert!(b.receive(bad, 101).is_err());
    b.receive(wire::decode(&wire).unwrap(), 101).unwrap();
    assert_eq!(b.history[0].text, text);
}
#[test]
fn decompression_bomb_and_malformed_wire_are_bounded() {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(&vec![b' '; MAX_FRAME * 10]).unwrap();
    let mut bytes = b"CM2".to_vec();
    bytes.extend(z.finish().unwrap());
    assert!(wire::decode(&STANDARD.encode(bytes))
        .err()
        .unwrap()
        .contains("decompressed"));
    assert!(wire::decode("not base64").is_err());
    assert!(wire::decode(&"A".repeat(MAX_FRAME * 2)).is_err());
}
#[test]
fn retry_backoff_is_per_link_and_allows_redundancy() {
    let (mut a, b) = pair();
    a.send(&b.identity.id(), "important", None, 100).unwrap();
    assert_eq!(a.outgoing_on_link("one", -60, 100, 1).unwrap().len(), 1);
    assert!(a.outgoing_on_link("one", -60, 101, 1).unwrap().is_empty());
    assert_eq!(a.outgoing_on_link("two", -70, 101, 1).unwrap().len(), 1);
    assert_eq!(a.outgoing_on_link("one", -60, 105, 1).unwrap().len(), 1);
    assert!(a.outgoing_on_link("one", -60, 106, 1).unwrap().is_empty());
    assert_eq!(a.outgoing_on_link("one", -60, 112, 1).unwrap().len(), 1);
    assert!(a.outgoing_on_link("one", -60, 113, 1).unwrap().is_empty());
}
#[test]
fn learned_routes_rank_hops_then_rssi_and_heal_after_disconnect() {
    let (mut a, mut b) = pair();
    b.send(&a.identity.id(), "route probe", None, 100).unwrap();
    let f = b.outgoing(100).remove(0);
    a.receive_on_link("weak", -95, f.clone(), 101).unwrap();
    a.receive_on_link("strong", -40, f.clone(), 101).unwrap();
    let mut longer = f.clone();
    longer.hops = 2;
    a.receive_on_link("longer", -20, longer, 101).unwrap();
    let id = a.send(&b.identity.id(), "reply", None, 102).unwrap();
    assert!(!a
        .outgoing_on_link("longer", -20, 102, 24)
        .unwrap()
        .iter()
        .any(|f| f.envelope.id == id));
    assert!(a
        .outgoing_on_link("strong", -40, 102, 24)
        .unwrap()
        .iter()
        .any(|f| f.envelope.id == id));
    a.routing.disconnect("weak");
    a.routing.disconnect("strong");
    assert!(a
        .outgoing_on_link("longer", -20, 103, 24)
        .unwrap()
        .iter()
        .any(|f| f.envelope.id == id));
}
#[test]
fn poisoned_route_rejected_and_no_echo_to_previous_hop() {
    let (mut a, b) = pair();
    a.send(&b.identity.id(), "forward", None, 100).unwrap();
    let f = a.outgoing(100).remove(0);
    let mut relay = Engine::new("relay");
    let mut bad = f.clone();
    bad.envelope.hop_limit = 1;
    assert!(relay.receive_on_link("bad", -40, bad, 101).is_err());
    relay.receive_on_link("source", -50, f, 101).unwrap();
    assert!(relay
        .outgoing_on_link("source", -50, 102, 24)
        .unwrap()
        .is_empty());
    assert_eq!(
        relay.outgoing_on_link("other", -60, 102, 24).unwrap().len(),
        1
    );
}
#[test]
fn lost_route_expands_to_other_links() {
    let (mut a, mut b) = pair();
    b.send(&a.identity.id(), "probe", None, 100).unwrap();
    a.receive_on_link("learned", -50, b.outgoing(100).remove(0), 101)
        .unwrap();
    let id = a.send(&b.identity.id(), "data", None, 102).unwrap();
    assert!(!a
        .outgoing_on_link("fallback", -60, 103, 24)
        .unwrap()
        .iter()
        .any(|f| f.envelope.id == id));
    assert!(a
        .outgoing_on_link("fallback", -60, 117, 24)
        .unwrap()
        .iter()
        .any(|f| f.envelope.id == id));
}
#[test]
fn read_before_ack_survives_restart_and_never_regresses() {
    let (mut a, mut b) = pair();
    let id = a.send(&b.identity.id(), "read me", None, 100).unwrap();
    b.receive(a.outgoing(100).remove(0), 101).unwrap();
    let ack = b.outgoing(101).remove(0);
    b.mark_read(&id, 102).unwrap();
    let count = b.outgoing(102).len();
    b.mark_read(&id, 103).unwrap();
    assert_eq!(b.outgoing(103).len(), count);
    b = serde_json::from_slice(&serde_json::to_vec(&b).unwrap()).unwrap();
    assert_eq!(b.history[0].status, "read");
    for f in b
        .outgoing(104)
        .into_iter()
        .filter(|f| f.envelope.id != ack.envelope.id)
    {
        a.receive(f, 104).unwrap();
    }
    assert_eq!(a.history[0].status, "read");
    a.receive(ack, 105).unwrap();
    assert_eq!(a.history[0].status, "read");
    assert!(a.outgoing(106).is_empty());
}
#[test]
fn offline_relay_restart_roaming_sync_and_read_receipt() {
    let (mut a, mut b) = pair();
    let mut relay = Engine::new("carrier");
    let id = a.send(&b.identity.id(), "offline", None, 100).unwrap();
    relay
        .receive_on_link("old", -60, a.outgoing(100).remove(0), 101)
        .unwrap();
    relay = serde_json::from_slice(&serde_json::to_vec(&relay).unwrap()).unwrap();
    for f in relay.outgoing_on_link("roaming", -65, 1000, 24).unwrap() {
        b.receive(f, 1000).unwrap();
    }
    b.mark_read(&id, 1001).unwrap();
    for f in b.outgoing(1001) {
        relay.receive(f, 1001).unwrap();
    }
    for f in relay.outgoing(1002) {
        a.receive(f, 1002).unwrap();
    }
    assert_eq!(a.history[0].status, "read");
    assert!(relay.history.is_empty());
}
#[test]
fn group_read_is_bound_to_recipient_and_ack_cannot_downgrade_it() {
    let (mut a, mut b) = pair();
    let mut c = Engine::new("丙");
    a.add_contact(c.identity.clone()).unwrap();
    c.add_contact(a.identity.clone()).unwrap();
    let group = a
        .create_group("group", &[b.identity.id(), c.identity.id()], None, 100)
        .unwrap();
    for f in a.outgoing(100) {
        b.receive(f.clone(), 101).unwrap();
        c.receive(f, 101).unwrap();
    }
    let id = a.send_group(&group, "hello", 102).unwrap();
    for f in a.outgoing(102) {
        b.receive(f.clone(), 103).unwrap();
        c.receive(f, 103).unwrap();
    }
    b.mark_read(&id, 104).unwrap();
    c.mark_read(&id, 104).unwrap();
    for f in b.outgoing(105) {
        a.receive(f, 105).unwrap();
    }
    for f in c.outgoing(105) {
        a.receive(f, 105).unwrap();
    }
    let msg = a.history.iter().find(|m| m.id == id).unwrap();
    assert_eq!(msg.status, "read");
    assert!(msg.delivery.values().all(|s| s == "read"));
}

#[test]
fn unavailable_rssi_does_not_become_the_strongest_route() {
    let (mut a, mut b) = pair();
    b.send(&a.identity.id(), "probe", None, 100).unwrap();
    let f = b.outgoing(100).remove(0);
    for (link, rssi) in [("strong", -50), ("medium", -60), ("unknown", 127)] {
        a.receive_on_link(link, rssi, f.clone(), 101).unwrap();
    }
    let id = a.send(&b.identity.id(), "reply", None, 102).unwrap();
    assert!(!a
        .outgoing_on_link("unknown", 127, 102, 24)
        .unwrap()
        .iter()
        .any(|f| f.envelope.id == id));
    assert!(a
        .outgoing_on_link("medium", -60, 102, 24)
        .unwrap()
        .iter()
        .any(|f| f.envelope.id == id));
}

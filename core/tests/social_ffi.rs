use campus_mesh::{ffi::*, Engine};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
unsafe fn call(handle: *mut Handle, value: Value) -> Value {
    let input = CString::new(value.to_string()).unwrap();
    let output = cm_call(handle, input.as_ptr());
    let result = serde_json::from_slice(CStr::from_ptr(output).to_bytes()).unwrap();
    cm_free(output);
    result
}
#[test]
fn mobile_commands_persist_groups_remarks_settings_and_roll_back_failed_saves() {
    let unique = Engine::new("temporary").identity.id();
    let directory = std::env::temp_dir().join(format!("mesh-social-ffi-{unique}"));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("mesh.json");
    let raw = CString::new(path.to_str().unwrap()).unwrap();
    let name = CString::new("手机").unwrap();
    unsafe {
        let h = cm_open(raw.as_ptr(), name.as_ptr(), 0, false);
        assert!(!h.is_null());
        let initial = call(h, json!({"op":"state"}))["data"].clone();
        let device_id = initial["device_id"].clone();
        assert!(initial.get("signing").is_none());
        assert_eq!(initial["contact_card"]["device_id"], device_id);
        let peer = Engine::new("好友");
        let text = serde_json::to_string(&peer.contact_card().unwrap()).unwrap();
        assert_eq!(call(h, json!({"op":"scan","text":text}))["ok"], true);
        assert_eq!(
            call(
                h,
                json!({"op":"friend_remark","id":peer.identity.id(),"remark":"同桌"})
            )["ok"],
            true
        );
        let settings = json!({"hop_limit":3,"message_ttl_secs":600,"sync_interval_secs":2,"batch_size":2,"max_peers":10,"cache_capacity":128});
        assert_eq!(
            call(h, json!({"op":"settings","settings":settings}))["ok"],
            true
        );
        let group = call(
            h,
            json!({"op":"group_create","name":"学习群","members":[peer.identity.id()]}),
        )["data"]["id"]
            .clone();
        assert!(group.is_string());
        assert_eq!(
            call(h, json!({"op":"group_send","id":group,"text":"你好"}))["ok"],
            true
        );
        assert_eq!(
            call(
                h,
                json!({"op":"send","to":peer.identity.id(),"text":"旧入口","group":"学习群"})
            )["ok"],
            false
        );
        assert!(
            call(h, json!({"op":"outgoing","limit":24}))["data"]
                .as_array()
                .unwrap()
                .len()
                <= 2
        );
        // Simulate an unwritable temporary state file. A failed command cannot
        // leave a new group in memory and cause duplicates when the UI retries.
        std::fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert_eq!(
            call(
                h,
                json!({"op":"group_create","name":"保存失败","members":[]})
            )["ok"],
            false
        );
        let state = call(h, json!({"op":"state"}))["data"].clone();
        assert_eq!(state["groups"].as_object().unwrap().len(), 1);
        std::fs::remove_dir(path.with_extension("json.tmp")).unwrap();
        assert_eq!(
            call(h, json!({"op":"profile","nickname":"新昵称"}))["ok"],
            true
        );
        cm_close(h);
        let h = cm_open(raw.as_ptr(), name.as_ptr(), 0, false);
        assert!(!h.is_null());
        let restored = call(h, json!({"op":"state"}))["data"].clone();
        assert_eq!(restored["device_id"], device_id);
        assert_eq!(restored["settings"], settings);
        assert_eq!(
            restored["contacts"][peer.identity.id()]["display_name"],
            "同桌"
        );
        assert_eq!(restored["contacts"][peer.identity.id()]["nickname"], "好友");
        assert_eq!(
            restored["groups"][group.as_str().unwrap()]["name"],
            "学习群"
        );
        assert_eq!(restored["history"].as_array().unwrap().len(), 1);
        cm_close(h);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

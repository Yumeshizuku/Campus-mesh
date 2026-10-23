//! C ABI: opaque runtime ownership, JSON commands, caller must serialize calls.
use crate::*;
use std::ffi::{c_char, CStr, CString};
pub struct Handle {
    runtime: network::Runtime,
    path: PathBuf,
    cursor: usize,
    _lock: fs::File,
}
fn cstring(v: serde_json::Value) -> *mut c_char {
    CString::new(v.to_string()).unwrap().into_raw()
}
fn command(h: &mut Handle, v: serde_json::Value) -> Result<serde_json::Value> {
    let op = v["op"].as_str().ok_or("missing op")?;
    if op == "peer" {
        let address: std::net::SocketAddr = v["address"]
            .as_str()
            .ok_or("missing address")?
            .parse()
            .map_err(err)?;
        h.runtime.add_peer(address)?;
        let mut e = h.runtime.engine.lock().map_err(err)?;
        let text = address.to_string();
        if !e.manual_peers.contains(&text) {
            e.manual_peers.push(text);
        }
        e.save(&h.path)?;
        return Ok(serde_json::json!({"ok":true}));
    }
    let mut e = h.runtime.engine.lock().map_err(err)?;
    e.prune(network::now());
    let result = match op {
        "state" => {
            let contacts: BTreeMap<_, _> = e
                .contacts
                .iter()
                .map(|(id, contact)| {
                    let remark = e.friends.get(id).map(|m| m.remark.as_str()).unwrap_or("");
                    let mut value = serde_json::to_value(contact).unwrap();
                    value["device_id"] = id.clone().into();
                    value["remark"] = remark.into();
                    value["display_name"] = if remark.is_empty() {
                        contact.nickname.clone()
                    } else {
                        remark.into()
                    }
                    .into();
                    (id.clone(), value)
                })
                .collect();
            serde_json::json!({"identity":e.identity,"device_id":e.identity.id(),"contact_card":e.contact_card()?,"contacts":contacts,"groups":e.groups,"pending_groups":e.pending_groups,"settings":e.settings,"history":e.history,"relay":e.relay,"port":h.runtime.port,"peers":h.runtime.peers.lock().map_err(err)?.values().collect::<Vec<_>>() })
        }
        "contact_card" => serde_json::to_value(e.contact_card()?).map_err(err)?,
        "scan" => e.scan_card(v["text"].as_str().ok_or("missing text")?, network::now())?,
        "friend_remark" => {
            e.set_remark(
                v["id"].as_str().ok_or("missing id")?,
                v["remark"].as_str().ok_or("missing remark")?,
            )?;
            serde_json::json!({"ok":true})
        }
        "settings" => {
            let settings = serde_json::from_value(v["settings"].clone()).map_err(err)?;
            e.update_settings(settings)?;
            let limit = e.settings.max_peers;
            let mut peers = h.runtime.peers.lock().map_err(err)?;
            while peers.len() > limit {
                let key = peers
                    .iter()
                    .find(|(_, p)| !p.manual)
                    .map(|(key, _)| *key)
                    .ok_or("neighbor limit")?;
                peers.remove(&key);
            }
            serde_json::json!({"ok":true})
        }
        "group_create" | "group_migrate" => {
            let members: Vec<String> = serde_json::from_value(v["members"].clone()).map_err(err)?;
            let name = v["name"].as_str().ok_or("missing name")?;
            serde_json::json!({"id":e.create_group(name,&members,if op == "group_migrate" {Some(name)} else {None},network::now())?})
        }
        "group_edit" => {
            let members: Vec<String> = serde_json::from_value(v["members"].clone()).map_err(err)?;
            e.edit_group(
                v["id"].as_str().ok_or("missing id")?,
                v["name"].as_str().ok_or("missing name")?,
                &members,
                network::now(),
            )?;
            serde_json::json!({"ok":true})
        }
        "group_invite" => serde_json::to_value(
            e.group_invite(v["id"].as_str().ok_or("missing id")?, network::now())?,
        )
        .map_err(err)?,
        "group_sync" => {
            e.sync_group(v["id"].as_str().ok_or("missing id")?, network::now())?;
            serde_json::json!({"ok":true})
        }
        "group_leave" => {
            e.leave_group(v["id"].as_str().ok_or("missing id")?, network::now())?;
            serde_json::json!({"ok":true})
        }
        "group_send" => {
            serde_json::json!({"id":e.send_group(v["id"].as_str().ok_or("missing id")?,v["text"].as_str().ok_or("missing text")?,network::now())?})
        }
        "profile" => {
            let name = v["nickname"].as_str().ok_or("missing nickname")?.trim();
            if name.is_empty() || name.len() > 96 {
                return Err("nickname must be 1..96 UTF-8 bytes".into());
            }
            e.identity.nickname = name.to_string();
            serde_json::json!({"ok":true})
        }
        "contact" => {
            let c = serde_json::from_value(v["contact"].clone()).map_err(err)?;
            serde_json::json!({"id":e.add_contact(c)?})
        }
        "send" => {
            if !v["group"].is_null() {
                return Err("请使用群聊发送接口".into());
            }
            serde_json::json!({"id":e.send(v["to"].as_str().ok_or("missing to")?,v["text"].as_str().ok_or("missing text")?,v["group"].as_str().map(str::to_owned),network::now())?})
        }
        "relay" => {
            e.relay = v["enabled"].as_bool().ok_or("missing enabled")?;
            serde_json::json!({"ok":true})
        }
        "ingest" => {
            serde_json::json!({"accepted":e.receive(serde_json::from_value(v["frame"].clone()).map_err(err)?,network::now())?})
        }
        "outgoing" => {
            let limit = (v["limit"].as_u64().unwrap_or(24).clamp(1, 24) as usize)
                .min(e.settings.batch_size);
            serde_json::to_value(e.outgoing_batch(network::now(), &mut h.cursor, limit))
                .map_err(err)?
        }
        _ => return Err("unknown operation".into()),
    };
    if !["state", "outgoing", "contact_card"].contains(&op) {
        e.save(&h.path)?;
    }
    Ok(result)
}
/// Returns null on startup error; use a private, writable state path.
/// # Safety
/// path and nickname must point to valid NUL-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn cm_open(
    path: *const c_char,
    nickname: *const c_char,
    port: u16,
    discovery: bool,
) -> *mut Handle {
    let result = std::panic::catch_unwind(|| -> Result<Handle> {
        if path.is_null() || nickname.is_null() {
            return Err("null string".into());
        }
        let path = PathBuf::from(CStr::from_ptr(path).to_str().map_err(err)?);
        let nickname = CStr::from_ptr(nickname).to_str().map_err(err)?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(err)?;
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(format!("{}.lock", path.display()))
            .map_err(err)?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(err)?;
        let e = if path.exists() {
            Engine::load(&path)?
        } else {
            Engine::new(nickname)
        };
        e.save(&path)?;
        Ok(Handle {
            runtime: network::Runtime::start(e, path.clone(), port, discovery)?,
            path,
            cursor: 0,
            _lock: lock,
        })
    });
    match result {
        Ok(Ok(h)) => Box::into_raw(Box::new(h)),
        _ => std::ptr::null_mut(),
    }
}
/// # Safety
/// handle must be live and exclusively accessed; json must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn cm_call(handle: *mut Handle, json: *const c_char) -> *mut c_char {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> Result<serde_json::Value> {
            if handle.is_null() || json.is_null() {
                return Err("null input".into());
            }
            let input = CStr::from_ptr(json).to_bytes();
            if input.len() > MAX_FRAME * 2 {
                return Err("command limit".into());
            }
            command(&mut *handle, serde_json::from_slice(input).map_err(err)?)
        },
    ));
    match result {
        Ok(Ok(v)) => cstring(serde_json::json!({"ok":true,"data":v})),
        Ok(Err(e)) => cstring(serde_json::json!({"ok":false,"error":e})),
        Err(_) => cstring(serde_json::json!({"ok":false,"error":"internal panic"})),
    }
}
/// # Safety
/// handle must be live, with no outstanding cm_call; call once only.
#[no_mangle]
pub unsafe extern "C" fn cm_close(handle: *mut Handle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}
/// # Safety
/// value must be returned by cm_call, and freed once only.
#[no_mangle]
pub unsafe extern "C" fn cm_free(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}

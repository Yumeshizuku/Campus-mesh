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
            serde_json::json!({"identity":e.identity,"contacts":e.contacts,"history":e.history,"relay":e.relay,"port":h.runtime.port,"peers":h.runtime.peers.lock().map_err(err)?.values().collect::<Vec<_>>() })
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
            serde_json::json!({"id":e.send(v["to"].as_str().ok_or("missing to")?,v["text"].as_str().ok_or("missing text")?,v["group"].as_str().map(str::to_owned),network::now())?})
        }
        "relay" => {
            e.relay = v["enabled"].as_bool().ok_or("missing enabled")?;
            serde_json::json!({"ok":true})
        }
        "ingest" => {
            serde_json::json!({"accepted":e.receive(serde_json::from_value(v["frame"].clone()).map_err(err)?,network::now())?})
        }
        "outgoing" => serde_json::to_value(e.outgoing_batch(
            network::now(),
            &mut h.cursor,
            v["limit"].as_u64().unwrap_or(24).clamp(1, 24) as usize,
        ))
        .map_err(err)?,
        _ => return Err("unknown operation".into()),
    };
    if op != "state" && op != "outgoing" {
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

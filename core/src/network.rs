//! Real TCP transport; UDP broadcast is optional discovery, never a trust source.
use crate::*;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub trait Transport {
    fn exchange(&mut self, engine: &mut Engine) -> Result<()>;
}
const MAX_PACKET: usize = 2 * 1024 * 1024;
fn write<T: Serialize>(s: &mut TcpStream, v: &T) -> Result<()> {
    let b = serde_json::to_vec(v).map_err(err)?;
    if b.len() > MAX_PACKET {
        return Err("packet limit".into());
    }
    s.write_all(&(b.len() as u32).to_be_bytes()).map_err(err)?;
    s.write_all(&b).map_err(err)
}
fn read<T: serde::de::DeserializeOwned>(s: &mut TcpStream) -> Result<T> {
    let mut len = [0; 4];
    s.read_exact(&mut len).map_err(err)?;
    let n = u32::from_be_bytes(len) as usize;
    if n > MAX_PACKET {
        return Err("packet limit".into());
    }
    let mut b = vec![0; n];
    s.read_exact(&mut b).map_err(err)?;
    serde_json::from_slice(&b).map_err(err)
}
fn inventory(e: &mut Engine) -> Vec<String> {
    e.prune(now());
    e.seen.keys().cloned().collect()
}
fn missing(e: &mut Engine, ids: &[String]) -> Vec<Frame> {
    e.prune(now());
    let known: std::collections::HashSet<_> = ids.iter().collect();
    e.cache
        .values()
        .filter(|f| {
            f.hops < f.envelope.hop_limit
                && (e.relay || f.envelope.sender.id() == e.identity.id())
                && !known.contains(&f.envelope.id)
        })
        .take(e.settings.batch_size)
        .cloned()
        .collect()
}
fn accept_frames(e: &mut Engine, frames: Vec<Frame>) -> Result<()> {
    if frames.len() > 24 {
        return Err("batch limit".into());
    }
    for f in frames {
        let _ = e.receive(f, now());
    }
    Ok(())
}
fn configure(s: &TcpStream) -> Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(err)?;
    s.set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(err)
}
pub struct TcpTransport(pub SocketAddr);
impl Transport for TcpTransport {
    fn exchange(&mut self, e: &mut Engine) -> Result<()> {
        let mut s = TcpStream::connect_timeout(&self.0, Duration::from_millis(500)).map_err(err)?;
        configure(&s)?;
        write(&mut s, &inventory(e))?;
        let remote: Vec<String> = read(&mut s)?;
        if remote.len() > 8192 {
            return Err("inventory limit".into());
        }
        write(&mut s, &missing(e, &remote))?;
        accept_frames(e, read(&mut s)?)
    }
}
#[derive(Clone, Serialize)]
pub struct Peer {
    pub address: SocketAddr,
    pub last_seen: u64,
    pub connected: bool,
    pub manual: bool,
}
pub struct Runtime {
    pub engine: Arc<Mutex<Engine>>,
    pub peers: Arc<Mutex<BTreeMap<SocketAddr, Peer>>>,
    stop: Arc<AtomicBool>,
    threads: Vec<thread::JoinHandle<()>>,
    pub port: u16,
}
impl Runtime {
    pub fn start(engine: Engine, path: PathBuf, port: u16, discovery: bool) -> Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", port)).map_err(err)?;
        let port = listener.local_addr().map_err(err)?.port();
        listener.set_nonblocking(true).map_err(err)?;
        let engine = Arc::new(Mutex::new(engine));
        let initial: BTreeMap<SocketAddr, Peer> = engine
            .lock()
            .map_err(err)?
            .manual_peers
            .iter()
            .take(300)
            .filter_map(|address| address.parse::<SocketAddr>().ok())
            .map(|address| {
                (
                    address,
                    Peer {
                        address,
                        last_seen: now(),
                        connected: false,
                        manual: true,
                    },
                )
            })
            .collect();
        let peers = Arc::new(Mutex::new(initial));
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();
        {
            let e = engine.clone();
            let stop = stop.clone();
            let path = path.clone();
            threads.push(thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut s, _)) => {
                            let result = (|| -> Result<()> {
                                configure(&s)?;
                                let ids: Vec<String> = read(&mut s)?;
                                if ids.len() > 8192 {
                                    return Err("inventory limit".into());
                                }
                                let (local, frames) = {
                                    let mut e = e.lock().map_err(err)?;
                                    (inventory(&mut e), missing(&mut e, &ids))
                                };
                                write(&mut s, &local)?;
                                let incoming: Vec<Frame> = read(&mut s)?;
                                {
                                    let mut e = e.lock().map_err(err)?;
                                    accept_frames(&mut e, incoming)?;
                                    e.save(&path)?;
                                }
                                write(&mut s, &frames)
                            })();
                            if let Err(error) = result {
                                eprintln!("mesh incoming: {error}");
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(100))
                        }
                        Err(_) => break,
                    }
                }
            }));
        }
        {
            let e = engine.clone();
            let ps = peers.clone();
            let stop = stop.clone();
            threads.push(thread::spawn(move || {
                let udp = if discovery {
                    UdpSocket::bind(("0.0.0.0", 37077)).ok()
                } else {
                    None
                };
                if let Some(u) = &udp {
                    let _ = u.set_broadcast(true);
                    let _ = u.set_nonblocking(true);
                }
                let id = e.lock().unwrap().identity.id();
                let mut last_sync = 0u64;
                while !stop.load(Ordering::Relaxed) {
                    let settings = e.lock().unwrap().settings.clone();
                    let sync_due = now().saturating_sub(last_sync) >= settings.sync_interval_secs;
                    if let Some(u) = &udp {
                        if sync_due {
                            let _ = u.send_to(
                                format!("CM1 {id} {port}").as_bytes(),
                                "255.255.255.255:37077",
                            );
                        }
                        for _ in 0..32 {
                            let mut b = [0; 256];
                            match u.recv_from(&mut b) {
                                Ok((n, addr)) => {
                                    let text = String::from_utf8_lossy(&b[..n]);
                                    let parts: Vec<_> = text.split_whitespace().collect();
                                    if parts.len() == 3 && parts[0] == "CM1" && parts[1] != id {
                                        if let Ok(p) = parts[2].parse::<u16>() {
                                            if p != 0 {
                                                let a = SocketAddr::new(addr.ip(), p);
                                                let live = e.lock().unwrap();
                                                let mut ps = ps.lock().unwrap();
                                                if ps.len() < live.settings.max_peers
                                                    || ps.contains_key(&a)
                                                {
                                                    ps.entry(a)
                                                        .and_modify(|x| x.last_seen = now())
                                                        .or_insert(Peer {
                                                            address: a,
                                                            last_seen: now(),
                                                            connected: false,
                                                            manual: false,
                                                        });
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    if sync_due {
                        let addresses = {
                            let mut ps = ps.lock().unwrap();
                            ps.retain(|_, p| p.manual || now().saturating_sub(p.last_seen) < 60);
                            ps.keys().cloned().collect::<Vec<_>>()
                        };
                        for a in addresses {
                            if stop.load(Ordering::Relaxed) {
                                break;
                            }
                            let result = (|| -> Result<()> {
                                let mut socket =
                                    TcpStream::connect_timeout(&a, Duration::from_millis(500))
                                        .map_err(err)?;
                                configure(&socket)?;
                                let ids = inventory(&mut *e.lock().map_err(err)?);
                                write(&mut socket, &ids)?;
                                let remote: Vec<String> = read(&mut socket)?;
                                if remote.len() > 8192 {
                                    return Err("inventory limit".into());
                                }
                                let frames = missing(&mut *e.lock().map_err(err)?, &remote);
                                write(&mut socket, &frames)?;
                                let incoming = read(&mut socket)?;
                                let mut live = e.lock().map_err(err)?;
                                accept_frames(&mut live, incoming)?;
                                live.save(&path)
                            })();
                            let ok = result.is_ok();
                            if let Some(p) = ps.lock().unwrap().get_mut(&a) {
                                p.connected = ok;
                                if ok {
                                    p.last_seen = now();
                                }
                            }
                        }
                    }
                    if sync_due {
                        last_sync = now();
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }));
        }
        Ok(Self {
            engine,
            peers,
            stop,
            threads,
            port,
        })
    }
    pub fn add_peer(&self, address: SocketAddr) -> Result<()> {
        if address.port() == 0 {
            return Err("端口不能为 0".into());
        }
        let live = self.engine.lock().map_err(err)?;
        let limit = live.settings.max_peers;
        let mut ps = self.peers.lock().map_err(err)?;
        if ps.len() >= limit && !ps.contains_key(&address) {
            return Err("neighbor limit".into());
        }
        ps.insert(
            address,
            Peer {
                address,
                last_seen: now(),
                connected: false,
                manual: true,
            },
        );
        Ok(())
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

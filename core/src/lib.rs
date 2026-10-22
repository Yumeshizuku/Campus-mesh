//! Campus Mesh v1: authenticated encrypted envelopes and bounded DTN flooding.
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use x25519_dalek::{PublicKey, StaticSecret};
pub mod ffi;
pub mod network;
pub type Result<T> = std::result::Result<T, String>;
pub const MAX_FRAME: usize = 65536;
const MAX_CACHE: usize = 4096;
const MAX_HISTORY: usize = 10000;
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Contact {
    pub nickname: String,
    pub sign: String,
    pub exchange: String,
}
impl Contact {
    pub fn id(&self) -> String {
        hex::encode(Sha256::digest(hex::decode(&self.sign).unwrap_or_default()))
    }
    pub fn validate(&self) -> Result<()> {
        VerifyingKey::from_bytes(&bytes32(&self.sign)?).map_err(err)?;
        bytes32(&self.exchange)?;
        if self.nickname.len() > 96 {
            return Err("nickname too long".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub version: u8,
    pub id: String,
    pub sender: Contact,
    pub recipient: String,
    pub created: u64,
    pub expires: u64,
    pub hop_limit: u8,
    pub ephemeral: String,
    pub nonce: String,
    pub ciphertext: String,
    pub signature: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Frame {
    pub envelope: Envelope,
    pub hops: u8,
}
#[derive(Clone, Serialize, Deserialize)]
struct Payload {
    text: Option<String>,
    ack: Option<String>,
    group: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub peer: String,
    pub text: String,
    pub group: Option<String>,
    pub incoming: bool,
    pub status: String,
    pub time: u64,
}
#[derive(Serialize, Deserialize)]
pub struct Engine {
    signing: String,
    exchange: String,
    pub identity: Contact,
    pub contacts: BTreeMap<String, Contact>,
    pub relay: bool,
    #[serde(default)]
    pub manual_peers: Vec<String>,
    cache: BTreeMap<String, Frame>,
    seen: BTreeMap<String, u64>,
    pub history: Vec<Message>,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn random<const N: usize>() -> [u8; N] {
    let mut b = [0; N];
    OsRng.fill_bytes(&mut b);
    b
}
fn bytes32(s: &str) -> Result<[u8; 32]> {
    hex::decode(s)
        .map_err(err)?
        .try_into()
        .map_err(|_| "expected 32 bytes".into())
}
fn canonical(e: &Envelope) -> Result<Vec<u8>> {
    let mut copy = e.clone();
    copy.signature.clear();
    serde_json::to_vec(&copy).map_err(err)
}
fn key(secret: &StaticSecret, public: &PublicKey) -> Result<[u8; 32]> {
    let shared = secret.diffie_hellman(public);
    if !shared.was_contributory() {
        return Err("invalid exchange key".into());
    }
    let mut out = [0; 32];
    Hkdf::<Sha256>::new(Some(b"CampusMesh/v1"), shared.as_bytes())
        .expand(b"message", &mut out)
        .map_err(err)?;
    Ok(out)
}
fn aad(e: &Envelope) -> Vec<u8> {
    format!("CampusMesh/v1:{}:{}:{}", e.id, e.sender.id(), e.recipient).into_bytes()
}
impl Engine {
    pub fn new(nickname: &str) -> Self {
        let signing = SigningKey::from_bytes(&random());
        let exchange = StaticSecret::from(random::<32>());
        Self {
            signing: hex::encode(signing.to_bytes()),
            exchange: hex::encode(exchange.to_bytes()),
            identity: Contact {
                nickname: nickname
                    .chars()
                    .take(32)
                    .scan(0usize, |bytes, c| {
                        *bytes += c.len_utf8();
                        (*bytes <= 96).then_some(c)
                    })
                    .collect(),
                sign: hex::encode(signing.verifying_key().to_bytes()),
                exchange: hex::encode(PublicKey::from(&exchange).as_bytes()),
            },
            contacts: BTreeMap::new(),
            relay: true,
            manual_peers: Vec::new(),
            cache: BTreeMap::new(),
            seen: BTreeMap::new(),
            history: Vec::new(),
        }
    }
    pub fn add_contact(&mut self, c: Contact) -> Result<String> {
        c.validate()?;
        let id = c.id();
        if self.contacts.len() >= 1000 && !self.contacts.contains_key(&id) {
            return Err("contact limit".into());
        }
        self.contacts.insert(id.clone(), c);
        Ok(id)
    }
    pub fn prune(&mut self, now: u64) {
        for m in &mut self.history {
            if !m.incoming && m.status == "queued" && m.time.saturating_add(86400) <= now {
                m.status = "expired".into();
            }
        }
        self.cache.retain(|_, f| f.envelope.expires > now);
        self.seen.retain(|_, exp| *exp > now);
    }
    fn seal(&self, to: &Contact, payload: &Payload, now: u64) -> Result<Frame> {
        let ephemeral = StaticSecret::from(random::<32>());
        let nonce = random::<24>();
        let mut e = Envelope {
            version: 1,
            id: format!("{}:{}", self.identity.id(), hex::encode(random::<16>())),
            sender: self.identity.clone(),
            recipient: to.id(),
            created: now,
            expires: now + 86400,
            hop_limit: 8,
            ephemeral: hex::encode(PublicKey::from(&ephemeral).as_bytes()),
            nonce: hex::encode(nonce),
            ciphertext: String::new(),
            signature: String::new(),
        };
        let cipher = XChaCha20Poly1305::new(
            (&key(&ephemeral, &PublicKey::from(bytes32(&to.exchange)?))?).into(),
        );
        e.ciphertext = hex::encode(
            cipher
                .encrypt(
                    XNonce::from_slice(&nonce),
                    chacha20poly1305::aead::Payload {
                        msg: &serde_json::to_vec(payload).map_err(err)?,
                        aad: &aad(&e),
                    },
                )
                .map_err(err)?,
        );
        e.signature = hex::encode(
            SigningKey::from_bytes(&bytes32(&self.signing)?)
                .sign(&canonical(&e)?)
                .to_bytes(),
        );
        Ok(Frame {
            envelope: e,
            hops: 0,
        })
    }
    pub fn send(
        &mut self,
        to: &str,
        text: &str,
        group: Option<String>,
        now: u64,
    ) -> Result<String> {
        self.prune(now);
        if text.is_empty() || text.len() > 8192 || group.as_ref().is_some_and(|g| g.len() > 128) {
            return Err("invalid message length".into());
        }
        if self.cache.len() >= MAX_CACHE {
            return Err("outbox full".into());
        }
        let contact = self
            .contacts
            .get(to)
            .ok_or("add recipient public key first")?;
        let f = self.seal(
            contact,
            &Payload {
                text: Some(text.into()),
                ack: None,
                group: group.clone(),
            },
            now,
        )?;
        let id = f.envelope.id.clone();
        self.seen.insert(id.clone(), f.envelope.expires);
        self.cache.insert(id.clone(), f);
        self.push_history(Message {
            id: id.clone(),
            peer: to.into(),
            text: text.into(),
            group,
            incoming: false,
            status: "queued".into(),
            time: now,
        });
        Ok(id)
    }
    fn push_history(&mut self, m: Message) {
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(m);
    }
    pub fn receive(&mut self, f: Frame, now: u64) -> Result<bool> {
        self.prune(now);
        let e = &f.envelope;
        if serde_json::to_vec(&f).map_err(err)?.len() > MAX_FRAME
            || e.version != 1
            || e.hop_limit == 0
            || e.hop_limit > 8
            || f.hops >= e.hop_limit
            || e.expires <= now
            || e.created > now.saturating_add(300)
            || e.expires <= e.created
            || e.expires - e.created > 86400
        {
            return Err("invalid/expired frame".into());
        }
        e.sender.validate()?;
        if e.id.len() != 97
            || !e.id.starts_with(&format!("{}:", e.sender.id()))
            || hex::decode(&e.id[65..]).map_or(true, |b| b.len() != 16)
            || hex::decode(&e.recipient).map_or(true, |b| b.len() != 32)
        {
            return Err("invalid message identity".into());
        }
        let sig = Signature::from_slice(&hex::decode(&e.signature).map_err(err)?).map_err(err)?;
        VerifyingKey::from_bytes(&bytes32(&e.sender.sign)?)
            .map_err(err)?
            .verify_strict(&canonical(e)?, &sig)
            .map_err(err)?;
        if self.seen.contains_key(&e.id) {
            return Ok(false);
        }
        if self.seen.len() >= MAX_CACHE * 2 {
            return Err("seen capacity".into());
        }
        if e.recipient == self.identity.id() {
            let trusted = self
                .contacts
                .get(&e.sender.id())
                .ok_or("untrusted sender: exchange QR first")?;
            if trusted.sign != e.sender.sign || trusted.exchange != e.sender.exchange {
                return Err("identity key changed".into());
            }
            let nonce = hex::decode(&e.nonce).map_err(err)?;
            if nonce.len() != 24 {
                return Err("bad nonce".into());
            }
            let cipher = XChaCha20Poly1305::new(
                (&key(
                    &StaticSecret::from(bytes32(&self.exchange)?),
                    &PublicKey::from(bytes32(&e.ephemeral)?),
                )?)
                    .into(),
            );
            let plain = cipher
                .decrypt(
                    XNonce::from_slice(&nonce),
                    chacha20poly1305::aead::Payload {
                        msg: &hex::decode(&e.ciphertext).map_err(err)?,
                        aad: &aad(e),
                    },
                )
                .map_err(err)?;
            let p: Payload = serde_json::from_slice(&plain).map_err(err)?;
            if let Some(ack) = p.ack {
                if let Some(m) = self
                    .history
                    .iter_mut()
                    .find(|m| m.id == ack && !m.incoming && m.peer == e.sender.id())
                {
                    m.status = "delivered".into();
                    self.cache.remove(&ack);
                }
            } else if let Some(text) = p.text {
                if text.len() > 8192 || p.group.as_ref().is_some_and(|g| g.len() > 128) {
                    return Err("payload too large".into());
                }
                if self.cache.len() >= MAX_CACHE {
                    return Err("receipt cache full".into());
                }
                let ack = self.seal(
                    trusted,
                    &Payload {
                        text: None,
                        ack: Some(e.id.clone()),
                        group: None,
                    },
                    now,
                )?;
                self.seen
                    .insert(ack.envelope.id.clone(), ack.envelope.expires);
                self.cache.insert(ack.envelope.id.clone(), ack);
                self.push_history(Message {
                    id: e.id.clone(),
                    peer: e.sender.id(),
                    text,
                    group: p.group,
                    incoming: true,
                    status: "received".into(),
                    time: now,
                });
            } else {
                return Err("empty payload".into());
            }
        } else if self.relay {
            if self.cache.len() >= MAX_CACHE {
                return Err("relay cache full".into());
            }
            self.cache.insert(
                e.id.clone(),
                Frame {
                    envelope: e.clone(),
                    hops: f.hops + 1,
                },
            );
        } else {
            return Ok(false);
        }
        self.seen.insert(e.id.clone(), e.expires);
        Ok(true)
    }
    pub fn knows(&self, id: &str) -> bool {
        self.seen.contains_key(id)
    }
    pub fn outgoing(&mut self, now: u64) -> Vec<Frame> {
        self.prune(now);
        self.cache
            .values()
            .filter(|f| {
                f.hops < f.envelope.hop_limit
                    && (self.relay || f.envelope.sender.id() == self.identity.id())
            })
            .cloned()
            .collect()
    }
    pub fn outgoing_batch(&mut self, now: u64, cursor: &mut usize, limit: usize) -> Vec<Frame> {
        self.prune(now);
        let frames: Vec<_> = self
            .cache
            .values()
            .filter(|f| {
                f.hops < f.envelope.hop_limit
                    && (self.relay || f.envelope.sender.id() == self.identity.id())
            })
            .collect();
        if frames.is_empty() {
            *cursor = 0;
            return Vec::new();
        }
        let count = limit.min(frames.len());
        let output = frames
            .iter()
            .cycle()
            .skip(*cursor % frames.len())
            .take(count)
            .map(|f| (*f).clone())
            .collect();
        *cursor = (*cursor + count) % frames.len();
        output
    }
    pub fn load(path: &Path) -> Result<Self> {
        serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(err)?;
        }
        let tmp = PathBuf::from(format!("{}.tmp", path.display()));
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        use std::io::Write;
        let mut file = opts.open(&tmp).map_err(err)?;
        file.write_all(&serde_json::to_vec(self).map_err(err)?)
            .map_err(err)?;
        file.sync_all().map_err(err)?;
        fs::rename(tmp, path).map_err(err)?;
        #[cfg(unix)]
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::File::open(parent)
                .map_err(err)?
                .sync_all()
                .map_err(err)?;
        }
        Ok(())
    }
}

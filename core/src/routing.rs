//! Bounded, opportunistic next-hop selection. Link names are transport-local,
//! never trusted device identities; only verified envelopes teach reverse routes.
use crate::*;
#[derive(Clone)]
struct Link {
    rssi: i16,
    seen: u64,
}
#[derive(Clone)]
struct Route {
    hops: u8,
    seen: u64,
}
#[derive(Clone, Default)]
struct Attempt {
    count: u32,
    next: u64,
}
#[derive(Clone, Default)]
pub struct Router {
    links: BTreeMap<String, Link>,
    routes: BTreeMap<(String, String), Route>,
    attempts: BTreeMap<(String, String), Attempt>,
    known: BTreeMap<(String, String), u64>,
}
impl Router {
    pub fn touch(&mut self, link: &str, rssi: i16, now: u64) -> Result<()> {
        self.prune(now);
        if link.is_empty()
            || link.len() > 128
            || (!self.links.contains_key(link) && self.links.len() >= 24)
        {
            return Err("BLE link limit".into());
        }
        self.links.insert(
            link.into(),
            Link {
                rssi: rssi.clamp(-127, -1),
                seen: now,
            },
        );
        Ok(())
    }
    pub fn disconnect(&mut self, link: &str) {
        self.links.remove(link);
        self.routes.retain(|(_, l), _| l != link);
        self.attempts.retain(|(_, l), _| l != link);
        self.known.retain(|(_, l), _| l != link);
    }
    fn prune(&mut self, now: u64) {
        self.links.retain(|_, l| now.saturating_sub(l.seen) < 90);
        self.routes
            .retain(|(_, l), r| self.links.contains_key(l) && now.saturating_sub(r.seen) < 60);
        self.known
            .retain(|(_, l), exp| *exp > now && self.links.contains_key(l));
        self.attempts.retain(|(_, l), _| self.links.contains_key(l));
    }
    fn observe(&mut self, link: &str, f: &Frame, now: u64) {
        if self.known.len() < 8192 {
            self.known
                .insert((f.envelope.id.clone(), link.into()), f.envelope.expires);
        }
        if self.routes.len() < 2048 {
            let route = self
                .routes
                .entry((f.envelope.sender.id(), link.into()))
                .or_insert(Route {
                    hops: f.hops,
                    seen: now,
                });
            if now.saturating_sub(route.seen) >= 30 || f.hops <= route.hops {
                route.hops = f.hops;
                route.seen = now;
            }
        }
    }
    fn preferred(&self, recipient: &str, now: u64) -> Vec<String> {
        let mut routes: Vec<_> = self
            .routes
            .iter()
            .filter_map(|((dest, link), r)| {
                let quality = self.links.get(link)?;
                (dest == recipient && now.saturating_sub(r.seen) < 60).then_some((
                    i32::from(r.hops) * 256 + i32::from(-quality.rssi),
                    link.clone(),
                ))
            })
            .collect();
        routes.sort();
        routes.into_iter().take(2).map(|(_, l)| l).collect()
    }
}
impl Engine {
    pub fn receive_on_link(&mut self, link: &str, rssi: i16, f: Frame, now: u64) -> Result<bool> {
        self.routing.touch(link, rssi, now)?;
        // receive verifies signature even for duplicates. Never learn from rejected input.
        let accepted = self.receive(f.clone(), now)?;
        self.routing.observe(link, &f, now);
        Ok(accepted)
    }
    pub fn outgoing_on_link(
        &mut self,
        link: &str,
        rssi: i16,
        now: u64,
        limit: usize,
    ) -> Result<Vec<Frame>> {
        self.prune(now);
        self.routing.touch(link, rssi, now)?;
        self.routing
            .attempts
            .retain(|(id, _), _| self.cache.contains_key(id));
        let mut frames: Vec<_> = self
            .cache
            .values()
            .filter(|f| {
                f.hops < f.envelope.hop_limit
                    && (self.relay || f.envelope.sender.id() == self.identity.id())
            })
            .cloned()
            .collect();
        // Unsent frames first; oldest creation time wins, so random IDs cannot starve a queue.
        frames.sort_by_key(|f| {
            (
                self.routing
                    .attempts
                    .get(&(f.envelope.id.clone(), link.into()))
                    .map_or(0, |a| a.count),
                u8::from(
                    !(f.envelope.sender.id() == self.identity.id()
                        && !self.history.iter().any(|m| m.id == f.envelope.id)
                        && !self.group_outbox.contains_key(&f.envelope.id)),
                ),
                f.envelope.created,
                f.envelope.id.clone(),
            )
        });
        let mut output = Vec::new();
        for f in frames {
            let key = (f.envelope.id.clone(), link.into());
            if self.routing.known.contains_key(&key) {
                continue;
            }
            let preferred = self.routing.preferred(&f.envelope.recipient, now);
            // Start with the best two learned next hops, then expand to other links
            // after 15 seconds if no E2E receipt has removed the cached message.
            if !preferred.is_empty()
                && !preferred.iter().any(|l| l == link)
                && now.saturating_sub(f.envelope.created) < 15
            {
                continue;
            }
            let a = self.routing.attempts.entry(key).or_default();
            if now < a.next {
                continue;
            }
            a.count = a.count.saturating_add(1);
            // Bounded deterministic jitter avoids synchronized retries without a clock/RNG dependency.
            let jitter = f
                .envelope
                .id
                .bytes()
                .chain(link.bytes())
                .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64))
                % 3;
            a.next = now.saturating_add((2u64.saturating_pow(a.count.min(8))).min(300) + jitter);
            output.push(f);
            if output.len() >= limit.clamp(1, 24).min(self.settings.batch_size) {
                break;
            }
        }
        Ok(output)
    }
    pub fn mark_read(&mut self, id: &str, now: u64) -> Result<()> {
        self.prune(now);
        let message = self
            .history
            .iter()
            .find(|m| m.id == id && m.incoming)
            .ok_or("incoming message not found")?
            .clone();
        if message.status == "read" {
            return Ok(());
        }
        let contact = self
            .contacts
            .get(&message.peer)
            .or_else(|| {
                self.groups
                    .values()
                    .find_map(|g| g.definition.members.get(&message.peer))
            })
            .ok_or("read receipt recipient not found")?;
        let receipt = self.seal(
            contact,
            &Payload {
                read: Some(id.into()),
                ..Payload::default()
            },
            now,
        )?;
        if self.cache.len() >= self.settings.cache_capacity || self.seen.len() >= MAX_CACHE * 2 {
            return Err("receipt cache full".into());
        }
        self.seen
            .insert(receipt.envelope.id.clone(), receipt.envelope.expires);
        self.cache.insert(receipt.envelope.id.clone(), receipt);
        self.history.iter_mut().find(|m| m.id == id).unwrap().status = "read".into();
        Ok(())
    }
    pub(crate) fn accept_read(&mut self, sender: &Contact, id: &str) {
        if let Some(m) = self.history.iter_mut().find(|m| m.id == id && !m.incoming) {
            if m.group.is_some() {
                if let Some(status) = m.delivery.get_mut(&sender.id()) {
                    *status = "read".into();
                    m.status = if m.delivery.values().all(|s| s == "read") {
                        "read"
                    } else if m.delivery.values().all(|s| s == "read" || s == "delivered") {
                        "delivered"
                    } else {
                        "partially_delivered"
                    }
                    .into();
                    let ids: Vec<_> = self
                        .group_outbox
                        .iter()
                        .filter(|(_, d)| d.message_id == id && d.recipient == sender.id())
                        .map(|(key, _)| key.clone())
                        .collect();
                    for key in ids {
                        self.cache.remove(&key);
                    }
                }
            } else if m.peer == sender.id() {
                m.status = "read".into();
                self.cache.remove(id);
            }
        }
    }
}

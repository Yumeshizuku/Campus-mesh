//! Persistent device contacts, local remarks and owner-signed Mesh groups.
use crate::*;

pub const MAX_GROUP_MEMBERS: usize = 32;
const MAX_GROUPS: usize = 64;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct MeshSettings {
    pub hop_limit: u8,
    pub message_ttl_secs: u64,
    pub sync_interval_secs: u64,
    pub batch_size: usize,
    pub max_peers: usize,
    pub cache_capacity: usize,
}
impl Default for MeshSettings {
    fn default() -> Self {
        Self {
            hop_limit: 8,
            message_ttl_secs: 86400,
            sync_interval_secs: 5,
            batch_size: 24,
            max_peers: 300,
            cache_capacity: 4096,
        }
    }
}
impl MeshSettings {
    pub fn validate(&self) -> Result<()> {
        if !(1..=8).contains(&self.hop_limit)
            || !(60..=86400).contains(&self.message_ttl_secs)
            || !(1..=60).contains(&self.sync_interval_secs)
            || !(1..=24).contains(&self.batch_size)
            || !(1..=300).contains(&self.max_peers)
            || !(64..=4096).contains(&self.cache_capacity)
        {
            return Err("Mesh 参数超出范围：跳数 1–8，有效期 60–86400 秒，同步 1–60 秒，批量 1–24，邻居 1–300，缓存 64–4096".into());
        }
        Ok(())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct FriendMetadata {
    pub remark: String,
    pub added_at: u64,
    pub last_message_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ContactCard {
    pub kind: String,
    pub version: u8,
    pub device_id: String,
    pub contact: Contact,
    pub signature: String,
}
impl ContactCard {
    fn canonical(&self) -> Result<Vec<u8>> {
        let mut value = self.clone();
        value.signature.clear();
        serde_json::to_vec(&value).map_err(err)
    }
    pub fn validate(&self) -> Result<()> {
        self.contact.validate()?;
        if self.kind != "campusmesh.contact"
            || self.version != 1
            || self.device_id != self.contact.id()
        {
            return Err("二维码中的设备 ID 与公钥不匹配".into());
        }
        verify(&self.contact, &self.canonical()?, &self.signature)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GroupDefinition {
    pub id: String,
    pub name: String,
    pub owner: Contact,
    pub revision: u64,
    pub members: BTreeMap<String, Contact>,
    pub dissolved: bool,
    pub signature: String,
}
impl GroupDefinition {
    fn canonical(&self) -> Result<Vec<u8>> {
        let mut value = self.clone();
        value.signature.clear();
        serde_json::to_vec(&value).map_err(err)
    }
    pub fn validate(&self) -> Result<()> {
        self.owner.validate()?;
        if !valid_id(&self.id, &self.owner.id())
            || self.revision == 0
            || self.name.trim().is_empty()
            || self.name.len() > 96
            || self.members.is_empty()
            || self.members.len() > MAX_GROUP_MEMBERS
            || self.members.get(&self.owner.id()) != Some(&self.owner)
        {
            return Err("群聊信息无效".into());
        }
        for (id, member) in &self.members {
            member.validate()?;
            if *id != member.id() {
                return Err("群成员设备 ID 不匹配".into());
            }
        }
        verify(&self.owner, &self.canonical()?, &self.signature)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct GroupRecord {
    #[serde(flatten)]
    pub definition: GroupDefinition,
    pub joined: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct GroupDelivery {
    pub message_id: String,
    pub recipient: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GroupInvite {
    pub kind: String,
    pub version: u8,
    pub group_id: String,
    pub group_name: String,
    pub owner: Contact,
    pub nonce: String,
    pub expires: u64,
    pub signature: String,
}
impl GroupInvite {
    fn canonical(&self) -> Result<Vec<u8>> {
        let mut value = self.clone();
        value.signature.clear();
        serde_json::to_vec(&value).map_err(err)
    }
    pub fn validate(&self, now: u64) -> Result<()> {
        self.owner.validate()?;
        if self.kind != "campusmesh.group"
            || self.version != 1
            || !valid_id(&self.group_id, &self.owner.id())
            || self.group_name.trim().is_empty()
            || self.group_name.len() > 96
            || self.nonce.len() != 32
            || hex::decode(&self.nonce).map_or(true, |x| x.len() != 16)
            || self.expires <= now
            || self.expires > now.saturating_add(600)
        {
            return Err("群邀请无效或已过期，请让群主重新生成二维码".into());
        }
        verify(&self.owner, &self.canonical()?, &self.signature)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GroupEvent {
    Snapshot { group: GroupDefinition },
    Join { invite: GroupInvite },
    Leave { group_id: String },
    Sync { group_id: String },
}

fn verify(contact: &Contact, data: &[u8], signature: &str) -> Result<()> {
    let signature = Signature::from_slice(&hex::decode(signature).map_err(err)?).map_err(err)?;
    VerifyingKey::from_bytes(&bytes32(&contact.sign)?)
        .map_err(err)?
        .verify_strict(data, &signature)
        .map_err(err)
}
pub(crate) fn valid_id(id: &str, sender: &str) -> bool {
    id.len() == 97
        && sender.len() == 64
        && hex::decode(sender).is_ok_and(|bytes| bytes.len() == 32)
        && id.starts_with(&format!("{sender}:"))
        && hex::decode(&id[65..]).is_ok_and(|x| x.len() == 16)
}

impl Engine {
    fn sign_bytes(&self, data: &[u8]) -> Result<String> {
        Ok(hex::encode(
            SigningKey::from_bytes(&bytes32(&self.signing)?)
                .sign(data)
                .to_bytes(),
        ))
    }
    pub fn contact_card(&self) -> Result<ContactCard> {
        let mut card = ContactCard {
            kind: "campusmesh.contact".into(),
            version: 1,
            device_id: self.identity.id(),
            contact: self.identity.clone(),
            signature: String::new(),
        };
        card.signature = self.sign_bytes(&card.canonical()?)?;
        Ok(card)
    }
    pub fn scan_card(&mut self, text: &str, now: u64) -> Result<serde_json::Value> {
        self.prune(now);
        if text.len() > 8192 {
            return Err("二维码内容过长".into());
        }
        let value: serde_json::Value = serde_json::from_str(text).map_err(err)?;
        if value["kind"] == "campusmesh.group" {
            let invite: GroupInvite = serde_json::from_value(value).map_err(err)?;
            let id = invite.group_id.clone();
            self.join_group(invite, now)?;
            return Ok(serde_json::json!({"kind":"group","id":id}));
        }
        let contact = if value["kind"].is_null() {
            // Import old public-key QR codes without changing existing device IDs.
            serde_json::from_value(value).map_err(err)?
        } else {
            let card: ContactCard = serde_json::from_value(value).map_err(err)?;
            card.validate()?;
            card.contact
        };
        let id = self.add_contact(contact)?;
        let metadata = self.friends.entry(id.clone()).or_default();
        if metadata.added_at == 0 {
            metadata.added_at = now;
        }
        Ok(serde_json::json!({"kind":"friend","id":id,"device_id":id}))
    }
    pub fn set_remark(&mut self, id: &str, remark: &str) -> Result<()> {
        if !self.contacts.contains_key(id) {
            return Err("好友不存在".into());
        }
        let remark = remark.trim();
        if remark.len() > 96 {
            return Err("备注最多 96 个 UTF-8 字节".into());
        }
        self.friends.entry(id.into()).or_default().remark = remark.into();
        Ok(())
    }
    pub fn update_settings(&mut self, settings: MeshSettings) -> Result<()> {
        settings.validate()?;
        if settings.cache_capacity < self.cache.len()
            || settings.max_peers < self.manual_peers.len()
        {
            return Err("新上限低于当前待传封装或已记住的邻居数量，请先处理现有数据".into());
        }
        self.settings = settings;
        Ok(())
    }
    fn transact<T>(&mut self, operation: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let mut next = self.clone();
        let result = operation(&mut next)?;
        *self = next;
        Ok(result)
    }
    fn check_room(&self, count: usize) -> Result<()> {
        if self.cache.len().saturating_add(count) > self.settings.cache_capacity
            || self.seen.len().saturating_add(count) > MAX_CACHE * 2
        {
            return Err("待传缓存不足，请等待同步或调大缓存上限".into());
        }
        Ok(())
    }
    fn queue_payload(&mut self, to: &Contact, payload: &Payload, now: u64) -> Result<String> {
        self.check_room(1)?;
        let frame = self.seal(to, payload, now)?;
        if serde_json::to_vec(&frame).map_err(err)?.len() > MAX_FRAME {
            return Err("群信息超出封装上限".into());
        }
        let id = frame.envelope.id.clone();
        self.seen.insert(id.clone(), frame.envelope.expires);
        self.cache.insert(id.clone(), frame);
        Ok(id)
    }
    fn publish_group(
        &mut self,
        mut definition: GroupDefinition,
        previous: BTreeMap<String, Contact>,
        now: u64,
    ) -> Result<()> {
        definition.signature = self.sign_bytes(&definition.canonical()?)?;
        definition.validate()?;
        let mut recipients = previous;
        recipients.extend(definition.members.clone());
        recipients.remove(&self.identity.id());
        self.check_room(recipients.len())?;
        for member in recipients.values() {
            self.queue_payload(
                member,
                &Payload {
                    group_event: Some(GroupEvent::Snapshot {
                        group: definition.clone(),
                    }),
                    ..Payload::default()
                },
                now,
            )?;
        }
        self.groups.insert(
            definition.id.clone(),
            GroupRecord {
                joined: !definition.dissolved,
                definition,
            },
        );
        Ok(())
    }
    pub fn create_group(
        &mut self,
        name: &str,
        members: &[String],
        legacy_name: Option<&str>,
        now: u64,
    ) -> Result<String> {
        if let Some(old) = legacy_name {
            if let Some(id) = self.legacy_groups.get(old) {
                return Ok(id.clone());
            }
        }
        self.transact(|next| {
            if next.groups.len() >= MAX_GROUPS {
                return Err("最多保存 64 个群聊".into());
            }
            let mut selected = BTreeMap::new();
            selected.insert(next.identity.id(), next.identity.clone());
            for id in members {
                selected.insert(
                    id.clone(),
                    next.contacts
                        .get(id)
                        .ok_or("请先添加好友再邀请入群")?
                        .clone(),
                );
            }
            let id = format!("{}:{}", next.identity.id(), hex::encode(random::<16>()));
            let mut display_name = name.trim().to_string();
            if legacy_name.is_some() {
                while display_name.len() > 96 {
                    display_name.pop();
                }
            }
            next.publish_group(
                GroupDefinition {
                    id: id.clone(),
                    name: display_name,
                    owner: next.identity.clone(),
                    revision: 1,
                    members: selected,
                    dissolved: false,
                    signature: String::new(),
                },
                BTreeMap::new(),
                now,
            )?;
            if let Some(old) = legacy_name {
                next.legacy_groups.insert(old.into(), id.clone());
                for message in &mut next.history {
                    if message.group.as_deref() == Some(old) {
                        message.group = Some(id.clone());
                    }
                }
            }
            Ok(id)
        })
    }
    pub fn edit_group(
        &mut self,
        id: &str,
        name: &str,
        member_ids: &[String],
        now: u64,
    ) -> Result<()> {
        self.transact(|next| {
            let record = next.groups.get(id).ok_or("群聊不存在")?.clone();
            let mut group = record.definition;
            if group.owner.id() != next.identity.id() || !record.joined || group.dissolved {
                return Err("只有群主可以修改群名和成员".into());
            }
            let previous = group.members.clone();
            let mut members = BTreeMap::new();
            // The signed owner identity remains stable even after a local nickname change.
            members.insert(group.owner.id(), group.owner.clone());
            for id in member_ids {
                if id == &group.owner.id() {
                    continue;
                }
                members.insert(
                    id.clone(),
                    next.contacts
                        .get(id)
                        .or_else(|| previous.get(id))
                        .ok_or("未知群成员")?
                        .clone(),
                );
            }
            group.members = members;
            group.name = name.trim().into();
            group.revision = group.revision.checked_add(1).ok_or("群版本超限")?;
            next.publish_group(group, previous, now)
        })
    }
    pub fn group_invite(&mut self, id: &str, now: u64) -> Result<GroupInvite> {
        let group = self.groups.get(id).ok_or("群聊不存在")?;
        if !group.joined
            || group.definition.dissolved
            || group.definition.owner.id() != self.identity.id()
        {
            return Err("只有群主可以生成邀请二维码".into());
        }
        let mut invite = GroupInvite {
            kind: "campusmesh.group".into(),
            version: 1,
            group_id: id.into(),
            group_name: group.definition.name.clone(),
            owner: self.identity.clone(),
            nonce: hex::encode(random::<16>()),
            expires: now.saturating_add(600),
            signature: String::new(),
        };
        invite.signature = self.sign_bytes(&invite.canonical()?)?;
        self.group_invites
            .retain(|_, old| old.expires > now && old.group_id != id);
        if self.group_invites.len() >= MAX_GROUPS {
            return Err("待处理邀请过多".into());
        }
        self.group_invites
            .insert(invite.nonce.clone(), invite.clone());
        Ok(invite)
    }
    pub fn join_group(&mut self, invite: GroupInvite, now: u64) -> Result<()> {
        invite.validate(now)?;
        if invite.owner.id() == self.identity.id() {
            return Err("你已经是这个群的群主".into());
        }
        if self
            .groups
            .get(&invite.group_id)
            .is_some_and(|r| r.joined && !r.definition.dissolved)
        {
            return Err("你已经加入这个群聊".into());
        }
        if self.pending_groups.len() >= MAX_GROUPS || self.groups.len() >= MAX_GROUPS {
            return Err("群聊数量已达上限".into());
        }
        if self
            .pending_groups
            .values()
            .any(|old| old.group_id == invite.group_id)
        {
            return Err("入群申请已提交，等待与群主同步".into());
        }
        self.queue_payload(
            &invite.owner,
            &Payload {
                group_event: Some(GroupEvent::Join {
                    invite: invite.clone(),
                }),
                ..Payload::default()
            },
            now,
        )?;
        self.pending_groups.insert(invite.group_id.clone(), invite);
        Ok(())
    }
    pub fn leave_group(&mut self, id: &str, now: u64) -> Result<()> {
        self.transact(|next| {
            let record = next.groups.get(id).ok_or("群聊不存在")?.clone();
            if !record.joined || record.definition.dissolved {
                return Err("已退出这个群聊".into());
            }
            if record.definition.owner.id() == next.identity.id() {
                let mut group = record.definition;
                let previous = group.members.clone();
                group.revision = group.revision.checked_add(1).ok_or("群版本超限")?;
                group.dissolved = true;
                next.publish_group(group, previous, now)?;
            } else {
                next.queue_payload(
                    &record.definition.owner,
                    &Payload {
                        group_event: Some(GroupEvent::Leave {
                            group_id: id.into(),
                        }),
                        ..Payload::default()
                    },
                    now,
                )?;
                next.groups.get_mut(id).unwrap().joined = false;
            }
            next.group_invites.retain(|_, invite| invite.group_id != id);
            Ok(())
        })
    }
    pub fn sync_group(&mut self, id: &str, now: u64) -> Result<()> {
        let record = self.groups.get(id).ok_or("群聊不存在")?.clone();
        if !record.joined || record.definition.dissolved {
            return Err("已不在这个群聊中".into());
        }
        if record.definition.owner.id() == self.identity.id() {
            return Ok(());
        }
        self.queue_payload(
            &record.definition.owner,
            &Payload {
                group_event: Some(GroupEvent::Sync {
                    group_id: id.into(),
                }),
                ..Payload::default()
            },
            now,
        )?;
        Ok(())
    }
    pub fn send_group(&mut self, id: &str, text: &str, now: u64) -> Result<String> {
        self.transact(|next| {
            next.prune(now);
            let record = next.groups.get(id).ok_or("群聊不存在")?.clone();
            let group = record.definition;
            if !record.joined || group.dissolved || !group.members.contains_key(&next.identity.id())
            {
                return Err("你已不在这个群聊中".into());
            }
            if text.trim().is_empty() || text.len() > 8192 {
                return Err("消息不能为空且最多 8192 字节".into());
            }
            let recipients: Vec<_> = group
                .members
                .values()
                .filter(|m| m.id() != next.identity.id())
                .cloned()
                .collect();
            if recipients.is_empty() {
                return Err("请先邀请至少一位群成员".into());
            }
            next.check_room(recipients.len())?;
            let message_id = format!("{}:{}", next.identity.id(), hex::encode(random::<16>()));
            let mut delivery = BTreeMap::new();
            for member in recipients {
                let frame_id = next.queue_payload(
                    &member,
                    &Payload {
                        text: Some(text.into()),
                        group: Some(id.into()),
                        group_revision: Some(group.revision),
                        group_message_id: Some(message_id.clone()),
                        ..Payload::default()
                    },
                    now,
                )?;
                next.group_outbox.insert(
                    frame_id,
                    GroupDelivery {
                        message_id: message_id.clone(),
                        recipient: member.id(),
                    },
                );
                delivery.insert(member.id(), "queued".into());
            }
            next.push_history(Message {
                id: message_id.clone(),
                peer: next.identity.id(),
                text: text.into(),
                group: Some(id.into()),
                incoming: false,
                status: "queued".into(),
                time: now,
                expires: now.saturating_add(next.settings.message_ttl_secs),
                delivery,
                sender_name: next.identity.nickname.clone(),
            });
            Ok(message_id)
        })
    }
    pub(crate) fn known_sender(&self, sender: &Contact) -> bool {
        let matches =
            |known: &Contact| known.sign == sender.sign && known.exchange == sender.exchange;
        self.contacts.get(&sender.id()).is_some_and(matches)
            || self
                .groups
                .values()
                .any(|r| r.definition.members.get(&sender.id()).is_some_and(matches))
            || self
                .pending_groups
                .values()
                .any(|invite| invite.owner.id() == sender.id() && matches(&invite.owner))
    }
    pub(crate) fn accept_group_event(
        &mut self,
        sender: &Contact,
        event: GroupEvent,
        now: u64,
    ) -> Result<()> {
        self.transact(|next| {
            match event {
                GroupEvent::Snapshot { group } => {
                    group.validate()?;
                    if group.owner.sign != sender.sign || group.owner.exchange != sender.exchange {
                        return Err("群信息不是由群主签发".into());
                    }
                    let previous = next.groups.get(&group.id);
                    if let Some(old) = previous {
                        if old.definition.owner.sign != group.owner.sign
                            || old.definition.owner.exchange != group.owner.exchange
                        {
                            return Err("群主密钥不匹配".into());
                        }
                        if group.revision <= old.definition.revision {
                            return Ok(());
                        }
                    } else if next.groups.len() >= MAX_GROUPS
                        || !group.members.contains_key(&next.identity.id())
                        || (!next.contacts.get(&sender.id()).is_some_and(|c| {
                            c.sign == sender.sign && c.exchange == sender.exchange
                        }) && !next.pending_groups.get(&group.id).is_some_and(|i| {
                            i.owner.sign == sender.sign && i.owner.exchange == sender.exchange
                        }))
                    {
                        return Err("请先扫码接受群邀请，或添加群主为好友".into());
                    }
                    let included = group.members.get(&next.identity.id()).is_some_and(|c| {
                        c.sign == next.identity.sign && c.exchange == next.identity.exchange
                    });
                    let requested = next.pending_groups.contains_key(&group.id);
                    let joined = included
                        && !group.dissolved
                        && (requested || previous.is_none_or(|r| r.joined));
                    next.pending_groups.remove(&group.id);
                    next.groups.insert(
                        group.id.clone(),
                        GroupRecord {
                            definition: group,
                            joined,
                        },
                    );
                }
                GroupEvent::Join { invite } => {
                    invite.validate(now)?;
                    let issued = next
                        .group_invites
                        .get(&invite.nonce)
                        .ok_or("群邀请已使用、撤销或过期")?;
                    if issued.signature != invite.signature
                        || invite.owner.id() != next.identity.id()
                    {
                        return Err("群邀请不匹配".into());
                    }
                    let record = next
                        .groups
                        .get(&invite.group_id)
                        .ok_or("群聊不存在")?
                        .clone();
                    if !record.joined || record.definition.dissolved {
                        return Err("群聊已解散".into());
                    }
                    let mut group = record.definition;
                    let previous = group.members.clone();
                    if let Some(old) = group.members.get(&sender.id()) {
                        if old.exchange != sender.exchange {
                            return Err("群成员密钥发生变化".into());
                        }
                    }
                    group.members.insert(sender.id(), sender.clone());
                    group.revision = group.revision.checked_add(1).ok_or("群版本超限")?;
                    next.publish_group(group, previous, now)?;
                    next.group_invites.remove(&invite.nonce);
                }
                GroupEvent::Sync { group_id } => {
                    let record = next.groups.get(&group_id).ok_or("群聊不存在")?.clone();
                    if record.definition.owner.id() != next.identity.id()
                        || !record
                            .definition
                            .members
                            .get(&sender.id())
                            .is_some_and(|m| m.sign == sender.sign && m.exchange == sender.exchange)
                    {
                        return Err("只有当前成员可以请求同步群信息".into());
                    }
                    next.queue_payload(
                        sender,
                        &Payload {
                            group_event: Some(GroupEvent::Snapshot {
                                group: record.definition,
                            }),
                            ..Payload::default()
                        },
                        now,
                    )?;
                }
                GroupEvent::Leave { group_id } => {
                    let group = next
                        .groups
                        .get(&group_id)
                        .ok_or("群聊不存在")?
                        .definition
                        .clone();
                    if group.owner.id() != next.identity.id()
                        || !group
                            .members
                            .get(&sender.id())
                            .is_some_and(|c| c.sign == sender.sign && c.exchange == sender.exchange)
                        || sender.id() == next.identity.id()
                    {
                        return Err("无效的退群申请".into());
                    }
                    let members: Vec<_> = group
                        .members
                        .keys()
                        .filter(|id| **id != sender.id())
                        .cloned()
                        .collect();
                    next.edit_group(&group_id, &group.name, &members, now)?;
                }
            }
            // Reserve room for the receipt and incoming deduplication ID.
            next.check_room(2)
        })
    }
    pub(crate) fn validate_group_message(
        &self,
        sender: &Contact,
        payload: &Payload,
    ) -> Result<String> {
        let id = payload.group.as_ref().ok_or("缺少群 ID")?;
        let record = self.groups.get(id).ok_or("尚未同步这个群的成员信息")?;
        let group = &record.definition;
        if !record.joined
            || group.dissolved
            || !group.members.contains_key(&self.identity.id())
            || !group
                .members
                .get(&sender.id())
                .is_some_and(|c| c.sign == sender.sign && c.exchange == sender.exchange)
            || payload
                .group_revision
                .is_none_or(|revision| revision == 0 || revision > group.revision)
        {
            return Err("发件人不是当前群成员，或群信息尚未同步".into());
        }
        let logical_id = payload.group_message_id.as_deref().ok_or("缺少群消息 ID")?;
        if !valid_id(logical_id, &sender.id()) {
            return Err("群消息 ID 无效".into());
        }
        Ok(logical_id.into())
    }
    pub(crate) fn accept_receipt(&mut self, sender: &Contact, ack: &str) {
        if let Some(delivery) = self.group_outbox.get(ack).cloned() {
            if delivery.recipient != sender.id() {
                return;
            }
            let logical = delivery.message_id;
            if let Some(message) = self
                .history
                .iter_mut()
                .find(|m| m.id == logical && !m.incoming)
            {
                if !message.delivery.contains_key(&sender.id()) {
                    return;
                }
                message.delivery.insert(sender.id(), "delivered".into());
                message.status = if message.delivery.values().all(|s| s == "delivered") {
                    "delivered"
                } else {
                    "partially_delivered"
                }
                .into();
            }
            self.group_outbox.remove(ack);
        } else if let Some(message) = self
            .history
            .iter_mut()
            .find(|m| m.id == ack && !m.incoming && m.peer == sender.id())
        {
            message.status = "delivered".into();
        } else if !self.cache.get(ack).is_some_and(|f| {
            f.envelope.sender.id() == self.identity.id() && f.envelope.recipient == sender.id()
        }) {
            return;
        }
        self.cache.remove(ack);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_group_identifiers_cannot_panic_at_a_utf8_boundary() {
        let prefix = "组".repeat(32);
        assert!(!valid_id(&format!("{prefix}:"), &prefix));
        assert!(!valid_id(
            &format!("{}:{}", "z".repeat(64), "0".repeat(32)),
            &"z".repeat(64)
        ));
    }
    #[test]
    fn receipt_is_bound_to_the_exact_group_recipient() {
        let mut owner = Engine::new("owner");
        let b = Engine::new("b");
        let c = Engine::new("c");
        owner.add_contact(b.identity.clone()).unwrap();
        owner.add_contact(c.identity.clone()).unwrap();
        let id = owner
            .create_group("group", &[b.identity.id(), c.identity.id()], None, 100)
            .unwrap();
        let logical = owner.send_group(&id, "hello", 100).unwrap();
        let frame = owner
            .cache
            .values()
            .find(|f| {
                f.envelope.recipient == b.identity.id()
                    && owner.group_outbox.contains_key(&f.envelope.id)
            })
            .unwrap()
            .clone();
        let forged = c
            .seal(
                &owner.identity,
                &Payload {
                    ack: Some(frame.envelope.id.clone()),
                    ..Payload::default()
                },
                100,
            )
            .unwrap();
        owner.receive(forged, 100).unwrap();
        assert!(owner.cache.contains_key(&frame.envelope.id));
        assert_eq!(
            owner
                .history
                .iter()
                .find(|m| m.id == logical)
                .unwrap()
                .status,
            "queued"
        );
        let legitimate = b
            .seal(
                &owner.identity,
                &Payload {
                    ack: Some(frame.envelope.id.clone()),
                    ..Payload::default()
                },
                100,
            )
            .unwrap();
        owner.receive(legitimate, 100).unwrap();
        assert!(!owner.cache.contains_key(&frame.envelope.id));
        assert_eq!(
            owner
                .history
                .iter()
                .find(|m| m.id == logical)
                .unwrap()
                .status,
            "partially_delivered"
        );
    }
}

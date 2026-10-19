//! Bounded account-private contacts, requests and inbox. Grants remain canonical.
use super::collaboration::{decoded, encoded, retry};
use super::*;
use crate::permissions::Actor;
use serde::Deserialize;
use sovereign_cloud::{drive::*, people::*};
const DAY: u64 = 86_400_000_000_000;
const INBOX_LIMIT: usize = 100;
#[derive(Clone, Serialize, Deserialize)]
struct Request {
    at: u64,
    message: String,
    resolved: bool,
}
pub(super) struct PeopleStore<M: Memory> {
    next: StableCell<u64, Vm<M>>,
    inbox: StableBTreeMap<(Principal, u64), Vec<u8>, Vm<M>>,
    requests: StableBTreeMap<(ObjectId, Principal), Vec<u8>, Vm<M>>,
    contacts: StableBTreeMap<(Principal, Principal), String, Vm<M>>,
    settings: StableBTreeMap<Principal, bool, Vm<M>>,
    rate: StableBTreeMap<Principal, Vec<u8>, Vm<M>>,
}
pub(super) struct PreparedNotice {
    recipient: Principal,
    notice: Notice,
    bytes: Vec<u8>,
    next: u64,
}
impl<M: Memory> PeopleStore<M> {
    pub(super) fn new(m: &MemoryManager<M>) -> Self {
        Self {
            next: StableCell::init(m.get(MemoryId::new(40)), 1),
            inbox: StableBTreeMap::init(m.get(MemoryId::new(41))),
            requests: StableBTreeMap::init(m.get(MemoryId::new(42))),
            contacts: StableBTreeMap::init(m.get(MemoryId::new(43))),
            settings: StableBTreeMap::init(m.get(MemoryId::new(44))),
            rate: StableBTreeMap::init(m.get(MemoryId::new(45))),
        }
    }
    pub(super) fn prepare(&self, recipient: Principal, object: ObjectId, sender: Principal, kind: NoticeKind, at: u64) -> DriveResult<Option<PreparedNotice>> {
        if recipient == sender || (kind != NoticeKind::AccessRequested && !self.settings.get(&recipient).unwrap_or(true)) {
            return Ok(None);
        }
        let id = *self.next.get();
        let next = id.checked_add(1).ok_or(DriveError::Internal)?;
        let notice = Notice { id, object, sender, kind, at, read: false };
        Ok(Some(PreparedNotice { recipient, bytes: encoded(&notice)?, notice, next }))
    }
    pub(super) fn apply(&mut self, prepared: Option<PreparedNotice>) {
        let Some(p) = prepared else { return };
        self.next.set(p.next);
        self.inbox.insert((p.recipient, p.notice.id), p.bytes);
        let keys: Vec<_> = self.inbox.range((p.recipient, 0)..=(p.recipient, u64::MAX)).map(|e| *e.key()).collect();
        for key in keys.iter().take(keys.len().saturating_sub(INBOX_LIMIT)) {
            if let Some(bytes) = self.inbox.remove(key)
                && let Ok(old) = decoded::<Notice>(&bytes)
                && old.kind == NoticeKind::AccessRequested
            {
                // Only the matching request is evicted; a newer request stays intact.
                let rk = (old.object, old.sender);
                if self.requests.get(&rk).is_some_and(|b| decoded::<Request>(&b).is_ok_and(|r| r.at == old.at)) {
                    self.requests.remove(&rk);
                }
            }
        }
    }
}
impl<M: Memory> Store<M> {
    pub(super) fn person(&self, actor: &Actor) -> DriveResult<()> {
        Actor::user(actor.sponsor)?;
        if actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() || !self.accounts.contains_key(&actor.sponsor) {
            return Err(DriveError::Inaccessible);
        }
        Ok(())
    }
    pub(crate) fn drive_contacts(&self, actor: &Actor) -> DriveResult<Vec<Contact>> {
        self.person(actor)?;
        Ok(self
            .people
            .contacts
            .range((actor.sponsor, Principal::from_slice(&[]))..=(actor.sponsor, Principal::from_slice(&[255; 29])))
            .map(|e| Contact { principal: e.key().1, label: e.value() })
            .collect())
    }
    pub(crate) fn drive_contact(&mut self, actor: &Actor, principal: Principal, label: String) -> DriveResult<()> {
        self.person(actor)?;
        let label = label.trim();
        if principal == Principal::anonymous() || label.len() > 100 || label.chars().any(char::is_control) {
            return Err(DriveError::InvalidInput("Invalid contact label".into()));
        }
        let key = (actor.sponsor, principal);
        if label.is_empty() {
            self.people.contacts.remove(&key);
            return Ok(());
        }
        if !self.people.contacts.contains_key(&key) && self.drive_contacts(actor)?.len() >= 200 {
            return Err(DriveError::Limit("Contact limit reached".into()));
        }
        self.people.contacts.insert(key, label.to_owned());
        Ok(())
    }
    pub(crate) fn drive_request_access(&mut self, actor: &Actor, object: ObjectId, message: String, now: u64) -> DriveResult<()> {
        self.person(actor)?;
        let message = message.trim();
        if message.len() > 500 || message.chars().any(|c| c.is_control() && c != '\n') {
            return Err(DriveError::InvalidInput("Keep the request under 500 bytes".into()));
        }
        // Apply the same requester budget to missing, blocked and existing objects.
        // This endpoint never returns the owner, a name, or existence information.
        let (since, used): (u64, u32) = self.people.rate.get(&actor.sponsor).map(|b| decoded(&b)).transpose()?.unwrap_or((now, 0));
        let (since, used) = if now.saturating_sub(since) >= DAY { (now, 0) } else { (since, used) };
        if used >= 10 {
            return Err(DriveError::Limit("Access request daily limit reached".into()));
        }
        let rate = encoded(&(since, used + 1))?;
        let key = (object, actor.sponsor);
        let existing = self.people.requests.get(&key).map(|b| decoded::<Request>(&b)).transpose()?;
        let candidate = self
            .indexed_graph(object)
            .ok()
            .filter(|g| g.ancestry(object, false).is_ok())
            .and_then(|g| if g.effective(actor, object, now, false).is_ok_and(|c| c.read) { None } else { g.objects.get(&object).cloned() });
        let duplicate = existing.is_some_and(|r| (!r.resolved && now.saturating_sub(r.at) < 30 * DAY) || now.saturating_sub(r.at) < 7 * DAY);
        let prepared = if let Some(item) = candidate.filter(|item| !duplicate && !self.safety.blocked(actor.sponsor, item.owner)) {
            let request = encoded(&Request { at: now, message: message.into(), resolved: false })?;
            Some((request, self.people.prepare(self.space_request_manager(item.space, item.owner)?, object, actor.sponsor, NoticeKind::AccessRequested, now)?))
        } else {
            None
        };
        self.people.rate.insert(actor.sponsor, rate);
        if let Some((request, notice)) = prepared {
            self.people.requests.insert(key, request);
            self.people.apply(notice);
        }
        Ok(())
    }
    pub(crate) fn drive_inbox(&self, actor: &Actor, now: u64) -> DriveResult<Inbox> {
        self.person(actor)?;
        let mut items = Vec::new();
        for entry in self.people.inbox.range((actor.sponsor, 0)..=(actor.sponsor, u64::MAX)).rev() {
            let notice: Notice = decoded(&entry.value())?;
            if self.safety.blocked(actor.sponsor, notice.sender) {
                continue;
            }
            let current = self.drive_get(actor, notice.object, now).ok();
            let request = if notice.kind == NoticeKind::AccessRequested {
                self.people.requests.get(&(notice.object, notice.sender)).map(|b| decoded::<Request>(&b)).transpose()?
            } else {
                None
            };
            let owner = current.as_ref().is_some_and(|e| e.space_access.as_ref().is_some_and(|s| s.manage));
            if notice.kind == NoticeKind::AccessRequested && !owner {
                continue;
            }
            // Never retain revoked file names in a notification snapshot.
            if current.is_none() && notice.kind != NoticeKind::AccessDeclined {
                continue;
            }
            let can_respond = owner && request.as_ref().is_some_and(|r| r.at == notice.at && !r.resolved && now.saturating_sub(r.at) < 30 * DAY);
            items.push(InboxItem {
                name: current.map(|e| e.object.name),
                message: request.filter(|r| r.at == notice.at).map(|r| r.message),
                can_respond,
                notice,
            });
        }
        Ok(Inbox { items, notifications: self.people.settings.get(&actor.sponsor).unwrap_or(true) })
    }
    pub(crate) fn drive_read_notice(&mut self, actor: &Actor, id: u64) -> DriveResult<()> {
        self.person(actor)?;
        let key = (actor.sponsor, id);
        if let Some(bytes) = self.people.inbox.get(&key) {
            let mut notice: Notice = decoded(&bytes)?;
            notice.read = true;
            self.people.inbox.insert(key, encoded(&notice)?);
        }
        Ok(())
    }
    pub(crate) fn drive_notifications(&mut self, actor: &Actor, enabled: bool) -> DriveResult<()> {
        self.person(actor)?;
        self.people.settings.insert(actor.sponsor, enabled);
        Ok(())
    }
    pub(crate) fn drive_decide_access(&mut self, actor: &Actor, decision: AccessDecision, now: u64) -> DriveResult<()> {
        self.person(actor)?;
        let entry = self.drive_get(actor, decision.object, now)?;
        if !entry.space_access.as_ref().is_some_and(|s| s.manage) {
            return Err(DriveError::Inaccessible);
        }
        let (key, digest) = retry(actor, "access-decision", &decision.request_id, &decision)?;
        if let Some(bytes) = self.drive_retries.get(&key) {
            let saved: String = decoded(&bytes)?;
            return if saved == digest { Ok(()) } else { Err(DriveError::Conflict) };
        }
        if self.drive_retries.len() >= 19_999 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        let request_key = (decision.object, decision.requester);
        let mut request: Request = decoded(&self.people.requests.get(&request_key).ok_or(DriveError::Inaccessible)?)?;
        if request.at != decision.requested_at || request.resolved || now.saturating_sub(request.at) >= 30 * DAY {
            return Err(DriveError::Conflict);
        }
        request.resolved = true;
        let request_bytes = encoded(&request)?;
        let receipt = encoded(&digest)?;
        let notice =
            if decision.approve { None } else { self.people.prepare(decision.requester, decision.object, actor.sponsor, NoticeKind::AccessDeclined, now)? };
        if decision.approve {
            self.drive_grant(
                actor,
                GrantRequest {
                    id: None,
                    expected_version: None,
                    root: decision.object,
                    subject: Participant::User(decision.requester),
                    capabilities: if decision.editor {
                        Capabilities { propose: false, create: entry.object.kind == ObjectKind::Folder, ..Capabilities::ALL }
                    } else {
                        Capabilities::READ
                    },
                    expires_at: decision.expires_at,
                    active: true,
                    request_id: decision.request_id,
                },
                now,
            )?;
        }
        // No fallible operation after the grant is committed.
        self.people.requests.insert(request_key, request_bytes);
        self.people.apply(notice);
        self.drive_retries.insert(key, receipt);
        Ok(())
    }
    pub(crate) fn drive_notify(&mut self, actor: &Actor, object: ObjectId, recipient: Principal, now: u64) -> DriveResult<()> {
        self.person(actor)?;
        let graph = self.indexed_graph(object)?;
        graph.owner(actor, object)?;
        graph.require(&Actor::user(recipient)?, object, Capabilities::READ, now, false)?;
        let recent = self
            .people
            .inbox
            .range((recipient, 0)..=(recipient, u64::MAX))
            .any(|e| decoded::<Notice>(&e.value()).is_ok_and(|n| n.object == object && n.sender == actor.sponsor && now.saturating_sub(n.at) < DAY / 24));
        if recent {
            return Ok(());
        }
        let notice = self.people.prepare(recipient, object, actor.sponsor, NoticeKind::Reminder, now)?;
        self.people.apply(notice);
        Ok(())
    }
}

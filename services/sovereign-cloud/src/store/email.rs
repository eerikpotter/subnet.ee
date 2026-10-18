//! Private contact bindings and pending invitations. They never replace principal ACLs.
use super::*;
use crate::mail::{DeliveryClaim, MailConfig, MailPayload};
use crate::permissions::Actor;
use sovereign_cloud::{drive::*, email::*};

const MINUTE: u64 = 60_000_000_000;
const DAY: u64 = 1_440 * MINUTE;
const MAX_EMAILS: usize = 10;
#[cfg(test)]
mod tests;

#[derive(Clone, Serialize, serde::Deserialize)]
struct Address {
    owner: Option<Principal>,
    generation: u64,
}
#[derive(Clone, Serialize, serde::Deserialize)]
struct Challenge {
    id: u64,
    email: String,
    request_id: String,
    expires: u64,
    hash: Vec<u8>,
    attempts: u8,
    proved: bool,
    address_generation: u64,
}
#[derive(Clone, Serialize, serde::Deserialize)]
struct Invitation {
    view: EmailInvitation,
    owner: Principal,
    generation: u64,
    recipient: Option<Principal>,
    request_id: String,
}
#[derive(Clone, Serialize, serde::Deserialize)]
struct MailJob {
    owner: Principal,
    payload: Option<MailPayload>,
    created: u64,
    expires: u64,
    status: DeliveryStatus,
    attempt: u8,
    attempted_at: u64,
}
#[derive(Clone, Default, Serialize, serde::Deserialize)]
struct Rate {
    day: u64,
    count: u32,
    last: u64,
}

pub(super) struct EmailStore<M: Memory> {
    contacts: StableBTreeMap<Principal, Vec<u8>, Vm<M>>,
    addresses: StableBTreeMap<String, Vec<u8>, Vm<M>>,
    challenges: StableBTreeMap<Principal, Vec<u8>, Vm<M>>,
    invitations: StableBTreeMap<u64, Vec<u8>, Vm<M>>,
    owners: StableBTreeMap<(Principal, u64), u8, Vm<M>>,
    jobs: StableBTreeMap<u64, Vec<u8>, Vm<M>>,
    rates: StableBTreeMap<String, Vec<u8>, Vm<M>>,
    config: Option<MailConfig>,
    next: StableCell<u64, Vm<M>>,
    cleanup_job: StableCell<u64, Vm<M>>,
    cleanup_rate: StableCell<String, Vm<M>>,
}
impl<M: Memory> EmailStore<M> {
    pub(super) fn new(m: &MemoryManager<M>) -> Self {
        Self {
            contacts: StableBTreeMap::init(m.get(MemoryId::new(26))),
            addresses: StableBTreeMap::init(m.get(MemoryId::new(27))),
            challenges: StableBTreeMap::init(m.get(MemoryId::new(28))),
            invitations: StableBTreeMap::init(m.get(MemoryId::new(29))),
            owners: StableBTreeMap::init(m.get(MemoryId::new(30))),
            jobs: StableBTreeMap::init(m.get(MemoryId::new(31))),
            rates: StableBTreeMap::init(m.get(MemoryId::new(32))),
            config: None,
            next: StableCell::init(m.get(MemoryId::new(33)), 1),
            cleanup_job: StableCell::init(m.get(MemoryId::new(34)), 0),
            cleanup_rate: StableCell::init(m.get(MemoryId::new(35)), String::new()),
        }
    }
    fn config(&self) -> CloudResult<MailConfig> {
        self.config.clone().ok_or_else(|| "Email is not configured".into())
    }
    fn emails(&self, caller: Principal) -> CloudResult<Vec<String>> {
        self.contacts.get(&caller).map(|b| decode(&b)).unwrap_or_else(|| Ok(vec![]))
    }
    fn address(&self, email: &str) -> CloudResult<Address> {
        self.addresses.get(&email.to_string()).map(|b| decode(&b)).unwrap_or_else(|| Ok(Address { owner: None, generation: 0 }))
    }
    fn challenge(&self, caller: Principal) -> CloudResult<Challenge> {
        decode(&self.challenges.get(&caller).ok_or("Verification expired. Request another code.")?)
    }
    fn job(&self, id: u64) -> CloudResult<MailJob> {
        decode(&self.jobs.get(&id).ok_or("Email request not found")?)
    }
    fn status(&self, id: u64, now: u64) -> CloudResult<DeliveryStatus> {
        let job = self.job(id)?;
        Ok(if now >= job.expires && job.status != DeliveryStatus::Accepted { DeliveryStatus::Expired } else { job.status })
    }
    fn allocate(&mut self) -> CloudResult<u64> {
        let id = *self.next.get();
        self.next.set(id.checked_add(1).ok_or("Email request capacity reached")?);
        Ok(id)
    }
    fn charge(&mut self, caller: Principal, email: &str, now: u64, verification: bool) -> CloudResult<()> {
        self.prune(now)?;
        // Fixed global bounds complement per-account limits; authentication alone is not an anti-spam control.
        if self.jobs.len() >= 10_000 || self.rates.len() >= 30_000 {
            return Err("Email capacity reached".into());
        }
        let keys = [("global".to_string(), 1_000), (format!("account:{caller}"), 30), (format!("address:{email}"), 10)];
        let mut updates = Vec::new();
        for (key, limit) in keys {
            let old: Rate = self.rates.get(&key).map(|b| decode(&b)).transpose()?.unwrap_or_default();
            let count = if old.day == now / DAY { old.count } else { 0 };
            if count >= limit || (verification && key.starts_with("account:") && old.count > 0 && now < old.last.saturating_add(MINUTE)) {
                return Err("Email limit reached. Please try later.".into());
            }
            updates.push((key, encode(&Rate { day: now / DAY, count: count + 1, last: now })?));
        }
        for (key, bytes) in updates {
            self.rates.insert(key, bytes);
        }
        Ok(())
    }
    fn prune(&mut self, now: u64) -> CloudResult<()> {
        use std::ops::Bound::{Excluded, Unbounded};
        let jobs: Vec<_> = self.jobs.range((Excluded(*self.cleanup_job.get()), Unbounded)).take(100).map(|e| (*e.key(), e.value())).collect();
        self.cleanup_job.set(jobs.last().map_or(0, |(id, _)| *id));
        for (id, bytes) in jobs {
            let mut job: MailJob = decode(&bytes)?;
            if now >= job.created.saturating_add(8 * DAY) {
                self.jobs.remove(&id);
                if let Some(bytes) = self.invitations.remove(&id) {
                    let invite: Invitation = decode(&bytes)?;
                    self.owners.remove(&(invite.owner, id));
                }
                if self.challenges.get(&job.owner).map(|b| decode::<Challenge>(&b)).transpose()?.is_some_and(|c| c.id == id) {
                    self.challenges.remove(&job.owner);
                }
            } else if now >= job.expires && job.payload.is_some() {
                job.payload = None;
                if job.status != DeliveryStatus::Accepted {
                    job.status = DeliveryStatus::Expired;
                }
                self.jobs.insert(id, encode(&job)?);
            }
        }
        let rates: Vec<_> = self.rates.range((Excluded(self.cleanup_rate.get().clone()), Unbounded)).take(100).map(|e| (e.key().clone(), e.value())).collect();
        self.cleanup_rate.set(rates.last().map_or_else(String::new, |(key, _)| key.clone()));
        for (key, bytes) in rates {
            let rate: Rate = decode(&bytes)?;
            if now / DAY > rate.day.saturating_add(2) {
                self.rates.remove(&key);
            }
        }
        Ok(())
    }
    fn bind(&mut self, caller: Principal, challenge: Challenge, transfer: bool, now: u64) -> CloudResult<EmailVerified> {
        if !challenge.proved || now >= challenge.expires {
            return Err("Verification expired. Request another code.".into());
        }
        let address = self.address(&challenge.email)?;
        if address.owner == Some(caller) {
            self.challenges.remove(&caller);
            return Ok(EmailVerified::Added);
        }
        // A different binding since the code was issued requires a new proof, including an ABA change.
        if address.generation != challenge.address_generation {
            return Err("Email binding changed. Request another code.".into());
        }
        if address.owner.is_some() && !transfer {
            return Ok(EmailVerified::TransferRequired);
        }
        let mut emails = self.emails(caller)?;
        if emails.len() >= MAX_EMAILS {
            return Err("An account can have up to 10 email addresses".into());
        }
        emails.push(challenge.email.clone());
        let contacts = encode(&emails)?;
        let previous = address
            .owner
            .map(|owner| {
                let mut emails = self.emails(owner)?;
                emails.retain(|email| email != &challenge.email);
                Ok::<_, String>((owner, encode(&emails)?))
            })
            .transpose()?;
        let bytes = encode(&Address { owner: Some(caller), generation: address.generation.checked_add(1).ok_or("Email binding capacity reached")? })?;
        if let Some((owner, bytes)) = previous {
            self.contacts.insert(owner, bytes);
        }
        self.contacts.insert(caller, contacts);
        self.addresses.insert(challenge.email, bytes);
        self.challenges.remove(&caller);
        Ok(EmailVerified::Added)
    }
}

/// Conservative ASCII mailbox policy. Never strip plus tags or dots, or infer provider aliases.
fn normalize_email(email: &str) -> CloudResult<String> {
    let email = email.trim();
    if email.len() > 254 || !email.is_ascii() {
        return Err("Enter a supported email address".into());
    }
    let (local, domain) = email.rsplit_once('@').ok_or("Enter a valid email address")?;
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local.bytes().all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&b))
        || !domain.contains('.')
        || domain.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err("Enter a valid email address".into());
    }
    Ok(email.to_ascii_lowercase())
}
fn code_hash(caller: Principal, id: u64, code: &str) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(b"subnet-email-code-v1");
    hash.update(caller.as_slice());
    hash.update(id.to_be_bytes());
    hash.update(code.as_bytes());
    hash.finalize().to_vec()
}
fn inaccessible<T>(_: T) -> String {
    "Item not found or not permitted".into()
}

impl<M: Memory> Store<M> {
    pub(crate) fn email_configure(&mut self, config: Option<MailConfig>) -> CloudResult<()> {
        if let Some(config) = &config {
            config.validate()?;
        }
        self.email.config = config;
        Ok(())
    }
    pub(crate) fn email_profile(&self, caller: Principal, now: u64) -> CloudResult<EmailProfile> {
        authenticated(caller)?;
        let pending = self
            .email
            .challenges
            .get(&caller)
            .map(|b| decode::<Challenge>(&b))
            .transpose()?
            .filter(|c| c.expires > now)
            .map(|c| {
                Ok::<_, String>(EmailVerification {
                    id: c.id,
                    email: c.email,
                    expires_at: c.expires,
                    delivery: self.email.status(c.id, now)?,
                    transfer_required: c.proved,
                })
            })
            .transpose()?;
        Ok(EmailProfile { enabled: self.email.config.is_some(), emails: self.email.emails(caller)?, pending })
    }
    /// Reserve quotas and the caller-bound request before the randomness await.
    pub(crate) fn email_prepare(&mut self, caller: Principal, request: EmailRequest, now: u64) -> CloudResult<(u64, bool)> {
        authenticated(caller)?;
        if self.accounts.get(&caller).is_none() {
            return Err("Open your cloud space first".into());
        }
        self.email.config()?;
        let email = normalize_email(&request.email)?;
        if !(16..=80).contains(&request.request_id.len()) {
            return Err("Invalid email request ID".into());
        }
        if let Ok(old) = self.email.challenge(caller)
            && old.request_id == request.request_id
        {
            return if old.email == email && old.expires > now { Ok((old.id, false)) } else { Err("Email request changed or expired".into()) };
        }
        if self.email.emails(caller)?.contains(&email) {
            return Err("This email is already verified on your account".into());
        }
        if self.email.emails(caller)?.len() >= MAX_EMAILS {
            return Err("An account can have up to 10 email addresses".into());
        }
        if self.email.addresses.len() >= 10_000 && !self.email.addresses.contains_key(&email) {
            return Err("Email capacity reached".into());
        }
        self.email.charge(caller, &email, now, true)?;
        let id = self.email.allocate()?;
        let generation = self.email.address(&email)?.generation;
        self.email.challenges.insert(
            caller,
            encode(&Challenge {
                id,
                email,
                request_id: request.request_id,
                expires: now.saturating_add(10 * MINUTE),
                hash: vec![],
                attempts: 0,
                proved: false,
                address_generation: generation,
            })?,
        );
        self.email.jobs.insert(
            id,
            encode(&MailJob {
                owner: caller,
                payload: None,
                created: now,
                expires: now.saturating_add(10 * MINUTE),
                status: DeliveryStatus::Queued,
                attempt: 0,
                attempted_at: 0,
            })?,
        );
        Ok((id, true))
    }
    pub(crate) fn email_issue(&mut self, caller: Principal, id: u64, random: &[u8], now: u64) -> CloudResult<()> {
        let mut c = self.email.challenge(caller)?;
        if c.id != id || c.expires <= now || !c.hash.is_empty() {
            return Err("Verification request changed or expired".into());
        }
        let bytes: [u8; 8] = random.get(..8).ok_or("Could not generate verification code")?.try_into().map_err(|_| "Could not generate verification code")?;
        let code = format!("{:08}", u64::from_be_bytes(bytes) % 100_000_000);
        c.hash = code_hash(caller, id, &code);
        let config = self.email.config()?;
        let payload = MailPayload {
            from: config.from,
            to: vec![c.email.clone()],
            subject: "Kinnita oma Subneti e-post / Verify your Subnet email".into(),
            text: format!(
                "Sinu Subneti kinnituskood / Your Subnet verification code: {code}\n\nKood kehtib 10 minutit. Ava allolev link, vali „Jätka e-posti kinnitamist” ja sisesta kood samal Subneti kontol, kus kinnitamist alustasid. See ei muuda sinu Internet Identity kontot.\n\nOpen the link below, choose ‘Continue email verification’ and enter the code on the Subnet account where you requested it. It expires in 10 minutes and does not change your Internet Identity. If you did not request this, ignore this message.\n\n{}/pilveruum/#email",
                config.origin
            ),
        };
        let mut job = self.email.job(id)?;
        job.payload = Some(payload);
        let job_bytes = encode(&job)?;
        let challenge_bytes = encode(&c)?;
        self.email.jobs.insert(id, job_bytes);
        self.email.challenges.insert(caller, challenge_bytes);
        Ok(())
    }
    pub(crate) fn email_verify(&mut self, caller: Principal, id: u64, code: String, now: u64) -> CloudResult<EmailVerified> {
        authenticated(caller)?;
        let mut c = self.email.challenge(caller)?;
        if c.id != id || c.expires <= now || c.attempts >= 5 || c.hash.is_empty() || c.proved {
            return Err("Verification expired. Request another code.".into());
        }
        c.attempts += 1;
        let valid = code.len() == 8 && code.bytes().all(|b| b.is_ascii_digit()) && code_hash(caller, id, &code) == c.hash;
        c.proved = valid;
        self.email.challenges.insert(caller, encode(&c)?);
        if !valid {
            return Err("Incorrect verification code".into());
        }
        self.email.bind(caller, c, false, now)
    }
    pub(crate) fn email_confirm_transfer(&mut self, caller: Principal, id: u64, now: u64) -> CloudResult<EmailVerified> {
        authenticated(caller)?;
        let c = self.email.challenge(caller)?;
        if c.id != id {
            return Err("Verification request changed".into());
        }
        self.email.bind(caller, c, true, now)
    }
    pub(crate) fn email_remove(&mut self, caller: Principal, email: String) -> CloudResult<()> {
        authenticated(caller)?;
        let email = normalize_email(&email)?;
        let address = self.email.address(&email)?;
        if address.owner != Some(caller) {
            return Err("Email is not linked to this account".into());
        }
        let mut emails = self.email.emails(caller)?;
        emails.retain(|e| e != &email);
        let contact = encode(&emails)?;
        let address = encode(&Address { owner: None, generation: address.generation.checked_add(1).ok_or("Email binding capacity reached")? })?;
        self.email.contacts.insert(caller, contact);
        self.email.addresses.insert(email, address);
        Ok(())
    }
    pub(crate) fn email_claim_delivery(&mut self, caller: Principal, id: u64, now: u64) -> CloudResult<Option<DeliveryClaim>> {
        authenticated(caller)?;
        let mut job = self.email.job(id)?;
        if job.owner != caller {
            return Err("Email request not found".into());
        }
        if job.status == DeliveryStatus::Accepted {
            return Ok(None);
        }
        if job.expires <= now || now >= job.created.saturating_add(DAY) {
            return Err("Email request expired".into());
        }
        if job.attempt >= 5 || job.status == DeliveryStatus::Failed {
            return Err("Email could not be sent".into());
        }
        if job.attempt > 0 && now < job.attempted_at.saturating_add(2 * MINUTE) {
            return Err("Email is being sent. Please wait before retrying.".into());
        }
        if let Some(c) = self.email.challenges.get(&caller).map(|b| decode::<Challenge>(&b)).transpose()? {
            if c.id != id && !self.email.invitations.contains_key(&id) {
                return Err("Verification request changed".into());
            }
        } else if !self.email.invitations.contains_key(&id) {
            return Err("Verification request expired".into());
        }
        if let Some(invite) = self.email.invitations.get(&id).map(|b| decode::<Invitation>(&b)).transpose()? {
            if !invite.view.active {
                return Err("Invitation is no longer active".into());
            }
            self.email_owner(caller, invite.view.object, now)?;
        }
        let payload = serde_json::to_vec(job.payload.as_ref().ok_or("Email is still being prepared")?).map_err(|_| "Invalid email payload")?;
        let key = self.email.config()?.api_key;
        job.attempt += 1;
        job.attempted_at = now;
        job.status = DeliveryStatus::Sending;
        let result = DeliveryClaim { attempt: job.attempt, key, payload, idempotency: format!("{}-{}", id, job.created) };
        self.email.jobs.insert(id, encode(&job)?);
        Ok(Some(result))
    }
    pub(crate) fn email_finish_delivery(&mut self, id: u64, attempt: u8, status: DeliveryStatus) -> CloudResult<()> {
        let mut job = self.email.job(id)?;
        if job.attempt == attempt && job.status == DeliveryStatus::Sending {
            if status == DeliveryStatus::Accepted || status == DeliveryStatus::Failed {
                job.payload = None;
            }
            job.status = status;
            self.email.jobs.insert(id, encode(&job)?);
        }
        Ok(())
    }
    fn email_owner(&self, caller: Principal, object: ObjectId, now: u64) -> CloudResult<DriveEntry> {
        let actor = Actor::user(caller).map_err(inaccessible)?;
        let graph = self.graph_for_object(object).map_err(inaccessible)?;
        graph.owner(&actor, object).map_err(inaccessible)?;
        self.entry(&graph, &actor, object, now).map_err(inaccessible)
    }
    fn email_grant(&mut self, owner: Principal, recipient: Principal, view: &EmailInvitation, now: u64) -> CloudResult<()> {
        let entry = self.email_owner(owner, view.object, now)?;
        let actor = Actor::user(owner).map_err(inaccessible)?;
        let existing = self.drive_grants(&actor, view.object).map_err(inaccessible)?.into_iter().find(|grant| {
            grant.root == view.object && grant.subject == Participant::User(recipient) && grant.active && grant.expires_at.is_none_or(|expiry| expiry > now)
        });
        let editor = view.role == EmailRole::Editor;
        self.drive_grant(
            &actor,
            GrantRequest {
                request_id: format!("email-invitation-{}", view.id),
                id: existing.as_ref().map(|grant| grant.id),
                expected_version: existing.map(|grant| grant.version),
                root: view.object,
                subject: Participant::User(recipient),
                active: true,
                expires_at: view.grant_expires_at,
                capabilities: Capabilities {
                    read: true,
                    create: editor && entry.object.kind == ObjectKind::Folder,
                    edit: editor,
                    delete: editor,
                    propose: false,
                },
            },
            now,
        )
        .map_err(inaccessible)?;
        Ok(())
    }
    pub(crate) fn email_share(&mut self, caller: Principal, request: EmailShare, now: u64) -> CloudResult<EmailInvitation> {
        let entry = self.email_owner(caller, request.object, now)?;
        if self.email.emails(caller)?.is_empty() {
            return Err("Verify an email address before sending invitations".into());
        }
        let email = normalize_email(&request.email)?;
        if !(16..=80).contains(&request.request_id.len()) {
            return Err("Invalid email request ID".into());
        }
        self.email.prune(now)?;
        let ids: Vec<_> = self.email.owners.range((caller, 0)..=(caller, u64::MAX)).map(|e| e.key().1).collect();
        for id in &ids {
            let invite: Invitation = decode(&self.email.invitations.get(id).ok_or("Invitation not found")?)?;
            if invite.request_id == request.request_id {
                if invite.view.email != email
                    || invite.view.object != request.object
                    || invite.view.role != request.role
                    || invite.view.grant_expires_at != request.grant_expires_at
                {
                    return Err("Email request changed".into());
                }
                return self.email_invitation_owner(caller, *id, now);
            }
        }
        if ids.len() >= 300 || self.email.invitations.len() >= 10_000 {
            return Err("Invitation capacity reached".into());
        }
        if request.grant_expires_at.is_some_and(|at| at <= now) {
            return Err("Access expiry must be in the future".into());
        }
        let config = self.email.config()?;
        let address = self.email.address(&email)?;
        self.email.charge(caller, &email, now, false)?;
        let id = self.email.allocate()?;
        let view = EmailInvitation {
            grant_expires_at: request.grant_expires_at,
            id,
            object: request.object,
            email: email.clone(),
            role: request.role,
            expires_at: now.saturating_add(7 * DAY),
            active: true,
            accepted: address.owner.is_some(),
            delivery: DeliveryStatus::Queued,
        };
        let route = if address.owner.is_some() {
            format!("{}/{}", if entry.object.kind == ObjectKind::Folder { "folder" } else { "file" }, request.object)
        } else {
            format!("#invite/{id}")
        };
        let payload = MailPayload {
            from: config.from,
            to: vec![email],
            subject: "Sinuga jagati Subnetis / Shared with you on Subnet".into(),
            text: format!(
                "Sinuga jagati Subneti pilveruumis faili või kausta.\nA file or folder was shared with you on Subnet.\n\n{}/pilveruum/{route}\n\nLogi sisse Internet Identityga. Link ise ligipääsu ei anna.\nSign in with Internet Identity. Forwarding this link does not grant access.",
                config.origin
            ),
        };
        let invite_bytes = encode(&Invitation {
            view: view.clone(),
            owner: caller,
            generation: address.generation,
            recipient: address.owner,
            request_id: request.request_id,
        })?;
        let job_bytes = encode(&MailJob {
            owner: caller,
            payload: Some(payload),
            created: now,
            expires: now.saturating_add(DAY),
            status: DeliveryStatus::Queued,
            attempt: 0,
            attempted_at: 0,
        })?;
        if let Some(recipient) = address.owner {
            self.email_grant(caller, recipient, &view, now)?;
        }
        self.email.invitations.insert(id, invite_bytes);
        self.email.owners.insert((caller, id), 0);
        self.email.jobs.insert(id, job_bytes);
        Ok(view)
    }
    pub(crate) fn email_invitation_owner(&self, caller: Principal, id: u64, now: u64) -> CloudResult<EmailInvitation> {
        let invite: Invitation = decode(&self.email.invitations.get(&id).ok_or("Invitation not found")?)?;
        if invite.owner != caller {
            return Err("Invitation not found".into());
        }
        let mut view = invite.view;
        view.delivery = self.email.status(id, now)?;
        Ok(view)
    }
    pub(crate) fn email_invitations(&self, caller: Principal, object: ObjectId, now: u64) -> CloudResult<Vec<EmailInvitation>> {
        self.email_owner(caller, object, now)?;
        self.email
            .owners
            .range((caller, 0)..=(caller, u64::MAX))
            .map(|e| self.email_invitation_owner(caller, e.key().1, now))
            .filter(|r| r.as_ref().map_or(true, |i| i.object == object))
            .collect()
    }
    pub(crate) fn email_revoke_invitation(&mut self, caller: Principal, id: u64, now: u64) -> CloudResult<()> {
        let mut invite: Invitation = decode(&self.email.invitations.get(&id).ok_or("Invitation not found")?)?;
        if invite.owner != caller {
            return Err("Invitation not found".into());
        }
        self.email_owner(caller, invite.view.object, now)?;
        if invite.view.accepted {
            return Err("Remove accepted access through file sharing".into());
        }
        invite.view.active = false;
        self.email.invitations.insert(id, encode(&invite)?);
        Ok(())
    }
    pub(crate) fn email_accept(&mut self, caller: Principal, id: u64, now: u64) -> CloudResult<ObjectId> {
        authenticated(caller)?;
        let mut invite: Invitation = decode(&self.email.invitations.get(&id).ok_or("Invitation is not available for this account")?)?;
        if invite.view.accepted {
            if invite.recipient != Some(caller) {
                return Err("Invitation is not available for this account".into());
            }
            self.drive_get(&Actor::user(caller).map_err(inaccessible)?, invite.view.object, now).map_err(inaccessible)?;
            return Ok(invite.view.object);
        }
        let address = self.email.address(&invite.view.email)?;
        if !invite.view.active || now >= invite.view.expires_at || address.owner != Some(caller) || address.generation != invite.generation.saturating_add(1) {
            return Err("Invitation is not available for this account. Verify the invited email or request a new invitation.".into());
        }
        invite.view.accepted = true;
        invite.recipient = Some(caller);
        let bytes = encode(&invite)?;
        self.email_grant(invite.owner, caller, &invite.view, now)?;
        self.email.invitations.insert(id, bytes);
        Ok(invite.view.object)
    }
}

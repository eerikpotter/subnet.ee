//! Recipient-controlled visibility and pairwise block policy, separate from grants.
use super::collaboration::{decoded, encoded};
use super::*;
use crate::permissions::Actor;
use sovereign_cloud::{drive::*, people::*};
pub(super) struct SafetyStore<M: Memory> {
    blocks: StableBTreeMap<(Principal, Principal), bool, Vm<M>>,
    hidden: StableBTreeMap<(Principal, ObjectId), bool, Vm<M>>,
    reports: StableBTreeMap<(Principal, ObjectId), Vec<u8>, Vm<M>>,
}
impl<M: Memory> SafetyStore<M> {
    pub(super) fn new(m: &MemoryManager<M>) -> Self {
        Self {
            blocks: StableBTreeMap::init(m.get(MemoryId::new(46))),
            hidden: StableBTreeMap::init(m.get(MemoryId::new(47))),
            reports: StableBTreeMap::init(m.get(MemoryId::new(48))),
        }
    }
    pub(super) fn blocked(&self, a: Principal, b: Principal) -> bool {
        a != b && (self.blocks.contains_key(&(a, b)) || self.blocks.contains_key(&(b, a)))
    }
    pub(super) fn hidden(&self, viewer: Principal, path: &[ObjectId]) -> bool {
        path.iter().any(|id| self.hidden.contains_key(&(viewer, *id)))
    }
}
impl<M: Memory> Store<M> {
    pub(crate) fn drive_sharing_safety(&self, actor: &Actor, now: u64) -> DriveResult<SharingSafety> {
        self.person(actor)?;
        let blocked = self
            .safety
            .blocks
            .range((actor.sponsor, Principal::from_slice(&[]))..=(actor.sponsor, Principal::from_slice(&[255; 29])))
            .map(|e| e.key().1)
            .collect();
        let hidden = self
            .safety
            .hidden
            .range((actor.sponsor, ObjectId::MIN)..=(actor.sponsor, ObjectId::MAX))
            .map(|e| HiddenItem { object: e.key().1, name: self.drive_get(actor, e.key().1, now).ok().map(|i| i.object.name) })
            .collect();
        Ok(SharingSafety { blocked, hidden })
    }
    pub(crate) fn drive_block(&mut self, actor: &Actor, other: Principal, blocked: bool) -> DriveResult<()> {
        self.person(actor)?;
        if other == actor.sponsor || other == Principal::anonymous() {
            return Err(DriveError::InvalidInput("Choose another account".into()));
        }
        let key = (actor.sponsor, other);
        if self.safety.blocks.contains_key(&key) == blocked {
            return Ok(());
        }
        if blocked && self.safety.blocks.range((actor.sponsor, Principal::from_slice(&[]))..=(actor.sponsor, Principal::from_slice(&[255; 29]))).count() >= 200
        {
            return Err(DriveError::Limit("Blocked account limit reached".into()));
        }
        let version = self.policy_versions.get(&actor.sponsor).unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        let other_version = self.policy_versions.get(&other).unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        if blocked {
            self.safety.blocks.insert(key, true);
        } else {
            self.safety.blocks.remove(&key);
        }
        self.policy_versions.insert(actor.sponsor, version);
        self.policy_versions.insert(other, other_version);
        Ok(())
    }
    pub(crate) fn drive_hide_shared(&mut self, actor: &Actor, object: ObjectId, hidden: bool, now: u64) -> DriveResult<()> {
        self.person(actor)?;
        let key = (actor.sponsor, object);
        if self.safety.hidden.contains_key(&key) == hidden {
            return Ok(());
        }
        if hidden {
            let entry = self.drive_get(actor, object, now)?;
            if entry.object.owner == actor.sponsor {
                return Err(DriveError::InvalidInput("Only shared items can be hidden".into()));
            }
            if self.safety.hidden.range((actor.sponsor, ObjectId::MIN)..=(actor.sponsor, ObjectId::MAX)).count() >= 200 {
                return Err(DriveError::Limit("Hidden item limit reached".into()));
            }
        }
        let version = self.policy_versions.get(&actor.sponsor).unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        if hidden {
            self.safety.hidden.insert(key, true);
        } else {
            self.safety.hidden.remove(&key);
        }
        self.policy_versions.insert(actor.sponsor, version);
        Ok(())
    }
    pub(crate) fn drive_report(&mut self, actor: &Actor, object: ObjectId, reason: String, now: u64) -> DriveResult<()> {
        self.person(actor)?;
        let item = self.drive_get(actor, object, now)?;
        let reason = reason.trim();
        if item.object.owner == actor.sponsor || reason.is_empty() || reason.len() > 500 || reason.chars().any(char::is_control) {
            return Err(DriveError::InvalidInput("Enter a report of up to 500 bytes".into()));
        }
        let key = (actor.sponsor, object);
        if !self.safety.reports.contains_key(&key) && self.safety.reports.range((actor.sponsor, ObjectId::MIN)..=(actor.sponsor, ObjectId::MAX)).count() >= 100
        {
            return Err(DriveError::Limit("Report limit reached".into()));
        }
        let report = SharingReport { reporter: actor.sponsor, object, owner: item.object.owner, reason: reason.into(), at: now };
        self.safety.reports.insert(key, encoded(&report)?);
        Ok(())
    }
    /// The canister entry point checks controller authority before invoking this.
    pub(crate) fn sharing_reports(&self, after: Option<(Principal, ObjectId)>) -> DriveResult<Vec<SharingReport>> {
        use std::ops::Bound::{Excluded, Unbounded};
        self.safety.reports.range((after.map_or(Unbounded, Excluded), Unbounded)).take(50).map(|e| decoded(&e.value())).collect()
    }
}

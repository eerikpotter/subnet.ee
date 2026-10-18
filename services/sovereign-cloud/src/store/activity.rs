//! Bounded mutation facts. This is not an authority, a snapshot, or a read log.
use super::collaboration::{decoded, encoded};
use super::*;
use crate::permissions::Actor;
use sovereign_cloud::drive::*;
use std::ops::Bound::{Excluded, Included};

const PER_OBJECT: usize = 100;
pub(super) struct ActivityStore<M: Memory> {
    next: StableCell<u64, Vm<M>>,
    records: StableBTreeMap<(Principal, u64), Vec<u8>, Vm<M>>,
    objects: StableBTreeMap<(ObjectId, u64), Principal, Vm<M>>,
}
pub(super) struct PreparedActivity {
    owner: Principal,
    event: ActivityEvent,
    bytes: Vec<u8>,
    next: u64,
}
impl<M: Memory> ActivityStore<M> {
    pub(super) fn new(m: &MemoryManager<M>) -> Self {
        Self {
            next: StableCell::init(m.get(MemoryId::new(36)), 1),
            records: StableBTreeMap::init(m.get(MemoryId::new(37))),
            objects: StableBTreeMap::init(m.get(MemoryId::new(38))),
        }
    }
    pub(super) fn prepare(&self, owner: Principal, object: ObjectId, actor: &Actor, kind: ActivityKind, at: u64) -> DriveResult<PreparedActivity> {
        let id = *self.next.get();
        let next = id.checked_add(1).ok_or(DriveError::Internal)?;
        let event = ActivityEvent { id, object, actor: actor.subject.clone(), kind, at };
        Ok(PreparedActivity { bytes: encoded(&event)?, owner, event, next })
    }
    pub(super) fn apply(&mut self, prepared: PreparedActivity) {
        let PreparedActivity { owner, event, bytes, next } = prepared;
        self.records.insert((owner, event.id), bytes);
        self.objects.insert((event.object, event.id), owner);
        self.next.set(next);
        // One event is published per successful operation. At most one old event
        // normally expires; a bounded per-object index makes cleanup independent
        // of the number of accounts or events elsewhere.
        let keys: Vec<_> = self.objects.range((event.object, 0)..=(event.object, u64::MAX)).map(|e| *e.key()).collect();
        for key in keys.iter().take(keys.len().saturating_sub(PER_OBJECT)) {
            self.objects.remove(key);
            self.records.remove(&(owner, key.1));
        }
    }
    pub(super) fn remove_object(&mut self, object: ObjectId) {
        let keys: Vec<_> = self.objects.range((object, 0)..=(object, u64::MAX)).map(|e| (*e.key(), e.value())).collect();
        for (key, owner) in keys {
            self.objects.remove(&key);
            self.records.remove(&(owner, key.1));
        }
    }
}
impl<M: Memory> Store<M> {
    /// None is the caller's own space. A file/folder scope can also be shared;
    /// it is exact-object history, not a recursive history of hidden descendants.
    pub(crate) fn drive_activity(&self, actor: &Actor, object: Option<ObjectId>, before: Option<u64>, limit: u32, now: u64) -> DriveResult<ActivityPage> {
        Actor::user(actor.sponsor)?;
        let (owner, mut graph) = if let Some(id) = object {
            let graph = self.indexed_graph(id)?;
            graph.require(actor, id, Capabilities::READ, now, false)?;
            (graph.objects.get(&id).ok_or(DriveError::Inaccessible)?.owner, graph)
        } else {
            // Owning-account feed does not enumerate other accounts' events.
            if actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() {
                return Err(DriveError::Inaccessible);
            }
            (actor.sponsor, self.read_graph())
        };
        let end = before.unwrap_or(u64::MAX);
        let keys: Vec<_> = if let Some(object) = object {
            self.activity.objects.range((Included((object, 0)), Excluded((object, end)))).rev().take(200).map(|e| e.key().1).collect()
        } else {
            self.activity.records.range((Included((owner, 0)), Excluded((owner, end)))).rev().take(200).map(|e| e.key().1).collect()
        };
        let limit = limit.clamp(1, 50) as usize;
        let mut items = Vec::new();
        let mut scanned = None;
        for (index, id) in keys.iter().enumerate() {
            scanned = Some(*id);
            let event: ActivityEvent = decoded(&self.activity.records.get(&(owner, *id)).ok_or(DriveError::Internal)?)?;
            if self.read_path(&mut graph, event.object).is_err() {
                continue;
            }
            let Some(current) = graph.objects.get(&event.object) else { continue };
            let owner_only = matches!(event.kind, ActivityKind::SharingChanged | ActivityKind::RevisionDeleted | ActivityKind::RevisionLabeled);
            if owner_only && current.owner != actor.sponsor {
                continue;
            }
            let trashed = graph.ancestry(current.id, false).is_err();
            // Trash activity is visible only to its owner; shared users revalidate
            // ordinary live Read authority before each page.
            if trashed && current.owner != actor.sponsor {
                continue;
            }
            if graph.require(actor, current.id, Capabilities::READ, now, trashed).is_err() {
                continue;
            }
            items.push(ActivityItem { event, name: current.name.clone(), trashed });
            if items.len() == limit {
                return Ok(ActivityPage { items, before: (index + 1 < keys.len() || keys.len() == 200).then_some(*id) });
            }
        }
        Ok(ActivityPage { items, before: (keys.len() == 200).then_some(scanned).flatten() })
    }
}

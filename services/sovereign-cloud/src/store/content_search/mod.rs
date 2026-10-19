//! Rebuildable per-space inverted index. Queries never read file bytes.
use super::collaboration::{decoded, encoded};
use super::*;
use crate::permissions::Actor;
use ic_stable_structures::Storable;
use sovereign_cloud::{drive::*, search::*};
use sovereign_cloud_text::{EXTRACTOR_VERSION, Extraction, MAX_INPUT, Outcome, Section};
use std::{
    borrow::Cow,
    collections::BTreeSet,
    ops::Bound::{Excluded, Included},
};
mod query;
#[cfg(test)]
mod tests;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
struct Posting(ObjectId, String, ObjectId);
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
struct TermKey(ObjectId, String);
impl Storable for TermKey {
    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Unbounded;
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        serde_json::to_vec(self).unwrap_or_else(|_| ic_cdk::trap("Invalid term key")).into()
    }
    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| ic_cdk::trap("Invalid term key"))
    }
}
impl Storable for Posting {
    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Unbounded;
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        serde_json::to_vec(self).unwrap_or_else(|_| ic_cdk::trap("Invalid search key")).into()
    }
    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| ic_cdk::trap("Invalid search key"))
    }
}
#[derive(Serialize, serde::Deserialize)]
struct Record {
    space: ObjectId,
    revision: u64,
    sha256: Vec<u8>,
    extractor: u32,
    state: IndexState,
    sections: u32,
    retry_after: u64,
}
pub(super) struct SearchStore<M: Memory> {
    records: StableBTreeMap<ObjectId, Vec<u8>, Vm<M>>,
    sections: StableBTreeMap<(ObjectId, u32), Vec<u8>, Vm<M>>,
    postings: StableBTreeMap<Posting, (u64, u32), Vm<M>>,
    reverse: StableBTreeMap<TermKey, ObjectId, Vm<M>>,
    counts: StableBTreeMap<TermKey, u64, Vm<M>>,
    queue: StableBTreeMap<(u64, ObjectId), u64, Vm<M>>,
    pending: StableBTreeMap<ObjectId, u64, Vm<M>>,
    backfill: StableCell<Vec<u8>, Vm<M>>,
}
impl<M: Memory> SearchStore<M> {
    pub(super) fn new(m: &MemoryManager<M>) -> Self {
        Self {
            records: StableBTreeMap::init(m.get(MemoryId::new(69))),
            sections: StableBTreeMap::init(m.get(MemoryId::new(70))),
            postings: StableBTreeMap::init(m.get(MemoryId::new(71))),
            reverse: StableBTreeMap::init(m.get(MemoryId::new(72))),
            counts: StableBTreeMap::init(m.get(MemoryId::new(73))),
            queue: StableBTreeMap::init(m.get(MemoryId::new(74))),
            pending: StableBTreeMap::init(m.get(MemoryId::new(75))),
            backfill: StableCell::init(m.get(MemoryId::new(76)), Vec::new()),
        }
    }
    pub(super) fn enqueue(&mut self, file: ObjectId, revision: u64) {
        if self.pending.get(&file) == Some(revision) {
            return;
        }
        if let Some(old) = self.pending.insert(file, revision) {
            self.queue.remove(&(old, file));
        }
        self.queue.insert((revision, file), 0);
    }
    fn remove_index(&mut self, file: ObjectId) {
        let terms: Vec<_> =
            self.reverse.range(TermKey(file, String::new())..=TermKey(file, "\u{10ffff}".into())).map(|e| (e.key().clone(), e.value())).collect();
        for (TermKey(_, term), space) in terms {
            self.postings.remove(&Posting(space, term.clone(), file));
            let key = TermKey(space, term.clone());
            let count = self.counts.get(&key).unwrap_or(1).saturating_sub(1);
            if count == 0 {
                self.counts.remove(&key);
            } else {
                self.counts.insert(key, count);
            }
            self.reverse.remove(&TermKey(file, term));
        }
        let sections: Vec<_> = self.sections.range((file, 0)..=(file, u32::MAX)).map(|e| e.key().to_owned()).collect();
        for key in sections {
            self.sections.remove(&key);
        }
        self.records.remove(&file);
    }
    pub(super) fn forget(&mut self, file: ObjectId) {
        self.remove_index(file);
        if let Some(revision) = self.pending.remove(&file) {
            self.queue.remove(&(revision, file));
        }
    }
}
impl<M: Memory> Store<M> {
    fn content_head(&self, file: ObjectId) -> DriveResult<(DriveObject, Revision)> {
        let object: DriveObject = decoded(&self.read_objects.get(&file).ok_or(DriveError::Inaccessible)?)?;
        let project: Project = decoded(&self.projects.get(&file).ok_or(DriveError::Inaccessible)?)?;
        Ok((object, project.revisions.last().ok_or(DriveError::Inaccessible)?.clone()))
    }
    pub(crate) fn content_status(&self, actor: &Actor, file: ObjectId, now: u64) -> DriveResult<ContentStatus> {
        self.drive_get(actor, file, now)?;
        let (_, head) = self.content_head(file)?;
        let record: Option<Record> = self.search.records.get(&file).map(|b| decoded(&b)).transpose()?;
        let current = record.filter(|r| r.revision == head.id && r.extractor == EXTRACTOR_VERSION);
        let pending = self.search.pending.get(&file) == Some(head.id);
        Ok(ContentStatus {
            object: file,
            revision: head.id,
            extractor_version: EXTRACTOR_VERSION,
            state: if pending { IndexState::Pending } else { current.as_ref().map(|r| r.state.clone()).unwrap_or(IndexState::Pending) },
            sections: current.map_or(0, |r| r.sections),
        })
    }
    pub(crate) fn content_rebuild(&mut self, actor: &Actor, file: ObjectId, now: u64) -> DriveResult<ContentStatus> {
        let graph = self.indexed_graph(file)?;
        graph.require(actor, file, Capabilities { edit: true, ..Capabilities::READ }, now, false)?;
        let (_, head) = self.content_head(file)?;
        let status = self.content_status(actor, file, now)?;
        // Ready current records need no costly repeat. New extractor versions rebuild incrementally.
        if status.state == IndexState::Failed {
            let record: Record = decoded(&self.search.records.get(&file).ok_or(DriveError::Internal)?)?;
            if now < record.retry_after {
                return Err(DriveError::Limit("Retry content indexing later".into()));
            }
        }
        if matches!(status.state, IndexState::Pending | IndexState::Failed) {
            self.search.enqueue(file, head.id);
        }
        self.content_status(actor, file, now)
    }
    /// One bounded deployment backfill batch, independent of any account's listing.
    pub(crate) fn content_backfill(&mut self) -> DriveResult<()> {
        let (version, after, done): (u32, ObjectId, bool) =
            if self.search.backfill.get().is_empty() { (EXTRACTOR_VERSION, ObjectId::MIN, false) } else { decoded(self.search.backfill.get())? };
        let (after, done) = if version == EXTRACTOR_VERSION { (after, done) } else { (ObjectId::MIN, false) };
        if done {
            return Ok(());
        }
        let ids: Vec<_> = self.read_objects.range((Excluded(after), Included(ObjectId::MAX))).take(16).map(|e| e.key().to_owned()).collect();
        let last = ids.last().copied().unwrap_or(after);
        let done = ids.len() < 16;
        for id in ids {
            if let Ok((_, head)) = self.content_head(id) {
                let record: Option<Record> = self.search.records.get(&id).map(|b| decoded(&b)).transpose()?;
                if record.is_none_or(|r| r.revision != head.id || r.extractor != EXTRACTOR_VERSION) {
                    self.search.enqueue(id, head.id);
                }
            }
        }
        self.search.backfill.set(encoded(&(EXTRACTOR_VERSION, last, done))?);
        Ok(())
    }
    /// A persistent lease bounds duplicate work across retries/upgrades.
    pub(crate) fn content_claim(&mut self, now: u64) -> Option<(ObjectId, u64)> {
        let entry = self.search.queue.first_key_value()?;
        let (revision, file) = entry.0;
        if entry.1 > now {
            return None;
        }
        self.search.queue.insert((revision, file), now.saturating_add(300_000_000_000));
        Some((file, revision))
    }
    pub(crate) fn content_failed(&mut self, file: ObjectId, revision: u64) -> DriveResult<()> {
        if self.search.pending.get(&file) != Some(revision) {
            return Ok(());
        }
        let Ok((object, head)) = self.content_head(file) else {
            self.search.forget(file);
            return Ok(());
        };
        if head.id != revision {
            return Ok(());
        }
        self.content_publish(object, head, Extraction { outcome: Outcome::Unreadable, sections: vec![] }, Some(IndexState::Failed))
    }
    /// Invoked only by the canister itself. Source bytes never leave the canister.
    pub(crate) fn content_process(&mut self, file: ObjectId, revision: u64) -> DriveResult<()> {
        if self.search.pending.get(&file) != Some(revision) {
            return Ok(());
        }
        let Ok((object, head)) = self.content_head(file) else {
            self.search.forget(file);
            return Ok(());
        };
        if head.id != revision {
            return Ok(());
        }
        let ObjectKind::File(format) = &object.kind else {
            return Err(DriveError::Inaccessible);
        };
        let extraction = if head.bytes > MAX_INPUT as u64 {
            Extraction { outcome: Outcome::TooLarge, sections: vec![] }
        } else {
            let mut bytes = Vec::with_capacity(head.bytes as usize);
            for index in 0..head.chunk_count {
                bytes.extend(self.chunks.get(&(head.id, index)).ok_or(DriveError::Internal)?);
            }
            if bytes.len() != head.bytes as usize || Sha256::digest(&bytes).to_vec() != head.sha256 {
                return Err(DriveError::Internal);
            }
            sovereign_cloud_text::extract(&object.name, &format.media_type, format.craft.as_ref().map(|k| k.format_id.as_str()), &bytes)
        };
        self.content_publish(object, head, extraction, None)
    }
    fn content_publish(&mut self, object: DriveObject, head: Revision, extraction: Extraction, failure: Option<IndexState>) -> DriveResult<()> {
        // All parsing/encoding completes before publishing the derived record atomically.
        let file = object.id;
        let (terms, truncated) = sovereign_cloud_text::terms(&extraction.sections);
        let mut state = match extraction.outcome {
            Outcome::Ready => IndexState::Ready,
            Outcome::Empty => IndexState::Empty,
            Outcome::Partial => IndexState::Partial,
            Outcome::Unsupported => IndexState::Unsupported,
            Outcome::TooLarge => IndexState::TooLarge,
            Outcome::Unreadable => IndexState::Unreadable,
        };
        if truncated {
            state = IndexState::Partial;
        }
        if let Some(f) = failure {
            state = f;
        }
        let record = encoded(&Record {
            space: object.space,
            revision: head.id,
            sha256: head.sha256,
            extractor: EXTRACTOR_VERSION,
            state,
            sections: extraction.sections.len() as u32,
            retry_after: self.search.queue.get(&(head.id, file)).unwrap_or(0),
        })?;
        let sections = extraction.sections.iter().map(encoded).collect::<DriveResult<Vec<_>>>()?;
        self.search.remove_index(file);
        for (term, section) in terms {
            let key = TermKey(object.space, term.clone());
            let count = self.search.counts.get(&key).unwrap_or(0) + 1;
            self.search.counts.insert(key, count);
            self.search.reverse.insert(TermKey(file, term.clone()), object.space);
            self.search.postings.insert(Posting(object.space, term, file), (head.id, section));
        }
        for (i, section) in sections.into_iter().enumerate() {
            self.search.sections.insert((file, i as u32), section);
        }
        self.search.records.insert(file, record);
        self.search.queue.remove(&(head.id, file));
        self.search.pending.remove(&file);
        Ok(())
    }
}

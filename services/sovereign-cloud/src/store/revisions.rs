//! Revision annotations are independent of immutable content and upload receipts.
use super::collaboration::{decoded, encoded, retry};
use super::*;
use crate::permissions::Actor;
use sovereign_cloud::drive::*;

pub(super) struct RevisionStore<M: Memory> {
    properties: StableBTreeMap<(ObjectId, u64), Vec<u8>, Vm<M>>,
}
impl<M: Memory> RevisionStore<M> {
    pub(super) fn new(m: &MemoryManager<M>) -> Self {
        Self { properties: StableBTreeMap::init(m.get(MemoryId::new(39))) }
    }
    pub(super) fn get(&self, object: ObjectId, revision: u64) -> DriveResult<RevisionProperties> {
        self.properties.get(&(object, revision)).map(|b| decoded(&b)).transpose().map(Option::unwrap_or_default)
    }
    pub(super) fn remove(&mut self, object: ObjectId, revision: u64) {
        self.properties.remove(&(object, revision));
    }
    pub(super) fn remove_object(&mut self, object: ObjectId) {
        let keys: Vec<_> = self.properties.range((object, 0)..=(object, u64::MAX)).map(|e| *e.key()).collect();
        for key in keys {
            self.properties.remove(&key);
        }
    }
}
impl<M: Memory> Store<M> {
    pub(crate) fn drive_versions(&self, actor: &Actor, object: ObjectId, now: u64) -> DriveResult<Vec<RevisionInfo>> {
        self.drive_history(actor, object, now)?
            .into_iter()
            .map(|revision| Ok(RevisionInfo { properties: self.revisions.get(object, revision.id)?, revision }))
            .collect()
    }
    pub(crate) fn drive_label_revision(&mut self, actor: &Actor, request: RevisionLabelRequest, now: u64) -> DriveResult<RevisionProperties> {
        let graph = self.indexed_graph(request.object)?;
        graph.owner(actor, request.object)?;
        graph.require(actor, request.object, Capabilities::READ, now, false)?;
        let (key, hash) = retry(actor, "revision-label", &request.request_id, &request)?;
        if let Some(bytes) = self.drive_retries.get(&key) {
            let (saved, result): (String, RevisionProperties) = decoded(&bytes)?;
            return if saved == hash { Ok(result) } else { Err(DriveError::Conflict) };
        }
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        let project: Project = decoded(&self.projects.get(&request.object).ok_or(DriveError::Inaccessible)?)?;
        if !project.revisions.iter().any(|r| r.id == request.revision) {
            return Err(DriveError::Inaccessible);
        }
        let mut properties = self.revisions.get(request.object, request.revision)?;
        if properties.version != request.expected_version {
            return Err(DriveError::Conflict);
        }
        let label = request.label.trim();
        if label.len() > 160 || label.chars().any(char::is_control) {
            return Err(DriveError::InvalidInput("Version name must be at most 160 bytes".into()));
        }
        properties.version = properties.version.checked_add(1).ok_or(DriveError::Internal)?;
        properties.label = label.to_string();
        properties.keep = request.keep || !label.is_empty();
        let bytes = encoded(&properties)?;
        let receipt = encoded(&(hash, &properties))?;
        let event = self.activity.prepare(project.owner, project.id, actor, ActivityKind::RevisionLabeled, now)?;
        self.revisions.properties.insert((request.object, request.revision), bytes);
        self.drive_retries.insert(key, receipt);
        self.activity.apply(event);
        Ok(properties)
    }
    pub(crate) fn drive_delete_revision(&mut self, actor: &Actor, request: RevisionDeleteRequest, now: u64) -> DriveResult<MutationReceipt> {
        let mut graph = self.graph_for_object(request.object)?;
        graph.owner(actor, request.object)?;
        graph.require(actor, request.object, Capabilities::READ, now, false)?;
        let (key, hash) = retry(actor, "revision-delete", &request.request_id, &request)?;
        if let Some(bytes) = self.drive_retries.get(&key) {
            let (saved, receipt): (String, MutationReceipt) = decoded(&bytes)?;
            return if saved == hash { Ok(receipt) } else { Err(DriveError::Conflict) };
        }
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        let object = graph.objects.get_mut(&request.object).ok_or(DriveError::Inaccessible)?;
        if object.version != request.expected_object_version {
            return Err(DriveError::Conflict);
        }
        let mut project: Project = decoded(&self.projects.get(&request.object).ok_or(DriveError::Inaccessible)?)?;
        let revision = project.revisions.iter().find(|r| r.id == request.revision).ok_or(DriveError::Inaccessible)?.clone();
        if project.pending_upload.is_some()
            || self.pending_in_space(object.space, project.owner).into_iter().any(|id| {
                self.drive_uploads
                    .get(&id)
                    .and_then(|b| decoded::<super::drive_transfer::Stage>(&b).ok())
                    .is_some_and(|s| matches!(s.request.target,UploadTarget::Replace{file,..} if file==request.object))
            })
        {
            return Err(DriveError::Conflict);
        }
        if project.revisions.last().is_some_and(|r| r.id == request.revision) {
            return Err(DriveError::InvalidInput("The current version cannot be deleted".into()));
        }
        let props = self.revisions.get(request.object, request.revision)?;
        if props.version != request.expected_properties_version {
            return Err(DriveError::Conflict);
        }
        if props.keep || !props.label.is_empty() {
            return Err(DriveError::InvalidInput("Remove the version name and keep setting before deleting".into()));
        }
        let space = object.space;
        let used = self.space_usage(space, project.owner).checked_sub(revision.bytes).ok_or(DriveError::Internal)?;
        let total = self.total.get().checked_sub(revision.bytes).ok_or(DriveError::Internal)?;
        project.revisions.retain(|r| r.id != revision.id);
        object.version = object.version.checked_add(1).ok_or(DriveError::Internal)?;
        let receipt = MutationReceipt { object: object.id, version: object.version };
        graph.tree_version = graph.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        let changes = self.prepare_graph(&graph)?;
        let bytes = encoded(&project)?;
        let saved = encoded(&(hash, &receipt))?;
        let event = self.activity.prepare(project.owner, project.id, actor, ActivityKind::RevisionDeleted, now)?;
        // All errors are resolved before publication; receipt tombstones keep a
        // delayed upload retry from recreating the deleted bytes.
        for index in 0..revision.chunk_count {
            self.chunks.remove(&(revision.id, index));
        }
        self.revisions.remove(project.id, revision.id);
        self.attributions.remove(&revision.id);
        self.projects.insert(project.id, bytes);
        self.apply_graph(changes);
        self.set_space_usage(space, project.owner, used);
        self.total.set(total);
        self.drive_retries.insert(key, saved);
        self.activity.apply(event);
        Ok(receipt)
    }
}

// Called before publishing a new head. Manual, named and pinned revisions are
// never candidates. New automatic snapshots participate in the same 20-item cap.
pub(super) struct Retention {
    removed: Vec<Revision>,
    used: u64,
    total: u64,
    properties: Option<Vec<u8>>,
    revision: u64,
}
impl<M: Memory> Store<M> {
    pub(super) fn prepare_retention(&self, project: &mut Project, space: ObjectId, revision: u64, automatic: bool) -> DriveResult<Retention> {
        let mut candidates = Vec::new();
        for item in &project.revisions {
            let props = if item.id == revision {
                RevisionProperties { automatic, ..RevisionProperties::default() }
            } else {
                self.revisions.get(project.id, item.id)?
            };
            if props.automatic && !props.keep && props.label.is_empty() {
                candidates.push(item.clone());
            }
        }
        let removed = candidates.iter().take(candidates.len().saturating_sub(20)).cloned().collect::<Vec<_>>();
        let freed = removed.iter().try_fold(0u64, |total, r| total.checked_add(r.bytes).ok_or(DriveError::Internal))?;
        let used = self.space_usage(space, project.owner).checked_sub(freed).ok_or(DriveError::Internal)?;
        let total = self.total.get().checked_sub(freed).ok_or(DriveError::Internal)?;
        project.revisions.retain(|r| !removed.iter().any(|old| old.id == r.id));
        let properties = if automatic { Some(encoded(&RevisionProperties { automatic: true, ..RevisionProperties::default() })?) } else { None };
        Ok(Retention { removed, used, total, properties, revision })
    }
    pub(super) fn apply_retention(&mut self, project: &Project, space: ObjectId, retention: Retention) {
        for revision in retention.removed {
            for index in 0..revision.chunk_count {
                self.chunks.remove(&(revision.id, index));
            }
            self.revisions.remove(project.id, revision.id);
            self.attributions.remove(&revision.id);
        }
        if let Some(bytes) = retention.properties {
            self.revisions.properties.insert((project.id, retention.revision), bytes);
        }
        self.set_space_usage(space, project.owner, retention.used);
        self.total.set(retention.total);
    }
}

//! Shared uploads use the immutable chunk/revision store. Memory 13 holds staging records.
use super::collaboration::{decoded, encoded, name, retry};
use super::*;
use crate::permissions::{AccessGraph, Actor};
use serde::Deserialize;
use sovereign_cloud::drive::*;

const UPLOAD_TTL: u64 = 24 * 60 * 60 * 1_000_000_000;
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Stage {
    pub(super) upload: DriveUpload,
    pub(super) request: DriveUploadRequest,
    pub(super) uploader: Principal,
    pub(super) owner: Principal,
    pub(super) space: ObjectId,
    created_at: u64,
}
impl<M: Memory> Store<M> {
    fn stage(&self, actor: &Actor, id: u64) -> DriveResult<Stage> {
        let stage: Stage = decoded(&self.drive_uploads.get(&id).ok_or(DriveError::Expired)?)?;
        if stage.uploader != actor.sponsor {
            return Err(DriveError::Inaccessible);
        }
        Ok(stage)
    }
    fn upload_access(&self, graph: &AccessGraph, actor: &Actor, target: &UploadTarget, now: u64) -> DriveResult<DriveObject> {
        let (id, required) = match target {
            UploadTarget::New { parent, .. } => (*parent, Capabilities { create: true, ..Capabilities::READ }),
            UploadTarget::Replace { file, .. } => (*file, Capabilities { edit: true, ..Capabilities::READ }),
        };
        graph.require(actor, id, required, now, false)?;
        let object = graph.objects.get(&id).ok_or(DriveError::Inaccessible)?.clone();
        match (target, &object.kind) {
            (UploadTarget::New { .. }, ObjectKind::Folder) | (UploadTarget::Replace { .. }, ObjectKind::File(_)) => Ok(object),
            _ => Err(DriveError::Inaccessible),
        }
    }
    fn check_upload_target(&self, graph: &AccessGraph, request: &DriveUploadRequest, object: &DriveObject) -> DriveResult<()> {
        match &request.target {
            UploadTarget::New { parent, name: value, format } => {
                let value = name(value)?;
                self.check_new_name(*parent, &value, None)?;
                if format.media_type.is_empty() || format.media_type.len() > 160 || format.media_type.chars().any(char::is_control) {
                    return Err(DriveError::InvalidInput("Invalid media type".into()));
                }
                if let Some(kind) = &format.craft {
                    kind.validate().map_err(DriveError::InvalidInput)?;
                }
                if graph.ancestry(*parent, false)?.len() >= crate::permissions::MAX_DEPTH
                    || self.read_objects.len() >= 10_000
                    || self.drive_space_count(graph, object.owner)? >= 200
                    || self.space_file_count(object.space, object.owner)? >= MAX_USER_PROJECTS
                    || self.projects.len() >= MAX_PROJECTS
                {
                    return Err(DriveError::Limit("Cloud file or folder limit reached".into()));
                }
            }
            UploadTarget::Replace { file, expected_revision } => {
                let project: Project = decoded(&self.projects.get(file).ok_or(DriveError::Inaccessible)?)?;
                if project.revisions.last().map(|r| r.id) != Some(*expected_revision) || project.pending_upload.is_some() {
                    return Err(DriveError::Conflict);
                }
                let can_prune = request.automatic == Some(true)
                    && project.revisions.iter().filter(|r| self.revisions.get(*file, r.id).is_ok_and(|p| p.automatic && !p.keep && p.label.is_empty())).count()
                        >= 20;
                if project.revisions.len() >= MAX_REVISIONS && !can_prune {
                    return Err(DriveError::Limit("Revision limit reached".into()));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn drive_begin(&mut self, actor: &Actor, request: DriveUploadRequest, now: u64) -> DriveResult<DriveUpload> {
        let target_id = match request.target {
            UploadTarget::New { parent, .. } => parent,
            UploadTarget::Replace { file, .. } => file,
        };
        let graph = self.graph_for_object(target_id)?;
        let object = self.upload_access(&graph, actor, &request.target, now)?;
        let (key, hash) = retry(actor, "upload", &request.request_id, &request)?;
        if let Some(bytes) = self.drive_retries.get(&key) {
            let (previous, id): (String, u64) = decoded(&bytes)?;
            if hash != previous {
                return Err(DriveError::Conflict);
            }
            let upload = self.stage(actor, id)?.upload;
            if upload.committed.is_some() {
                self.drive_get(actor, upload.file, now)?;
            }
            return Ok(upload);
        }
        if request.bytes > MAX_DOCUMENT_BYTES || request.sha256.len() != 32 {
            return Err(DriveError::InvalidInput("Files must be at most 64 MiB with a SHA-256 digest".into()));
        }
        self.check_upload_target(&graph, &request, &object)?;
        if self.drive_uploads.len() >= 20_000
            || self.drive_retries.len() >= 20_000
            || self.pending_owners.len() >= 1_000
            || self.pending_in_space(object.space, object.owner).len() >= 20
        {
            return Err(DriveError::Limit("Upload limit reached; discard unfinished uploads".into()));
        }
        let used = self.space_usage(object.space, object.owner).checked_add(request.bytes).ok_or(DriveError::Internal)?;
        let total = self.total.get().checked_add(request.bytes).ok_or(DriveError::Internal)?;
        if used > USER_QUOTA_BYTES || total > TOTAL_QUOTA_BYTES {
            return Err(DriveError::Limit("Cloud storage quota reached".into()));
        }
        let (id, next) = self.next_id().map_err(|_| DriveError::Internal)?;
        let file = match request.target {
            UploadTarget::New { .. } => self.next_object_id().map_err(|_| DriveError::Internal)?,
            UploadTarget::Replace { file, .. } => file,
        };
        let upload = DriveUpload { id, file, committed: None };
        let stage = Stage { upload: upload.clone(), request, uploader: actor.sponsor, owner: object.owner, space: object.space, created_at: now };
        let (bytes, receipt) = (encoded(&stage)?, encoded(&(hash, id))?);
        self.drive_uploads.insert(id, bytes);
        self.add_pending_space(object.space, object.owner, id);
        self.pending_uploaders.insert((actor.sponsor, id), 0);
        self.drive_retries.insert(key, receipt);
        self.set_space_usage(object.space, object.owner, used);
        self.total.set(total);
        self.next.set(next);
        Ok(upload)
    }
    pub(crate) fn drive_put(&mut self, actor: &Actor, id: u64, index: u64, bytes: Vec<u8>, now: u64) -> DriveResult<()> {
        let stage = self.stage(actor, id)?;
        let target = match stage.request.target {
            UploadTarget::New { parent, .. } => parent,
            UploadTarget::Replace { file, .. } => file,
        };
        self.upload_access(&self.indexed_graph(target)?, actor, &stage.request.target, now)?;
        if stage.upload.committed.is_some() {
            return Err(DriveError::Conflict);
        }
        if now.saturating_sub(stage.created_at) > UPLOAD_TTL {
            return Err(DriveError::Expired);
        }
        let n = stage.request.bytes.div_ceil(CHUNK_BYTES as u64);
        if index >= n || bytes.len() as u64 != (stage.request.bytes - index * CHUNK_BYTES as u64).min(CHUNK_BYTES as u64) {
            return Err(DriveError::InvalidInput("Invalid chunk size or index".into()));
        }
        if let Some(old) = self.chunks.get(&(id, index)) {
            return if old == bytes { Ok(()) } else { Err(DriveError::Conflict) };
        }
        self.chunks.insert((id, index), bytes);
        Ok(())
    }
    pub(crate) fn drive_commit(&mut self, actor: &Actor, id: u64, now: u64) -> DriveResult<DriveUpload> {
        let mut stage = self.stage(actor, id)?;
        let target = match stage.request.target {
            UploadTarget::New { parent, .. } => parent,
            UploadTarget::Replace { file, .. } => file,
        };
        let mut graph = self.graph_for_object(target)?;
        let target = self.upload_access(&graph, actor, &stage.request.target, now)?;
        if stage.upload.committed.is_some() {
            self.drive_get(actor, stage.upload.file, now)?;
            return Ok(stage.upload);
        }
        if now.saturating_sub(stage.created_at) > UPLOAD_TTL {
            return Err(DriveError::Expired);
        }
        if target.owner != stage.owner || target.space != stage.space {
            return Err(DriveError::Conflict);
        }
        self.check_upload_target(&graph, &stage.request, &target)?;
        let n = stage.request.bytes.div_ceil(CHUNK_BYTES as u64);
        let mut hash = Sha256::new();
        for index in 0..n {
            hash.update(self.chunks.get(&(id, index)).ok_or_else(|| DriveError::InvalidInput("Upload incomplete".into()))?);
        }
        if hash.finalize().to_vec() != stage.request.sha256 {
            return Err(DriveError::InvalidInput("File integrity check failed".into()));
        }
        let revision =
            Revision { id, bytes: stage.request.bytes, sha256: stage.request.sha256.clone(), chunk_count: n, author: actor.sponsor, created_at: now };
        let (mut project, kind) = match &stage.request.target {
            UploadTarget::New { parent, name: value, format } => {
                let value = name(value)?;
                graph.objects.insert(
                    stage.upload.file,
                    DriveObject {
                        id: stage.upload.file,
                        owner: stage.owner,
                        space: stage.space,
                        parent: Some(*parent),
                        name: value.clone(),
                        kind: ObjectKind::File(format.clone()),
                        version: 1,
                        created_at: now,
                        updated_at: now,
                        trashed_at: None,
                    },
                );
                let kind = format.craft.clone().unwrap_or(FileKind { format_id: "cloud.binary".into(), format_version: 1, created_by_app: "cloud".into() });
                (Project { id: stage.upload.file, owner: stage.owner, name: value, created_at: now, revisions: vec![], pending_upload: None }, Some(kind))
            }
            UploadTarget::Replace { file, .. } => {
                let object = graph.objects.get_mut(file).ok_or(DriveError::Inaccessible)?;
                object.updated_at = now;
                object.version = object.version.checked_add(1).ok_or(DriveError::Internal)?;
                (decoded::<Project>(&self.projects.get(file).ok_or(DriveError::Inaccessible)?)?, None)
            }
        };
        project.revisions.push(revision.clone());
        let retention = self.prepare_retention(&mut project, stage.space, revision.id, stage.request.automatic == Some(true))?;
        stage.upload.committed = Some(revision.clone());
        let upload = Upload {
            id,
            created_at: stage.created_at,
            committed: Some(revision),
            request: BeginUpload {
                project_id: project.id,
                bytes: stage.request.bytes,
                sha256: stage.request.sha256.clone(),
                request_id: stage.request.request_id.clone(),
                expected_revision: match stage.request.target {
                    UploadTarget::Replace { expected_revision, .. } => Some(expected_revision),
                    _ => None,
                },
            },
        };
        let attribution =
            Attribution { actor: actor.subject.clone(), sponsor: None, credential: actor.credential.clone(), mutation: stage.request.request_id.clone() };
        graph.tree_version = graph.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        let (gb, pb, ub, sb, ab, kb) = (
            self.prepare_graph_size(&graph, Some((project.id, stage.request.bytes)))?,
            encoded(&project)?,
            encoded(&upload)?,
            encoded(&stage)?,
            encoded(&attribution)?,
            kind.as_ref().map(encoded).transpose()?,
        );
        let event_kind = if kind.is_some() { ActivityKind::Created } else { ActivityKind::ContentSaved };
        let event = self.activity.prepare(project.owner, project.id, actor, event_kind, now)?;
        self.apply_retention(&project, stage.space, retention);
        self.apply_graph(gb);
        self.activity.apply(event);
        self.projects.insert(project.id, pb);
        self.personal_project_index(project.owner, stage.space, project.id);
        self.uploads.insert(id, ub);
        self.drive_uploads.insert(id, sb);
        self.remove_pending_space(stage.space, stage.owner, id);
        self.pending_uploaders.remove(&(stage.uploader, id));
        self.attributions.insert(id, ab);
        if let Some(bytes) = kb {
            self.file_kinds.insert(project.id, bytes);
        }
        self.account_files.insert((stage.space, project.id), 0);
        self.search.enqueue(project.id, id);
        Ok(stage.upload)
    }
    /// Cleanup is allowed to the uploader/space owner even after sharing is revoked or expires.
    pub(crate) fn drive_abort(&mut self, actor: &Actor, id: u64) -> DriveResult<()> {
        let Some(bytes) = self.drive_uploads.get(&id) else {
            return Ok(());
        };
        let stage: Stage = decoded(&bytes)?;
        if actor.sponsor != stage.uploader && !self.manages_space(actor, stage.space, stage.owner) {
            return Err(DriveError::Inaccessible);
        }
        if stage.upload.committed.is_some() {
            return Err(DriveError::Conflict);
        }
        let used = self.space_usage(stage.space, stage.owner).checked_sub(stage.request.bytes).ok_or(DriveError::Internal)?;
        let total = self.total.get().checked_sub(stage.request.bytes).ok_or(DriveError::Internal)?;
        for i in 0..stage.request.bytes.div_ceil(CHUNK_BYTES as u64) {
            self.chunks.remove(&(id, i));
        }
        self.drive_uploads.remove(&id);
        self.remove_pending_space(stage.space, stage.owner, id);
        self.pending_uploaders.remove(&(stage.uploader, id));
        self.set_space_usage(stage.space, stage.owner, used);
        self.total.set(total);
        Ok(())
    }
    pub(crate) fn drive_pending(&self, actor: &Actor) -> DriveResult<Vec<DriveUpload>> {
        let mut ids: std::collections::BTreeSet<_> = self
            .pending_owners
            .range((actor.sponsor, 0)..=(actor.sponsor, u64::MAX))
            .chain(self.pending_uploaders.range((actor.sponsor, 0)..=(actor.sponsor, u64::MAX)))
            .map(|e| e.key().1)
            .collect();
        for space in self.spaces.ids(actor.sponsor) {
            if self.spaces.role(space, actor.sponsor)? == Some(sovereign_cloud::spaces::SpaceRole::Manager) {
                ids.extend(self.pending_in_space(space, actor.sponsor));
            }
        }
        ids.into_iter()
            .map(|id| {
                let stage: Stage = decoded(&self.drive_uploads.get(&id).ok_or(DriveError::Internal)?)?;
                if stage.upload.committed.is_some() || (!self.manages_space(actor, stage.space, stage.owner) && stage.uploader != actor.sponsor) {
                    return Err(DriveError::Internal);
                }
                Ok(stage.upload)
            })
            .collect()
    }

    pub(crate) fn drive_history(&self, actor: &Actor, file: ObjectId, now: u64) -> DriveResult<Vec<Revision>> {
        self.indexed_graph(file)?.require(actor, file, Capabilities { edit: true, ..Capabilities::READ }, now, false)?;
        let project: Project = decoded(&self.projects.get(&file).ok_or(DriveError::Inaccessible)?)?;
        Ok(project.revisions)
    }
    pub(crate) fn drive_revision_chunk(&self, actor: &Actor, file: ObjectId, revision: u64, index: u64, now: u64) -> DriveResult<Vec<u8>> {
        let entry = self.drive_get(actor, file, now)?;
        let current = entry.latest.ok_or(DriveError::Inaccessible)?;
        let manifest = if current.id == revision {
            current
        } else {
            self.drive_history(actor, file, now)?.into_iter().find(|r| r.id == revision).ok_or(DriveError::Inaccessible)?
        };
        if index >= manifest.chunk_count {
            return Err(DriveError::InvalidInput("Invalid chunk index".into()));
        }
        self.chunks.get(&(revision, index)).ok_or(DriveError::Internal)
    }
    /// Recovery entries are paginated and require live Read + Delete rights.
    pub(crate) fn drive_trash(&self, actor: &Actor, cursor: Option<DriveCursor>, limit: u32, now: u64) -> DriveResult<DrivePage> {
        let query = BrowseQuery {
            scope: BrowseScope::Trash,
            text: String::new(),
            file_type: String::new(),
            owner: None,
            modified_after: None,
            modified_before: None,
            sort: SortField::Name,
            descending: false,
        };
        let page = self.drive_query(actor, query, cursor, limit, now)?;
        Ok(DrivePage { entries: page.items.into_iter().map(|i| i.entry).collect(), next: page.next })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use ic_stable_structures::VectorMemory;
    fn actor(n: u8) -> Actor {
        Actor::user(Principal::self_authenticating([n; 32])).unwrap()
    }
    fn request(parent: ObjectId, name: &str, data: &[u8], token: &str) -> DriveUploadRequest {
        DriveUploadRequest {
            automatic: None,
            target: UploadTarget::New { parent, name: name.into(), format: FileFormat { media_type: "text/plain".into(), craft: None } },
            bytes: data.len() as u64,
            sha256: Sha256::digest(data).to_vec(),
            request_id: token.into(),
        }
    }
    fn put(s: &mut Store<VectorMemory>, a: &Actor, id: u64, bytes: &[u8]) {
        for (i, chunk) in bytes.chunks(CHUNK_BYTES).enumerate() {
            s.drive_put(a, id, i as u64, chunk.to_vec(), 20).unwrap();
        }
    }
    #[test]
    fn complete_drive_journey_preserves_bytes_versions_and_stable_state() {
        let memory = VectorMemory::default();
        let mut s = Store::new(memory.clone());
        let a = actor(1);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        let folder = s
            .drive_create_folder(&a, CreateFolder { parent: root, name: "Documents".into(), request_id: "create-folder-journey".into() }, 2)
            .unwrap()
            .object
            .id;
        let data = vec![73; CHUNK_BYTES + 17];
        let req = request(folder, "notes.txt", &data, "upload-journey-001");
        let stage = s.drive_begin(&a, req.clone(), 3).unwrap();
        assert_eq!(stage.id, s.drive_begin(&a, req, 4).unwrap().id);
        assert!(s.drive_children(&a, folder, 5).unwrap().is_empty());
        assert!(s.drive_commit(&a, stage.id, 5).is_err());
        put(&mut s, &a, stage.id, &data);
        drop(s);
        let mut s = Store::new(memory.clone()); // Pending upload survives upgrade.
        let done = s.drive_commit(&a, stage.id, 21).unwrap();
        assert_eq!(s.drive_commit(&a, stage.id, 22).unwrap().committed, done.committed);
        assert_eq!(s.drive_revision_chunk(&a, done.file, stage.id, 1, 23).unwrap(), vec![73; 17]);
        let file = s.drive_get(&a, done.file, 23).unwrap();
        s.drive_organize(
            &a,
            OrganizeRequest {
                object: done.file,
                expected_version: file.object.version,
                request_id: "rename-journey-001".into(),
                change: OrganizationChange::Rename("renamed.txt".into()),
            },
            24,
        )
        .unwrap();
        let file = s.drive_get(&a, done.file, 25).unwrap();
        s.drive_organize(
            &a,
            OrganizeRequest {
                object: done.file,
                expected_version: file.object.version,
                request_id: "move-journey-00001".into(),
                change: OrganizationChange::Move(root),
            },
            26,
        )
        .unwrap();
        let replacement = DriveUploadRequest {
            automatic: None,
            target: UploadTarget::Replace { file: done.file, expected_revision: stage.id },
            bytes: 3,
            sha256: Sha256::digest(b"new").to_vec(),
            request_id: "replace-journey-01".into(),
        };
        let next = s.drive_begin(&a, replacement, 27).unwrap();
        put(&mut s, &a, next.id, b"new");
        s.drive_commit(&a, next.id, 28).unwrap();
        assert_eq!(s.drive_history(&a, done.file, 29).unwrap().len(), 2);
        assert_eq!(s.drive_revision_chunk(&a, done.file, stage.id, 0, 29).unwrap(), vec![73; CHUNK_BYTES]);
        let file = s.drive_get(&a, done.file, 29).unwrap();
        s.drive_organize(
            &a,
            OrganizeRequest {
                object: done.file,
                expected_version: file.object.version,
                request_id: "trash-journey-0001".into(),
                change: OrganizationChange::Trash,
            },
            30,
        )
        .unwrap();
        assert!(s.drive_get(&a, done.file, 31).is_err());
        assert!(s.drive_history(&a, done.file, 31).is_err());
        let trash = s.drive_trash(&a, None, 50, 31).unwrap().entries;
        assert_eq!(trash.len(), 1);
        s.drive_organize(
            &a,
            OrganizeRequest {
                object: done.file,
                expected_version: trash[0].object.version,
                request_id: "restore-journey-01".into(),
                change: OrganizationChange::Restore,
            },
            32,
        )
        .unwrap();
        drop(s);
        let s = Store::new(memory);
        let file = s.drive_get(&a, done.file, 33).unwrap();
        assert_eq!(file.object.name, "renamed.txt");
        assert_eq!(file.object.parent, Some(root));
        assert_eq!(s.drive_revision_chunk(&a, done.file, next.id, 0, 33).unwrap(), b"new");
    }
    #[test]
    fn shared_upload_revocation_cleanup_and_author_are_enforced() {
        let mut s = Store::new(VectorMemory::default());
        let a = actor(1);
        let b = actor(2);
        let c = actor(3);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        s.drive_initialize(b.sponsor, 1).unwrap();
        let shared =
            s.drive_create_folder(&a, CreateFolder { parent: root, name: "Shared".into(), request_id: "shared-folder-test".into() }, 2).unwrap().object.id;
        let grant_req = GrantRequest {
            request_id: "shared-grant-test".into(),
            id: None,
            expected_version: None,
            root: shared,
            subject: b.subject.clone(),
            capabilities: Capabilities { create: true, ..Capabilities::READ },
            expires_at: None,
            active: true,
        };
        let grant = s.drive_grant(&a, grant_req.clone(), 3).unwrap();
        assert!(s.drive_begin(&b, request(root, "private", b"x", "private-upload-001"), 4).is_err());
        let stage = s.drive_begin(&b, request(shared, "shared.txt", b"abc", "shared-upload-001"), 4).unwrap();
        assert!(s.drive_put(&c, stage.id, 0, b"abc".to_vec(), 5).is_err());
        put(&mut s, &b, stage.id, b"abc");
        let result = s.drive_commit(&b, stage.id, 21).unwrap();
        let entry = s.drive_get(&a, result.file, 22).unwrap();
        assert_eq!(entry.latest.unwrap().author, b.sponsor);
        assert_eq!(entry.object.owner, a.sponsor);
        assert_eq!(s.usage.get(&a.sponsor), Some(3));
        assert_eq!(s.usage.get(&b.sponsor), None);
        let replace = DriveUploadRequest {
            automatic: None,
            target: UploadTarget::Replace { file: result.file, expected_revision: stage.id },
            bytes: 1,
            sha256: Sha256::digest(b"x").to_vec(),
            request_id: "denied-edit-00001".into(),
        };
        assert!(s.drive_begin(&b, replace, 23).is_err());
        let unfinished = s.drive_begin(&b, request(shared, "unfinished", b"pending", "shared-upload-002"), 24).unwrap();
        s.drive_grant(
            &a,
            GrantRequest { id: Some(grant.id), expected_version: Some(grant.version), active: false, request_id: "revoke-shared-001".into(), ..grant_req },
            25,
        )
        .unwrap();
        assert!(s.drive_put(&b, unfinished.id, 0, b"pending".to_vec(), 26).is_err());
        assert!(s.drive_commit(&b, unfinished.id, 26).is_err());
        assert!(s.drive_commit(&b, stage.id, 26).is_err());
        assert!(s.drive_history(&b, result.file, 26).is_err());
        assert!(s.drive_abort(&c, unfinished.id).is_err());
        s.drive_abort(&b, unfinished.id).unwrap();
        s.drive_abort(&b, unfinished.id).unwrap();
        assert_eq!(s.usage.get(&a.sponsor), Some(3));
    }
    #[test]
    fn simultaneous_replacements_cannot_overwrite_and_expired_stages_are_reclaimable() {
        let mut s = Store::new(VectorMemory::default());
        let a = actor(1);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        let initial = s.drive_begin(&a, request(root, "file", b"first", "initial-upload-001"), 2).unwrap();
        put(&mut s, &a, initial.id, b"first");
        s.drive_commit(&a, initial.id, 21).unwrap();
        let req = DriveUploadRequest {
            automatic: None,
            target: UploadTarget::Replace { file: initial.file, expected_revision: initial.id },
            bytes: 1,
            sha256: Sha256::digest(b"x").to_vec(),
            request_id: "concurrent-first-01".into(),
        };
        let one = s.drive_begin(&a, req.clone(), 22).unwrap();
        let two = s.drive_begin(&a, DriveUploadRequest { automatic: None, request_id: "concurrent-second-1".into(), ..req }, 22).unwrap();
        put(&mut s, &a, one.id, b"x");
        put(&mut s, &a, two.id, b"x");
        s.drive_commit(&a, one.id, 23).unwrap();
        assert_eq!(s.drive_commit(&a, two.id, 24).unwrap_err(), DriveError::Conflict);
        s.drive_abort(&a, two.id).unwrap();
        let expired = s.drive_begin(&a, request(root, "expires", b"abc", "expired-upload-001"), 30).unwrap();
        assert_eq!(s.drive_put(&a, expired.id, 0, b"abc".to_vec(), 31 + UPLOAD_TTL).unwrap_err(), DriveError::Expired);
        s.drive_abort(&a, expired.id).unwrap();
        assert_eq!(s.drive_pending(&a).unwrap().len(), 0);
        assert_eq!(*s.total.get(), 6);
        let empty = s.drive_begin(&a, request(root, "empty", b"", "empty-upload-00001"), 32).unwrap();
        s.drive_commit(&a, empty.id, 33).unwrap();
        assert_eq!(s.drive_get(&a, empty.file, 34).unwrap().latest.unwrap().bytes, 0);
    }
}

//! Owner-only, atomic deletion of a confirmed Trash snapshot. Existing alpha
//! quotas bound the work to 199 objects, 20 files and 256 MiB of reserved bytes.
use super::collaboration::{decoded, encoded, retry};
use super::drive_transfer::Stage;
use super::*;
use crate::permissions::Actor;
use sovereign_cloud::drive::*;
use std::collections::{BTreeMap, BTreeSet};

impl<M: Memory> Store<M> {
    pub(crate) fn drive_purge_trash(&mut self, actor: &Actor, request: PurgeRequest) -> DriveResult<PurgeReceipt> {
        // Delete grants only authorize recoverable Trash/Restore. Purge requires
        // the owning account, including for each descendant and staged upload.
        if actor.sponsor == Principal::anonymous() || actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() {
            return Err(DriveError::Inaccessible);
        }
        if request.items.is_empty() || request.items.len() > 200 {
            return Err(DriveError::InvalidInput("Select 1–200 trashed items".into()));
        }
        let (key, hash) = retry(actor, "purge", &request.request_id, &request)?;
        if let Some(bytes) = self.drive_retries.get(&key) {
            let (saved, receipt): (String, PurgeReceipt) = decoded(&bytes)?;
            return if saved == hash { Ok(receipt) } else { Err(DriveError::Conflict) };
        }
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        let mut graph = self.read_graph();
        graph.owner_scope = Some(actor.sponsor);
        for item in &request.items {
            self.read_path(&mut graph, item.object)?;
            graph.owner(actor, item.object)?;
            self.read_subtree(&mut graph, item.object)?;
            if graph.objects.len() > 8192 {
                return Err(DriveError::Limit("Choose a smaller batch".into()));
            }
        }
        let mut roots = BTreeSet::new();
        for item in &request.items {
            let object = graph.objects.get(&item.object).ok_or(DriveError::Inaccessible)?;
            if object.owner != actor.sponsor || object.parent.is_none() {
                return Err(DriveError::Inaccessible);
            }
            if object.trashed_at.is_none() || object.version != item.version || !roots.insert(item.object) {
                return Err(DriveError::Conflict);
            }
        }
        let mut ids = BTreeSet::new();
        for object in graph.objects.values() {
            let path = graph.ancestry(object.id, true)?;
            if path.iter().any(|id| roots.contains(id)) {
                ids.insert(object.id);
            }
        }

        // Collect and validate all accounting/metadata before any stable writes.
        // A committed Drive upload also has a legacy Upload; count it only once.
        let mut uploads = BTreeMap::new();
        for id in &ids {
            if let Some(bytes) = self.projects.get(id) {
                let project: Project = decoded(&bytes)?;
                if project.owner != actor.sponsor {
                    return Err(DriveError::Internal);
                }
                for revision in project.revisions {
                    uploads.insert(revision.id, revision.bytes);
                }
                if let Some(pending) = project.pending_upload {
                    let upload: Upload = decoded(&self.uploads.get(&pending).ok_or(DriveError::Internal)?)?;
                    if upload.request.project_id != *id || upload.committed.is_some() {
                        return Err(DriveError::Internal);
                    }
                    uploads.insert(pending, upload.request.bytes);
                }
            }
        }
        let mut stages = BTreeMap::new();
        for entry in self.pending_owners.range((actor.sponsor, 0)..=(actor.sponsor, u64::MAX)) {
            let id = entry.key().1;
            let stage: Stage = decoded(&self.drive_uploads.get(&id).ok_or(DriveError::Internal)?)?;
            let target = match stage.request.target {
                UploadTarget::New { parent, .. } => parent,
                UploadTarget::Replace { file, .. } => file,
            };
            if ids.contains(&target) {
                if stage.owner != actor.sponsor || stage.upload.committed.is_some() {
                    return Err(DriveError::Internal);
                }
                uploads.insert(id, stage.request.bytes);
                stages.insert(id, stage);
            }
        }
        for id in uploads.keys() {
            if let Some(bytes) = self.drive_uploads.get(id) {
                let stage: Stage = decoded(&bytes)?;
                if stage.owner != actor.sponsor {
                    return Err(DriveError::Internal);
                }
                stages.insert(*id, stage);
            }
        }
        let freed_bytes = uploads.values().try_fold(0u64, |sum, bytes| sum.checked_add(*bytes).ok_or(DriveError::Internal))?;
        let used = self.usage.get(&actor.sponsor).unwrap_or(0).checked_sub(freed_bytes).ok_or(DriveError::Internal)?;
        let total = self.total.get().checked_sub(freed_bytes).ok_or(DriveError::Internal)?;
        let removed: Vec<_> = ids.iter().map(|id| graph.objects.remove(id).ok_or(DriveError::Internal)).collect::<DriveResult<_>>()?;
        graph.grants.retain(|_, grant| !ids.contains(&grant.root));
        graph.tree_version = graph.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        let changes = self.prepare_graph(&graph)?;
        let receipt = PurgeReceipt { deleted_objects: ids.len() as u64, freed_bytes };
        let saved = encoded(&(hash, &receipt))?;

        // No await and no fallible results after this point: errors cannot leave
        // partial deletion. Canister traps roll back the entire update message.
        for (id, bytes) in uploads {
            for index in 0..bytes.div_ceil(CHUNK_BYTES as u64) {
                self.chunks.remove(&(id, index));
            }
            self.uploads.remove(&id);
            self.drive_uploads.remove(&id);
            self.attributions.remove(&id);
            if let Some(stage) = stages.get(&id) {
                self.pending_owners.remove(&(stage.owner, id));
                self.pending_uploaders.remove(&(stage.uploader, id));
            }
        }
        for object in removed {
            self.activity.remove_object(object.id);
            self.revisions.remove_object(object.id);
            self.projects.remove(&object.id);
            self.project_owners.remove(&(object.owner, object.id));
            self.account_files.remove(&(object.space, object.id));
            self.file_kinds.remove(&object.id);
        }
        self.apply_graph(changes);
        self.usage.insert(actor.sponsor, used);
        self.total.set(total);
        // Keep prior retry tombstones so a delayed create cannot recreate a
        // purged file. Purge retries return the same receipt after an upgrade.
        self.drive_retries.insert(key, saved);
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_stable_structures::VectorMemory;

    fn actor(n: u8) -> Actor {
        Actor::user(Principal::self_authenticating([n; 32])).unwrap()
    }
    fn folder(s: &mut Store<VectorMemory>, a: &Actor, parent: ObjectId, name: &str) -> DriveEntry {
        s.drive_create_folder(a, CreateFolder { parent, name: name.into(), request_id: format!("create-folder-{parent}-{name}") }, 10).unwrap()
    }
    fn file(s: &mut Store<VectorMemory>, a: &Actor, parent: ObjectId, name: &str) -> DriveEntry {
        s.drive_write_inline(
            a,
            InlineFile {
                parent,
                name: name.into(),
                format: FileFormat { media_type: "text/plain".into(), craft: None },
                bytes: b"hello".to_vec(),
                request_id: format!("create-file-{parent}-{name}"),
            },
            11,
        )
        .unwrap()
    }
    fn trash(s: &mut Store<VectorMemory>, a: &Actor, entry: &DriveEntry) -> MutationReceipt {
        s.drive_organize(
            a,
            OrganizeRequest {
                object: entry.object.id,
                expected_version: entry.object.version,
                change: OrganizationChange::Trash,
                request_id: format!("trash-object-{:016}", entry.object.id),
            },
            20,
        )
        .unwrap()
    }
    fn request(items: Vec<MutationReceipt>) -> PurgeRequest {
        PurgeRequest { items, request_id: "purge-test-request-0001".into() }
    }
    fn stage(s: &mut Store<VectorMemory>, a: &Actor, target: UploadTarget, token: &str) -> DriveUpload {
        let upload = s
            .drive_begin(a, DriveUploadRequest { automatic: None, target, bytes: 3, sha256: Sha256::digest(b"abc").to_vec(), request_id: token.into() }, 12)
            .unwrap();
        s.drive_put(a, upload.id, 0, b"abc".to_vec(), 13).unwrap();
        upload
    }

    #[test]
    fn purge_cleans_subtree_versions_sharing_staging_and_quota_and_survives_reopen() {
        let memory = VectorMemory::default();
        let mut s = Store::new(memory.clone());
        let a = actor(1);
        let b = actor(2);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        s.drive_initialize(b.sponsor, 1).unwrap();
        let parent = folder(&mut s, &a, root, "parent");
        let nested = folder(&mut s, &a, parent.object.id, "nested");
        let first = file(&mut s, &a, nested.object.id, "first");
        let keep = file(&mut s, &a, root, "keep");
        s.drive_grant(
            &a,
            GrantRequest {
                id: None,
                expected_version: None,
                root: parent.object.id,
                subject: b.subject.clone(),
                capabilities: Capabilities::ALL,
                expires_at: None,
                active: true,
                request_id: "grant-purge-test-0001".into(),
            },
            11,
        )
        .unwrap();
        let revision =
            stage(&mut s, &b, UploadTarget::Replace { file: first.object.id, expected_revision: first.latest.as_ref().unwrap().id }, "committed-upload-0001");
        s.drive_commit(&b, revision.id, 14).unwrap();
        let pending = stage(&mut s, &b, UploadTarget::Replace { file: first.object.id, expected_revision: revision.id }, "pending-replace-0001");
        let new = stage(
            &mut s,
            &b,
            UploadTarget::New { parent: nested.object.id, name: "new".into(), format: FileFormat { media_type: "text/plain".into(), craft: None } },
            "pending-create-0001",
        );
        let independent = trash(&mut s, &a, &nested);
        let parent_trash = trash(&mut s, &a, &parent);
        let req = request(vec![parent_trash, independent]);
        let result = s.drive_purge_trash(&a, req.clone()).unwrap();
        assert_eq!(result, PurgeReceipt { deleted_objects: 3, freed_bytes: 14 });
        assert_eq!(s.account(a.sponsor, 21).unwrap().used_bytes, 5);
        assert_eq!(*s.total.get(), 5);
        assert_eq!(s.owned_project_count(a.sponsor).unwrap(), 1);
        assert!(s.drive_pending(&b).unwrap().is_empty());
        assert!(s.drive_trash(&a, None, 50, 22).unwrap().entries.is_empty());
        assert!(s.graph_for_owner(a.sponsor).unwrap().grants.is_empty());
        for id in [first.latest.unwrap().id, revision.id, pending.id, new.id] {
            assert!(s.chunks.get(&(id, 0)).is_none());
            assert!(s.uploads.get(&id).is_none());
            assert!(s.drive_uploads.get(&id).is_none());
            assert!(s.attributions.get(&id).is_none());
        }
        assert!(s.drive_commit(&b, pending.id, 22).is_err());
        assert!(s.drive_get(&a, first.object.id, 22).is_err());
        assert!(s.project(a.sponsor, first.object.id).is_err());
        assert_eq!(s.drive_current_chunk(&a, keep.object.id, keep.latest.unwrap().id, 0, 22).unwrap(), b"hello");
        drop(s);
        let mut s = Store::new(memory);
        assert_eq!(s.drive_purge_trash(&a, req).unwrap(), result);
        assert_eq!(s.account(a.sponsor, 23).unwrap().used_bytes, 5);
        assert_eq!(s.drive_initialize(a.sponsor, 23).unwrap().object.id, root);
        assert!(s.drive_get(&a, parent.object.id, 23).is_err());
    }

    #[test]
    fn purge_rejects_nonowners_live_roots_duplicates_and_stale_versions_without_partial_changes() {
        let mut s = Store::new(VectorMemory::default());
        let a = actor(3);
        let b = actor(4);
        let root = s.drive_initialize(a.sponsor, 1).unwrap();
        s.drive_initialize(b.sponsor, 1).unwrap();
        let target = file(&mut s, &a, root.object.id, "target");
        let other = file(&mut s, &a, root.object.id, "other");
        s.drive_grant(
            &a,
            GrantRequest {
                id: None,
                expected_version: None,
                root: target.object.id,
                subject: b.subject.clone(),
                capabilities: Capabilities { delete: true, ..Capabilities::READ },
                expires_at: None,
                active: true,
                request_id: "grant-owner-test-0001".into(),
            },
            11,
        )
        .unwrap();
        let trashed = trash(&mut s, &a, &target);
        assert_eq!(s.drive_purge_trash(&b, request(vec![trashed.clone()])).unwrap_err(), DriveError::Inaccessible);
        for items in [
            vec![trashed.clone(), MutationReceipt { object: other.object.id, version: other.object.version }],
            vec![trashed.clone(), MutationReceipt { object: root.object.id, version: root.object.version }],
            vec![trashed.clone(), trashed.clone()],
            vec![MutationReceipt { object: trashed.object, version: trashed.version - 1 }],
        ] {
            assert!(s.drive_purge_trash(&a, request(items)).is_err());
            assert_eq!(s.account(a.sponsor, 21).unwrap().used_bytes, 10);
            assert_eq!(s.drive_trash(&a, None, 50, 21).unwrap().entries.len(), 1);
        }
        s.drive_organize(
            &a,
            OrganizeRequest {
                object: trashed.object,
                expected_version: trashed.version,
                change: OrganizationChange::Restore,
                request_id: "restore-stale-test-0001".into(),
            },
            22,
        )
        .unwrap();
        assert_eq!(s.drive_purge_trash(&a, request(vec![trashed])).unwrap_err(), DriveError::Conflict);
        assert!(s.drive_get(&a, target.object.id, 23).is_ok());
    }

    #[test]
    fn empty_trash_snapshot_spans_pages_but_does_not_include_newly_trashed_items() {
        let mut s = Store::new(VectorMemory::default());
        let a = actor(5);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        for i in 0..60 {
            let entry = folder(&mut s, &a, root, &format!("folder-{i}"));
            trash(&mut s, &a, &entry);
        }
        let first = s.drive_trash(&a, None, 50, 21).unwrap();
        assert_eq!(first.entries.len(), 50);
        let second = s.drive_trash(&a, first.next, 50, 21).unwrap();
        assert_eq!(second.entries.len(), 10);
        let items = first.entries.into_iter().chain(second.entries).map(|e| MutationReceipt { object: e.object.id, version: e.object.version }).collect();
        let late = folder(&mut s, &a, root, "late");
        trash(&mut s, &a, &late);
        let req = request(items);
        assert_eq!(s.drive_purge_trash(&a, req.clone()).unwrap().deleted_objects, 60);
        assert_eq!(s.drive_trash(&a, None, 50, 23).unwrap().entries[0].object.id, late.object.id);
        let mut changed = req;
        changed.items.pop();
        assert_eq!(s.drive_purge_trash(&a, changed).unwrap_err(), DriveError::Conflict);
    }

    #[test]
    fn purge_removes_legacy_pending_upload_without_touching_live_files() {
        let mut s = Store::new(VectorMemory::default());
        let a = actor(6);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        let target = file(&mut s, &a, root, "legacy");
        let pending = s
            .begin(
                a.sponsor,
                BeginUpload {
                    project_id: target.object.id,
                    expected_revision: target.latest.as_ref().map(|r| r.id),
                    bytes: 3,
                    sha256: Sha256::digest(b"abc").to_vec(),
                    request_id: "legacy-pending-0001".into(),
                },
                12,
            )
            .unwrap();
        s.put(a.sponsor, pending.id, 0, b"abc".to_vec()).unwrap();
        let trashed = trash(&mut s, &a, &target);
        assert_eq!(s.drive_purge_trash(&a, request(vec![trashed])).unwrap().freed_bytes, 8);
        assert_eq!(s.account(a.sponsor, 23).unwrap().used_bytes, 0);
        assert!(s.chunks.get(&(pending.id, 0)).is_none());
        assert!(s.commit(a.sponsor, pending.id, 24).is_err());
    }
}

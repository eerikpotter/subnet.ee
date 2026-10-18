use super::*;
use crate::permissions::{AccessGraph, Actor};
use sovereign_cloud::drive::*;
use unicode_normalization::UnicodeNormalization;

pub(super) fn encoded<T: Serialize>(value: &T) -> DriveResult<Vec<u8>> {
    encode(value).map_err(|_| DriveError::Internal)
}
pub(super) fn decoded<T: DeserializeOwned>(bytes: &[u8]) -> DriveResult<T> {
    decode(bytes).map_err(|_| DriveError::Internal)
}
fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) fn retry<T: Serialize>(actor: &Actor, method: &str, id: &str, request: &T) -> DriveResult<(String, String)> {
    if !(16..=80).contains(&id.len()) || id.chars().any(char::is_control) {
        return Err(DriveError::InvalidInput("A 16–80 byte request ID is required".into()));
    }
    Ok((digest(&format!("{:?}:{method}:{id}", actor.subject)), digest(&serde_json::to_string(request).map_err(|_| DriveError::Internal)?)))
}
pub(super) fn name(value: &str) -> DriveResult<String> {
    let value: String = value.trim().nfc().collect();
    if value.is_empty() || value.len() > 160 || value == "." || value == ".." || value.chars().any(|c| c.is_control() || c == '/' || c == '\\') {
        return Err(DriveError::InvalidInput("Names must contain 1–160 bytes without path separators".into()));
    }
    Ok(value)
}
impl<M: Memory> Store<M> {
    pub(super) fn persist_graph(&mut self, graph: &AccessGraph) -> DriveResult<()> {
        let changes = self.prepare_graph(graph)?;
        self.apply_graph(changes);
        Ok(())
    }
    pub(super) fn drive_space_count(&self, _graph: &AccessGraph, owner: Principal) -> DriveResult<usize> {
        Ok(self.catalog_counts.get(&owner).unwrap_or((0, 0)).0 as usize)
    }
    pub(crate) fn drive_initialize(&mut self, caller: Principal, now: u64) -> DriveResult<DriveEntry> {
        let actor = Actor::user(caller)?;
        let account = self.account(caller, now).map_err(|_| DriveError::Internal)?.account;
        if self.read_objects.contains_key(&account.id) {
            return self.drive_get(&actor, account.id, now);
        }
        let mut graph = self.read_graph();
        graph.owner_scope = Some(caller);
        graph.objects.insert(
            account.id,
            DriveObject {
                id: account.id,
                space: account.id,
                owner: caller,
                parent: None,
                name: "My files".into(),
                kind: ObjectKind::Folder,
                version: 1,
                created_at: account.created_at,
                updated_at: account.created_at,
                trashed_at: None,
            },
        );
        let entry = self.entry(&graph, &actor, account.id, now)?;
        self.persist_graph(&graph)?;
        Ok(entry)
    }
    pub(super) fn entry(&self, graph: &AccessGraph, actor: &Actor, id: ObjectId, now: u64) -> DriveResult<DriveEntry> {
        graph.require(actor, id, Capabilities::READ, now, false)?;
        let object = graph.objects.get(&id).ok_or(DriveError::Inaccessible)?.clone();
        let latest = match object.kind {
            ObjectKind::Folder | ObjectKind::Shortcut(_) => None,
            ObjectKind::File(_) => {
                let project: Project = decoded(&self.projects.get(&id).ok_or(DriveError::Inaccessible)?)?;
                Some(project.revisions.last().ok_or(DriveError::Inaccessible)?.clone())
            }
        };
        let attribution = latest.as_ref().and_then(|r| self.attributions.get(&r.id)).map(|bytes| decoded(&bytes)).transpose()?;
        Ok(DriveEntry { capabilities: graph.effective(actor, id, now, false)?, sharing: Some(graph.sharing(actor, id, now)?), object, latest, attribution })
    }
    pub(crate) fn drive_get(&self, actor: &Actor, id: ObjectId, now: u64) -> DriveResult<DriveEntry> {
        self.entry(&self.indexed_graph(id)?, actor, id, now)
    }
    #[cfg(test)]
    pub(crate) fn drive_children(&self, actor: &Actor, parent: ObjectId, now: u64) -> DriveResult<Vec<DriveEntry>> {
        Ok(self.indexed_page(actor, Some(parent), None, 50, now)?.entries)
    }
    pub(super) fn check_new_name(&self, parent: ObjectId, value: &str, except: Option<ObjectId>) -> DriveResult<()> {
        if self.names.get(&super::read_index::NameKey(parent, super::browse::normalized(value))).is_some_and(|id| Some(id) != except) {
            return Err(DriveError::NameConflict);
        }
        Ok(())
    }
    pub(crate) fn drive_create_folder(&mut self, actor: &Actor, request: CreateFolder, now: u64) -> DriveResult<DriveEntry> {
        let receipt = retry(actor, "folder", &request.request_id, &request)?;
        self.create_catalog_object(actor, request, ObjectKind::Folder, receipt, now)
    }
    pub(crate) fn drive_create_shortcut(&mut self, actor: &Actor, request: CreateShortcut, now: u64) -> DriveResult<DriveEntry> {
        let target = self.drive_get(actor, request.target, now)?;
        if matches!(target.object.kind, ObjectKind::Shortcut(_)) {
            return Err(DriveError::InvalidInput("Choose the original file or folder, not another shortcut".into()));
        }
        let receipt = retry(actor, "shortcut", &request.request_id, &request)?;
        let folder = CreateFolder { parent: request.parent, name: request.name, request_id: request.request_id };
        self.create_catalog_object(actor, folder, ObjectKind::Shortcut(request.target), receipt, now)
    }
    pub(crate) fn drive_shortcut_target(&self, actor: &Actor, object: ObjectId, now: u64) -> DriveResult<DriveEntry> {
        let shortcut = self.drive_get(actor, object, now)?;
        let ObjectKind::Shortcut(target) = shortcut.object.kind else {
            return Err(DriveError::InvalidInput("Not a shortcut".into()));
        };
        self.drive_get(actor, target, now)
    }
    fn create_catalog_object(
        &mut self,
        actor: &Actor,
        request: CreateFolder,
        kind: ObjectKind,
        receipt: (String, String),
        now: u64,
    ) -> DriveResult<DriveEntry> {
        let mut graph = self.graph_for_object(request.parent)?;
        let parent = request.parent;
        graph.require(actor, parent, Capabilities { create: true, ..Capabilities::READ }, now, false)?;
        let (retry_key, request_digest) = receipt;
        if let Some(bytes) = self.drive_retries.get(&retry_key) {
            let (saved, id): (String, ObjectId) = decoded(&bytes)?;
            if saved != request_digest {
                return Err(DriveError::Conflict);
            }
            return self.drive_get(actor, id, now);
        }
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        let parent_object = graph.objects.get(&parent).ok_or(DriveError::Inaccessible)?;
        if parent_object.kind != ObjectKind::Folder {
            return Err(DriveError::Inaccessible);
        }
        if graph.ancestry(parent, false)?.len() >= crate::permissions::MAX_DEPTH {
            return Err(DriveError::Limit("Folder depth limit reached".into()));
        }
        if self.read_objects.len() >= 10_000 || self.drive_space_count(&graph, parent_object.owner)? >= 200 {
            return Err(DriveError::Limit("Folder limit reached".into()));
        }
        let name = name(&request.name)?;
        self.check_new_name(parent, &name, None)?;
        let id = self.next_object_id().map_err(|_| DriveError::Internal)?;
        let object = DriveObject {
            id,
            space: parent_object.space,
            owner: parent_object.owner,
            parent: Some(parent),
            name,
            kind,
            version: 1,
            created_at: now,
            updated_at: now,
            trashed_at: None,
        };
        graph.objects.insert(id, object);
        graph.tree_version = graph.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        let entry = self.entry(&graph, actor, id, now)?;
        let bytes = self.prepare_graph(&graph)?;
        let receipt = encoded(&(request_digest, id))?;
        let event = self.activity.prepare(entry.object.owner, id, actor, ActivityKind::Created, now)?;
        self.apply_graph(bytes);
        self.activity.apply(event);
        self.drive_retries.insert(retry_key, receipt);
        Ok(entry)
    }
    pub(crate) fn drive_grant(&mut self, actor: &Actor, request: GrantRequest, now: u64) -> DriveResult<AccessGrant> {
        let mut graph = self.graph_for_object(request.root)?;
        graph.owner(actor, request.root)?;
        let (retry_key, request_digest) = retry(actor, "grant", &request.request_id, &request)?;
        if let Some(bytes) = self.drive_retries.get(&retry_key) {
            let (saved_digest, result): (String, AccessGrant) = decoded(&bytes)?;
            return if saved_digest == request_digest { Ok(result) } else { Err(DriveError::Conflict) };
        }
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        if request.id.is_none() && graph.grants.values().filter(|g| g.root == request.root).count() >= 100 {
            return Err(DriveError::Limit("Sharing limit reached for this item".into()));
        }
        let (new_id, next) = self.next_id().map_err(|_| DriveError::Internal)?;
        let id = request.id.unwrap_or(new_id);
        if request.id.is_some() && request.expected_version.is_none() {
            return Err(DriveError::InvalidInput("Expected grant version required".into()));
        }
        match request.subject {
            Participant::User(principal) => {
                graph.owner(actor, request.root)?;
                if self.accounts.get(&principal).is_none() || (request.active && self.safety.blocked(actor.sponsor, principal)) {
                    return Err(DriveError::Inaccessible);
                }
            }
            Participant::Agent(_) => return Err(DriveError::InvalidInput("Separate agent grants are not available in alpha".into())),
        }
        if request.id.is_none() && self.read_grants.len() >= crate::permissions::MAX_GRANTS as u64 {
            return Err(DriveError::Limit("Sharing limit reached".into()));
        }
        let grant = graph.set_grant(
            actor,
            AccessGrant {
                id,
                root: request.root,
                subject: request.subject,
                capabilities: request.capabilities,
                expires_at: request.expires_at,
                active: request.active,
                version: 0,
            },
            request.expected_version,
            now,
        )?;
        let (graph_bytes, receipt_bytes) = (self.prepare_graph(&graph)?, encoded(&(request_digest, &grant))?);
        let event = self.activity.prepare(actor.sponsor, request.root, actor, ActivityKind::SharingChanged, now)?;
        let notice = match grant.subject {
            Participant::User(recipient) if grant.active => {
                self.people.prepare(recipient, grant.root, actor.sponsor, sovereign_cloud::people::NoticeKind::Shared, now)?
            }
            _ => None,
        };
        self.apply_graph(graph_bytes);
        self.activity.apply(event);
        self.people.apply(notice);
        self.drive_retries.insert(retry_key, receipt_bytes);
        if request.id.is_none() {
            self.next.set(next);
        }
        Ok(grant)
    }
    pub(crate) fn drive_write_inline(&mut self, actor: &Actor, request: InlineFile, now: u64) -> DriveResult<DriveEntry> {
        let mut graph = self.graph_for_object(request.parent)?;
        graph.require(actor, request.parent, Capabilities { create: true, ..Capabilities::READ }, now, false)?;
        let parent = graph.objects.get(&request.parent).ok_or(DriveError::Inaccessible)?.clone();
        if parent.kind != ObjectKind::Folder {
            return Err(DriveError::Inaccessible);
        }
        if request.bytes.len() > CHUNK_BYTES || !(16..=80).contains(&request.request_id.len()) {
            return Err(DriveError::InvalidInput("Inline files require at most 512 KiB and a 16–80 byte request ID".into()));
        }
        let name = name(&request.name)?;
        if request.format.media_type.is_empty() || request.format.media_type.len() > 160 || request.format.media_type.chars().any(char::is_control) {
            return Err(DriveError::InvalidInput("Invalid media type".into()));
        }
        if let Some(kind) = &request.format.craft {
            kind.validate().map_err(DriveError::InvalidInput)?;
        }
        let request_digest = digest(&serde_json::to_string(&request).map_err(|_| DriveError::Internal)?);
        let retry_key = digest(&format!("{:?}:create:{}", actor.subject, request.request_id));
        if let Some(previous) = self.drive_retries.get(&retry_key) {
            let (saved_digest, id): (String, ObjectId) = decoded(&previous)?;
            if saved_digest != request_digest {
                return Err(DriveError::Conflict);
            }
            return self.drive_get(actor, id, now);
        }
        self.check_new_name(request.parent, &name, None)?;
        if graph.ancestry(parent.id, false)?.len() >= crate::permissions::MAX_DEPTH {
            return Err(DriveError::Limit("Folder depth limit reached".into()));
        }
        if self.read_objects.len() >= 10_000 || self.drive_space_count(&graph, parent.owner)? >= 200 {
            return Err(DriveError::Limit("Drive object limit reached".into()));
        }
        let project_count = self.owned_project_count(parent.owner).map_err(|_| DriveError::Internal)?;
        if project_count >= MAX_USER_PROJECTS || self.projects.len() >= MAX_PROJECTS || self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Cloud file or retry limit reached".into()));
        }
        let used = self.usage.get(&parent.owner).unwrap_or(0).checked_add(request.bytes.len() as u64).ok_or(DriveError::Internal)?;
        let total = self.total.get().checked_add(request.bytes.len() as u64).ok_or(DriveError::Internal)?;
        if used > USER_QUOTA_BYTES || total > TOTAL_QUOTA_BYTES {
            return Err(DriveError::Limit("Cloud storage quota reached".into()));
        }
        let id = self.next_object_id().map_err(|_| DriveError::Internal)?;
        let (revision_id, next) = self.next_id().map_err(|_| DriveError::Internal)?;
        let revision = Revision {
            id: revision_id,
            bytes: request.bytes.len() as u64,
            sha256: Sha256::digest(&request.bytes).to_vec(),
            created_at: now,
            author: actor.sponsor,
            chunk_count: u64::from(!request.bytes.is_empty()),
        };
        let project = Project { id, owner: parent.owner, name: name.clone(), created_at: now, revisions: vec![revision.clone()], pending_upload: None };
        let upload = Upload {
            id: revision_id,
            request: BeginUpload {
                project_id: id,
                expected_revision: None,
                bytes: revision.bytes,
                sha256: revision.sha256.clone(),
                request_id: request.request_id.clone(),
            },
            created_at: now,
            committed: Some(revision.clone()),
        };
        let attribution = Attribution {
            actor: actor.subject.clone(),
            sponsor: matches!(actor.subject, Participant::Agent(_)).then_some(actor.sponsor),
            credential: actor.credential.clone(),
            mutation: request.request_id,
        };
        let legacy_kind =
            request.format.craft.clone().unwrap_or(FileKind { format_id: "cloud.binary".into(), format_version: 1, created_by_app: "cloud".into() });
        graph.objects.insert(
            id,
            DriveObject {
                id,
                space: parent.space,
                owner: parent.owner,
                parent: Some(parent.id),
                name,
                kind: ObjectKind::File(request.format),
                version: 1,
                created_at: now,
                updated_at: now,
                trashed_at: None,
            },
        );
        graph.tree_version = graph.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        let capabilities = graph.effective(actor, id, now, false)?;
        let entry = DriveEntry {
            object: graph.objects.get(&id).ok_or(DriveError::Internal)?.clone(),
            capabilities,
            sharing: Some(graph.sharing(actor, id, now)?),
            latest: Some(revision),
            attribution: Some(attribution.clone()),
        };
        let (gb, pb, ub, ab, kb, rb) = (
            self.prepare_graph_size(&graph, Some((id, project.revisions.last().map_or(0, |r| r.bytes))))?,
            encoded(&project)?,
            encoded(&upload)?,
            encoded(&attribution)?,
            encoded(&legacy_kind)?,
            encoded(&(request_digest, id))?,
        );
        let event = self.activity.prepare(parent.owner, id, actor, ActivityKind::Created, now)?;
        // Everything fallible is complete. Publish content, metadata, quota and retry together.
        self.projects.insert(id, pb);
        self.project_owners.insert((parent.owner, id), 0);
        self.uploads.insert(revision_id, ub);
        if !request.bytes.is_empty() {
            self.chunks.insert((revision_id, 0), request.bytes);
        }
        self.attributions.insert(revision_id, ab);
        self.file_kinds.insert(id, kb);
        self.account_files.insert((parent.space, id), 0);
        self.usage.insert(parent.owner, used);
        self.total.set(total);
        self.apply_graph(gb);
        self.activity.apply(event);
        self.drive_retries.insert(retry_key, rb);
        self.next.set(next);
        Ok(entry)
    }
    pub(crate) fn drive_current_chunk(&self, actor: &Actor, file: ObjectId, revision: u64, index: u64, now: u64) -> DriveResult<Vec<u8>> {
        let entry = self.drive_get(actor, file, now)?;
        let head = entry.latest.ok_or(DriveError::Inaccessible)?;
        if head.id != revision {
            return Err(DriveError::Conflict);
        }
        if index >= head.chunk_count {
            return Err(DriveError::InvalidInput("Invalid chunk index".into()));
        }
        self.chunks.get(&(revision, index)).ok_or(DriveError::Internal)
    }

    pub(crate) fn drive_list(&self, actor: &Actor, parent: Option<ObjectId>, cursor: Option<DriveCursor>, limit: u32, now: u64) -> DriveResult<DrivePage> {
        self.indexed_page(actor, parent, cursor, limit, now)
    }

    pub(crate) fn drive_grants(&self, actor: &Actor, object: ObjectId) -> DriveResult<Vec<AccessGrant>> {
        let graph = self.indexed_graph(object)?;
        graph.owner(actor, object)?;
        let path = graph.ancestry(object, true)?;
        Ok(graph.grants.values().filter(|g| path.contains(&g.root)).cloned().collect())
    }

    pub(crate) fn drive_organize(&mut self, actor: &Actor, request: OrganizeRequest, now: u64) -> DriveResult<MutationReceipt> {
        let mut graph = self.graph_for_object(request.object)?;
        let recover = matches!(request.change, OrganizationChange::Restore | OrganizationChange::RestoreReviewed { .. });
        let required = match request.change {
            OrganizationChange::Rename(_) => Capabilities { edit: true, ..Capabilities::READ },
            OrganizationChange::Move(_) | OrganizationChange::MoveReviewed { .. } | OrganizationChange::RestoreReviewed { .. } => {
                Capabilities { edit: true, delete: true, ..Capabilities::READ }
            }
            _ => Capabilities { delete: true, ..Capabilities::READ },
        };
        // Retries never reveal receipts after access is revoked. Trash receipts are
        // checked against the retained hierarchy, since successful trash is hidden.
        graph.require(actor, request.object, required, now, recover || matches!(request.change, OrganizationChange::Trash))?;
        let (key, hash) = retry(actor, "organize", &request.request_id, &request)?;
        if let Some(bytes) = self.drive_retries.get(&key) {
            let (saved, receipt): (String, MutationReceipt) = decoded(&bytes)?;
            return if saved == hash { Ok(receipt) } else { Err(DriveError::Conflict) };
        }
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        let mut object = graph.objects.get(&request.object).ok_or(DriveError::Inaccessible)?.clone();
        let old_parent = object.parent.ok_or_else(|| DriveError::InvalidInput("The space root cannot be changed".into()))?;
        if object.version != request.expected_version {
            return Err(DriveError::Conflict);
        }
        graph.ancestry(object.id, recover)?;
        if !matches!(request.change, OrganizationChange::Rename(_)) {
            self.read_subtree(&mut graph, object.id)?;
        }
        match &request.change {
            OrganizationChange::Move(parent) | OrganizationChange::MoveReviewed { parent, .. } | OrganizationChange::RestoreReviewed { parent, .. } => {
                self.read_path(&mut graph, *parent)?
            }
            _ => {}
        }
        let subtree: Vec<_> = graph
            .objects
            .values()
            .filter(|candidate| candidate.space == object.space)
            .map(|candidate| Ok((candidate.id, graph.ancestry(candidate.id, true)?)))
            .collect::<DriveResult<Vec<_>>>()?
            .into_iter()
            .filter(|(_, path)| path.contains(&object.id))
            .map(|(id, _)| id)
            .collect();
        if !matches!(request.change, OrganizationChange::Rename(_)) {
            for id in &subtree {
                graph.require(actor, *id, required, now, true)?;
            }
        }
        let event_kind = match &request.change {
            OrganizationChange::Rename(_) => ActivityKind::Renamed,
            OrganizationChange::Move(_) | OrganizationChange::MoveReviewed { .. } => ActivityKind::Moved,
            OrganizationChange::Trash => ActivityKind::Trashed,
            OrganizationChange::Restore | OrganizationChange::RestoreReviewed { .. } => ActivityKind::Restored,
        };
        match request.change {
            OrganizationChange::Rename(value) => {
                let new_name = name(&value)?;
                if new_name != object.name {
                    self.check_new_name(old_parent, &new_name, Some(object.id))?;
                    object.name = new_name;
                }
            }
            OrganizationChange::Move(parent) | OrganizationChange::MoveReviewed { parent, .. } => {
                if let OrganizationChange::MoveReviewed { tree_version, .. } = request.change
                    && tree_version != graph.tree_version
                {
                    return Err(DriveError::Conflict);
                }
                self.plan_move(&graph, actor, object.id, parent, now)?;
                object.parent = Some(parent);
            }
            OrganizationChange::Trash => {
                object.trashed_at = Some(now);
            }
            OrganizationChange::Restore => {
                if object.trashed_at.is_none() {
                    return Err(DriveError::Conflict);
                }
                graph.ancestry(old_parent, false)?;
                self.check_new_name(old_parent, &object.name, Some(object.id))?;
                object.trashed_at = None;
            }
            OrganizationChange::RestoreReviewed { parent, ref name, tree_version } => {
                if tree_version != graph.tree_version {
                    return Err(DriveError::Conflict);
                }
                self.plan_relocation(&graph, actor, object.id, parent, Some(name), now)?;
                object.parent = Some(parent);
                object.name = self::name(name)?;
                object.trashed_at = None;
            }
        }
        if graph.objects.get(&object.id) == Some(&object) {
            let receipt = MutationReceipt { object: object.id, version: object.version };
            self.drive_retries.insert(key, encoded(&(hash, &receipt))?);
            return Ok(receipt);
        }
        object.version = object.version.checked_add(1).ok_or(DriveError::Internal)?;
        object.updated_at = now;
        let receipt = MutationReceipt { object: object.id, version: object.version };
        graph.objects.insert(object.id, object.clone());
        for id in subtree {
            graph.ancestry(id, true)?;
        }
        graph.tree_version = graph.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        // Keep the legacy name in sync without changing file bytes or revision IDs.
        let project = self
            .projects
            .get(&object.id)
            .map(|bytes| {
                let mut project: Project = decoded(&bytes)?;
                project.name = object.name;
                encoded(&project)
            })
            .transpose()?;
        let (graph_bytes, receipt_bytes) = (self.prepare_graph(&graph)?, encoded(&(hash, &receipt))?);
        let event = self.activity.prepare(object.owner, object.id, actor, event_kind, now)?;
        self.apply_graph(graph_bytes);
        self.activity.apply(event);
        self.drive_retries.insert(key, receipt_bytes);
        if let Some(bytes) = project {
            self.projects.insert(object.id, bytes);
        }
        Ok(receipt)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use ic_stable_structures::VectorMemory;

    fn user(n: u8) -> Actor {
        Actor::user(Principal::self_authenticating([n; 32])).unwrap()
    }
    fn folder(store: &mut Store<VectorMemory>, actor: &Actor, parent: ObjectId, name: &str) -> DriveEntry {
        store.drive_create_folder(actor, CreateFolder { parent, name: name.into(), request_id: format!("folder-{parent}-{name}-request") }, 10).unwrap()
    }
    fn share(store: &mut Store<VectorMemory>, owner: &Actor, root: ObjectId, recipient: &Actor, capabilities: Capabilities) -> AccessGrant {
        let request = GrantRequest {
            request_id: format!("grant-test-{}-request", store.next.get()),
            id: None,
            expected_version: None,
            root,
            subject: recipient.subject.clone(),
            capabilities,
            expires_at: None,
            active: true,
        };
        let grant = store.drive_grant(owner, request.clone(), 11).unwrap();
        assert_eq!(store.drive_grant(owner, request.clone(), 12).unwrap(), grant);
        assert!(matches!(store.drive_grant(owner, GrantRequest { active: false, ..request }, 12), Err(DriveError::Conflict)));
        grant
    }
    fn file(parent: ObjectId, name: &str, id: &str) -> InlineFile {
        InlineFile {
            parent,
            name: name.into(),
            request_id: id.into(),
            bytes: b"shared bytes".to_vec(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
        }
    }
    fn change(entry: &DriveEntry, change: OrganizationChange, id: &str) -> OrganizeRequest {
        OrganizeRequest { object: entry.object.id, expected_version: entry.object.version, request_id: id.into(), change }
    }
    fn setup() -> (Store<VectorMemory>, Actor, Actor, ObjectId, ObjectId) {
        let mut store = Store::new(VectorMemory::default());
        let (alice, bob) = (user(1), user(2));
        let a = store.drive_initialize(alice.sponsor, 1).unwrap().object.id;
        let b = store.drive_initialize(bob.sponsor, 1).unwrap().object.id;
        (store, alice, bob, a, b)
    }

    #[test]
    fn details_only_explain_the_callers_live_access_and_visible_ancestors() {
        let (mut s, owner, viewer, root, _) = setup();
        let private = folder(&mut s, &owner, root, "Private parent");
        let f = s.drive_write_inline(&owner, file(private.object.id, "Read me.txt", "details-file-request"), 10).unwrap();
        share(&mut s, &owner, f.object.id, &viewer, Capabilities::READ);
        let other = user(9);
        s.drive_initialize(other.sponsor, 1).unwrap();
        share(&mut s, &owner, private.object.id, &other, Capabilities::ALL);
        let details = s.drive_details(&viewer, f.object.id, 12).unwrap();
        assert!(details.locations.is_empty());
        assert_eq!(details.revision_count, 1);
        assert_eq!(details.access.len(), 1);
        assert!(!details.access[0].inherited);
        assert_eq!(details.access[0].location.as_ref().unwrap().id, f.object.id);
        assert!(s.drive_details(&viewer, private.object.id, 12).is_err());
        let grant = share(&mut s, &owner, private.object.id, &viewer, Capabilities::ALL);
        let details = s.drive_details(&viewer, f.object.id, 12).unwrap();
        assert_eq!(details.locations.len(), 1);
        assert_eq!(details.locations[0].name, "Private parent");
        assert_eq!(details.access.len(), 2);
        let mut graph = s.graph_for_object(f.object.id).unwrap();
        for g in graph.grants.values_mut().filter(|g| g.subject == viewer.subject) {
            g.expires_at = Some(15);
        }
        s.persist_graph(&graph).unwrap();
        assert!(s.drive_details(&viewer, f.object.id, 15).is_err());
        assert_eq!(s.drive_details(&owner, f.object.id, 15).unwrap().access.len(), 0);
        assert!(grant.active);
    }

    #[test]
    fn restore_to_new_location_reviews_access_and_preserves_collision_and_retry_rules() {
        let (mut s, owner, editor, root, other_root) = setup();
        let original = folder(&mut s, &owner, root, "Original");
        let destination = folder(&mut s, &owner, root, "Destination");
        share(&mut s, &owner, original.object.id, &editor, Capabilities::ALL);
        share(&mut s, &owner, destination.object.id, &editor, Capabilities::ALL);
        let f = s.drive_write_inline(&owner, file(original.object.id, "Report.txt", "restore-file-request"), 10).unwrap();
        let trashed = s.drive_organize(&editor, change(&f, OrganizationChange::Trash, "restore-trash-request"), 12).unwrap();
        s.drive_organize(&owner, change(&original, OrganizationChange::Trash, "restore-parent-trash"), 13).unwrap();
        assert!(
            s.drive_organize(
                &owner,
                OrganizeRequest {
                    object: f.object.id,
                    expected_version: trashed.version,
                    request_id: "restore-original-request".into(),
                    change: OrganizationChange::Restore
                },
                14
            )
            .is_err()
        );
        assert!(s.drive_restore_preview(&owner, f.object.id, other_root, "Report.txt", 14).is_err());
        let occupied = s.drive_write_inline(&owner, file(destination.object.id, "Report.txt", "restore-collision-file"), 14).unwrap();
        assert!(matches!(s.drive_restore_preview(&owner, f.object.id, destination.object.id, "Report.txt", 14), Err(DriveError::NameConflict)));
        // A different duration/audience is owner-only even when the caller can
        // edit both source and destination.
        let other = user(8);
        s.drive_initialize(other.sponsor, 1).unwrap();
        share(&mut s, &owner, destination.object.id, &other, Capabilities::READ);
        assert!(matches!(s.drive_restore_preview(&editor, f.object.id, destination.object.id, "Recovered.txt", 14), Err(DriveError::Inaccessible)));
        let preview = s.drive_restore_preview(&owner, f.object.id, destination.object.id, "Recovered.txt", 14).unwrap();
        assert!(preview.affected_subjects > 0);
        let request = OrganizeRequest {
            object: f.object.id,
            expected_version: trashed.version,
            request_id: "restore-reviewed-request".into(),
            change: OrganizationChange::RestoreReviewed { parent: destination.object.id, name: "Recovered.txt".into(), tree_version: preview.tree_version },
        };
        let result = s.drive_organize(&owner, request.clone(), 15).unwrap();
        assert_eq!(result, s.drive_organize(&owner, request, 16).unwrap());
        let restored = s.drive_get(&owner, f.object.id, 16).unwrap();
        assert_eq!(restored.object.parent, Some(destination.object.id));
        assert_eq!(restored.object.name, "Recovered.txt");
        assert_eq!(restored.latest.unwrap().id, f.latest.unwrap().id);
        assert_eq!(s.drive_get(&owner, occupied.object.id, 16).unwrap().object.name, "Report.txt");
    }

    #[test]
    fn reverse_commands_reject_stale_versions_and_revoked_access() {
        let (mut s, owner, editor, root, _) = setup();
        let f = folder(&mut s, &owner, root, "Original");
        let grant = share(&mut s, &owner, f.object.id, &editor, Capabilities::ALL);
        let first = s.drive_organize(&editor, change(&f, OrganizationChange::Rename("Changed".into()), "undo-first-operation"), 12).unwrap();
        let mut inverse = OrganizeRequest {
            object: f.object.id,
            expected_version: first.version,
            request_id: "undo-original-name".into(),
            change: OrganizationChange::Rename(f.object.name),
        };
        let current = s.drive_get(&owner, f.object.id, 13).unwrap();
        s.drive_organize(&owner, change(&current, OrganizationChange::Rename("Someone else's change".into()), "undo-other-operation"), 13).unwrap();
        assert!(matches!(s.drive_organize(&editor, inverse.clone(), 14), Err(DriveError::Conflict)));
        inverse.expected_version = s.drive_get(&owner, f.object.id, 14).unwrap().object.version;
        s.drive_grant(
            &owner,
            GrantRequest {
                request_id: "undo-revoke-access".into(),
                id: Some(grant.id),
                expected_version: Some(grant.version),
                root: grant.root,
                subject: grant.subject,
                capabilities: grant.capabilities,
                expires_at: None,
                active: false,
            },
            14,
        )
        .unwrap();
        assert!(matches!(s.drive_organize(&editor, inverse, 15), Err(DriveError::Inaccessible)));
    }

    #[test]
    fn moves_review_effective_access_durations_and_reauthorize_legacy_calls() {
        let (mut s, owner, editor, root, _) = setup();
        let a = folder(&mut s, &owner, root, "A");
        let b = folder(&mut s, &owner, root, "B");
        share(&mut s, &owner, root, &editor, Capabilities::ALL);
        let f = folder(&mut s, &owner, a.object.id, "Nested");
        let child = s.drive_write_inline(&owner, file(f.object.id, "Child.txt", "child-move-test-01"), 12).unwrap();
        // Equal inherited access is available to an Editor.
        let preview = s.drive_move_preview(&editor, f.object.id, b.object.id, 13).unwrap();
        assert_eq!(preview.affected_subjects, 0);
        let reader = user(3);
        s.drive_initialize(reader.sponsor, 1).unwrap();
        let grant = share(&mut s, &owner, a.object.id, &reader, Capabilities::READ);
        // Direct access on the root alone cannot mask a loss on its descendants.
        share(&mut s, &owner, f.object.id, &reader, Capabilities::READ);
        assert_eq!(s.drive_move_preview(&editor, f.object.id, b.object.id, 13).unwrap().affected_objects, 0);
        // A longer-lived destination grant changes future access, even while today's
        // capabilities match; the Editor cannot change that via the old endpoint.
        let mut graph = s.graph_for_object(f.object.id).unwrap();
        for g in graph.grants.values_mut().filter(|g| g.subject == reader.subject) {
            g.expires_at = Some(u64::MAX);
        }
        s.persist_graph(&graph).unwrap();
        share(&mut s, &owner, b.object.id, &reader, Capabilities::READ);
        assert!(matches!(s.drive_move_preview(&editor, f.object.id, b.object.id, 13), Err(DriveError::Inaccessible)));
        assert!(matches!(
            s.drive_organize(&editor, change(&f, OrganizationChange::Move(b.object.id), "editor-move-denied"), 13),
            Err(DriveError::Inaccessible)
        ));
        let impact = s.drive_move_preview(&owner, f.object.id, b.object.id, 13).unwrap();
        assert_eq!(impact.affected_objects, 2);
        assert_eq!(impact.changes[0].subject, reader.subject);
        assert!(impact.changes[0].gains);
        let reviewed = change(&f, OrganizationChange::MoveReviewed { parent: b.object.id, tree_version: impact.tree_version }, "reviewed-move-01");
        folder(&mut s, &owner, root, "Concurrent change");
        assert!(matches!(s.drive_organize(&owner, reviewed, 14), Err(DriveError::Conflict)));
        let impact = s.drive_move_preview(&owner, f.object.id, b.object.id, 14).unwrap();
        let reviewed = change(&f, OrganizationChange::MoveReviewed { parent: b.object.id, tree_version: impact.tree_version }, "reviewed-move-02");
        let receipt = s.drive_organize(&owner, reviewed.clone(), 15).unwrap();
        assert_eq!(s.drive_organize(&owner, reviewed, 16).unwrap(), receipt);
        assert!(s.drive_get(&reader, child.object.id, 101).is_ok());
        assert!(grant.active);
    }

    #[test]
    fn create_only_share_charges_owner_attributes_account_and_revokes_retries() {
        let (mut store, alice, bob, root, _) = setup();
        let folder = folder(&mut store, &alice, root, "Shared");
        let rights = Capabilities { create: true, ..Capabilities::READ };
        let grant = share(&mut store, &alice, folder.object.id, &bob, rights);
        let request = file(folder.object.id, "Example.txt", "create-only-request-1");
        let created = store.drive_write_inline(&bob, request.clone(), 12).unwrap();
        assert_eq!(created.object.owner, alice.sponsor);
        assert_eq!(created.attribution.as_ref().unwrap().actor, bob.subject);
        assert_eq!(created.capabilities, Capabilities::READ);
        assert_eq!(created.latest.as_ref().unwrap().author, bob.sponsor);
        assert_eq!(store.drive_write_inline(&bob, request.clone(), 13).unwrap().object.id, created.object.id);
        assert!(store.drive_write_inline(&bob, InlineFile { bytes: vec![9], ..request.clone() }, 13).is_err());
        assert_eq!(store.usage.get(&alice.sponsor), Some(12));
        assert_eq!(store.usage.get(&bob.sponsor), None);
        assert!(store.drive_organize(&bob, change(&created, OrganizationChange::Rename("Changed".into()), "rename-denied-request"), 14).is_err());
        assert!(store.drive_organize(&bob, change(&created, OrganizationChange::Trash, "trash-denied-request"), 14).is_err());
        assert!(store.drive_grants(&bob, folder.object.id).is_err());
        assert!(store.drive_write_inline(&bob, file(root, "Private.txt", "private-denied-request"), 14).is_err());
        let rev = created.latest.unwrap();
        assert_eq!(store.drive_current_chunk(&bob, created.object.id, rev.id, 0, 14).unwrap(), b"shared bytes");
        store
            .drive_grant(
                &alice,
                GrantRequest {
                    request_id: format!("grant-test-{}-request", store.next.get()),
                    id: Some(grant.id),
                    expected_version: Some(grant.version),
                    root: grant.root,
                    subject: bob.subject.clone(),
                    capabilities: rights,
                    expires_at: None,
                    active: false,
                },
                15,
            )
            .unwrap();
        assert!(store.drive_write_inline(&bob, request, 16).is_err());
        assert!(store.drive_current_chunk(&bob, created.object.id, rev.id, 0, 16).is_err());
    }

    #[test]
    fn direct_file_share_reveals_no_private_ancestors_or_roster() {
        let (mut store, alice, bob, root, _) = setup();
        let private = folder(&mut store, &alice, root, "Private");
        let visible = store.drive_write_inline(&alice, file(private.object.id, "Shared.txt", "direct-share-file-request"), 12).unwrap();
        share(&mut store, &alice, visible.object.id, &bob, Capabilities::READ);
        let roots = store.drive_list(&bob, None, None, 50, 13).unwrap();
        assert_eq!(roots.entries.iter().filter(|e| e.object.owner == alice.sponsor).map(|e| e.object.id).collect::<Vec<_>>(), [visible.object.id]);
        assert!(store.drive_get(&bob, private.object.id, 13).is_err());
        assert!(store.drive_children(&bob, private.object.id, 13).is_err());
        assert!(store.drive_grants(&bob, visible.object.id).is_err());
        assert!(
            store
                .drive_grant(
                    &bob,
                    GrantRequest {
                        request_id: format!("grant-test-{}-request", store.next.get()),
                        id: None,
                        expected_version: None,
                        root: visible.object.id,
                        subject: user(3).subject,
                        capabilities: Capabilities::ALL,
                        expires_at: None,
                        active: true
                    },
                    13
                )
                .is_err()
        );
    }

    #[test]
    fn moves_are_atomic_bounded_and_keep_ids_and_content() {
        let (mut store, alice, bob, root, foreign_root) = setup();
        let source = folder(&mut store, &alice, root, "Source");
        let child = folder(&mut store, &alice, source.object.id, "Child");
        let destination = folder(&mut store, &alice, root, "Destination");
        let created = store.drive_write_inline(&alice, file(child.object.id, "File.txt", "move-file-create-request"), 12).unwrap();
        share(&mut store, &alice, source.object.id, &bob, Capabilities::ALL);
        assert!(store.drive_organize(&alice, change(&source, OrganizationChange::Move(child.object.id), "cycle-rejected-request"), 13).is_err());
        assert!(store.drive_organize(&alice, change(&source, OrganizationChange::Move(foreign_root), "cross-space-request"), 13).is_err());
        assert_eq!(store.drive_get(&alice, source.object.id, 13).unwrap().object, source.object);
        let request = change(&child, OrganizationChange::Move(destination.object.id), "move-valid-request-1");
        let receipt = store.drive_organize(&alice, request.clone(), 14).unwrap();
        assert_eq!(receipt, store.drive_organize(&alice, request, 15).unwrap());
        assert!(store.drive_get(&bob, created.object.id, 15).is_err());
        let after = store.drive_get(&alice, created.object.id, 15).unwrap();
        assert_eq!(after.latest, created.latest);
        assert_eq!(after.object.id, created.object.id);
        let child = store.drive_get(&alice, child.object.id, 15).unwrap();
        let no_op = store.drive_organize(&alice, change(&child, OrganizationChange::Move(destination.object.id), "same-parent-request"), 16).unwrap();
        assert_eq!(no_op.version, child.object.version);
    }

    #[test]
    fn trash_retains_independent_child_state_and_blocks_legacy_paths() {
        let (mut store, alice, _, root, _) = setup();
        let parent = folder(&mut store, &alice, root, "Parent");
        let hidden = folder(&mut store, &alice, parent.object.id, "Earlier trash");
        let file = store.drive_write_inline(&alice, file(parent.object.id, "File.txt", "trash-file-create-request"), 12).unwrap();
        store.drive_organize(&alice, change(&hidden, OrganizationChange::Trash, "trash-child-request"), 13).unwrap();
        let request = change(&parent, OrganizationChange::Trash, "trash-parent-request");
        let receipt = store.drive_organize(&alice, request.clone(), 14).unwrap();
        assert_eq!(store.drive_organize(&alice, request, 15).unwrap(), receipt);
        assert!(store.drive_get(&alice, file.object.id, 15).is_err());
        assert!(store.project(alice.sponsor, file.object.id).is_err());
        assert!(store.list(alice.sponsor).unwrap().is_empty());
        assert!(store.files(alice.sponsor, root, None, 50).unwrap().files.is_empty());
        assert!(store.remove_upload(alice.sponsor, file.latest.as_ref().unwrap().id, 22).is_err());
        let restored = store
            .drive_organize(
                &alice,
                OrganizeRequest {
                    object: parent.object.id,
                    expected_version: receipt.version,
                    request_id: "restore-parent-request".into(),
                    change: OrganizationChange::Restore,
                },
                16,
            )
            .unwrap();
        assert_eq!(restored.version, receipt.version + 1);
        assert!(store.drive_get(&alice, file.object.id, 17).is_ok());
        assert!(store.drive_get(&alice, hidden.object.id, 17).is_err());
    }

    #[test]
    fn nfc_collisions_pagination_and_stable_state_survive_reopening() {
        let memory = VectorMemory::default();
        let mut store = Store::new(memory.clone());
        let alice = user(1);
        let root = store.drive_initialize(alice.sponsor, 1).unwrap().object.id;
        let a = folder(&mut store, &alice, root, "Café");
        let b = folder(&mut store, &alice, root, "Other");
        assert!(matches!(
            store.drive_create_folder(&alice, CreateFolder { parent: root, name: " Cafe\u{301} ".into(), request_id: "normalization-collision".into() }, 10),
            Err(DriveError::NameConflict)
        ));
        let first = store.drive_list(&alice, Some(root), None, 1, 12).unwrap();
        assert_eq!(first.entries[0].object.id, a.object.id);
        store.drive_initialize(alice.sponsor, 13).unwrap();
        assert_eq!(store.drive_list(&alice, Some(root), first.next.clone(), 1, 13).unwrap().entries[0].object.id, b.object.id);
        assert!(store.drive_list(&user(2), Some(root), first.next.clone(), 1, 13).is_err());
        assert!(store.drive_list(&alice, Some(a.object.id), first.next, 1, 13).is_err());
        drop(store);
        let store = Store::new(memory);
        assert_eq!(store.drive_get(&alice, a.object.id, 14).unwrap().object, a.object);
    }
    #[test]
    fn trashed_files_still_count_against_legacy_and_drive_creation_limits() {
        let (mut store, alice, _, root, _) = setup();
        for index in 0..MAX_USER_PROJECTS {
            let request = file(root, &format!("File {index}"), &format!("quota-file-request-{index}"));
            let entry = store.drive_write_inline(&alice, request, 12).unwrap();
            store.drive_organize(&alice, change(&entry, OrganizationChange::Trash, &format!("quota-trash-request-{index}")), 13).unwrap();
        }
        assert!(store.list(alice.sponsor).unwrap().is_empty());
        assert!(store.create(alice.sponsor, "Bypass".into(), 14).is_err());
        assert!(store.drive_write_inline(&alice, file(root, "Bypass", "quota-bypass-request"), 14).is_err());
        assert_eq!(store.owned_project_count(alice.sponsor).unwrap(), MAX_USER_PROJECTS);
    }
}

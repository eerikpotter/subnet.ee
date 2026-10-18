//! Authoritative per-object stable storage with owner, parent and grant indexes.
//! Changes compare only the explicitly loaded working set; no owner-wide diff.
use super::collaboration::{decoded, encoded};
use super::*;
use crate::permissions::{AccessGraph, Actor, MAX_DEPTH};
use sovereign_cloud::drive::*;
use std::collections::BTreeSet;
use std::ops::Bound::{Excluded, Included};

/// The stable-structures tuple encoding requires bounded members. Store a
/// dedicated key instead of composing it with an unbounded String tuple.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct NameKey(pub ObjectId, pub String);
impl ic_stable_structures::Storable for NameKey {
    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Unbounded;
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        let mut bytes = self.0.as_bytes().to_vec();
        bytes.extend(self.1.as_bytes());
        bytes.into()
    }
    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }
    fn from_bytes(bytes: std::borrow::Cow<'_, [u8]>) -> Self {
        let (id, name) = bytes.split_at_checked(16).unwrap_or_else(|| ic_cdk::trap("Invalid stable name key"));
        Self(
            ObjectId::from_bytes(std::borrow::Cow::Borrowed(id)),
            String::from_utf8(name.to_vec()).unwrap_or_else(|_| ic_cdk::trap("Invalid stable name encoding")),
        )
    }
}

#[derive(Serialize, serde::Deserialize)]
struct ListBookmark {
    scope: Option<ObjectId>,
    actor: Principal,
    tree_version: u64,
    expires_at: Option<u64>,
    after: ObjectId,
}

pub(super) struct GraphChanges {
    removed_keys: Vec<super::catalog_index::CatalogKey>,
    added_keys: Vec<super::catalog_index::CatalogKey>,
    removed_objects: Vec<DriveObject>,
    objects: Vec<(DriveObject, Vec<u8>)>,
    removed_grants: Vec<AccessGrant>,
    grants: Vec<(AccessGrant, Vec<u8>)>,
    spaces: Vec<(ObjectId, u64)>,
    folders: Vec<(ObjectId, u64)>,
    acl: Vec<(ObjectId, u64)>,
    discovery: Vec<(Principal, u64)>,
    counts: Vec<(Principal, (u64, u64))>,
    roots: Vec<((Principal, ObjectId), u64)>,
}
fn changes(graph: &AccessGraph) -> DriveResult<GraphChanges> {
    Ok(GraphChanges {
        removed_keys: Vec::new(),
        added_keys: Vec::new(),
        removed_objects: graph.original_objects.iter().filter(|(id, o)| graph.objects.get(id) != Some(o)).map(|(_, o)| o.clone()).collect(),
        objects: graph
            .objects
            .iter()
            .filter(|(id, o)| graph.original_objects.get(id) != Some(o))
            .map(|(_, o)| Ok((o.clone(), encoded(o)?)))
            .collect::<DriveResult<_>>()?,
        removed_grants: graph.original_grants.iter().filter(|(id, g)| graph.grants.get(id) != Some(g)).map(|(_, g)| g.clone()).collect(),
        grants: graph
            .grants
            .iter()
            .filter(|(id, g)| graph.original_grants.get(id) != Some(g))
            .map(|(_, g)| Ok((g.clone(), encoded(g)?)))
            .collect::<DriveResult<_>>()?,
        spaces: Vec::new(),
        folders: Vec::new(),
        acl: Vec::new(),
        discovery: Vec::new(),
        counts: Vec::new(),
        roots: Vec::new(),
    })
}
impl<M: Memory> Store<M> {
    pub(super) fn graph_for_owner(&self, owner: Principal) -> DriveResult<AccessGraph> {
        let mut graph = self.read_graph();
        graph.owner_scope = Some(owner);
        if let Some(account) = self.accounts.get(&owner) {
            let account: Account = decoded(&account)?;
            graph.tree_version = self.space_versions.get(&account.personal_space).unwrap_or(0);
        }
        for entry in self.read_owners.range((owner, ObjectId::MIN)..=(owner, ObjectId::MAX)).take(8193) {
            if graph.objects.len() >= 8192 {
                return Err(DriveError::Limit("Account summary requires a smaller scope".into()));
            }
            self.read_path(&mut graph, entry.key().1)?;
        }
        if graph.objects.values().any(|o| o.owner != owner) {
            return Err(DriveError::Internal);
        }
        Ok(graph)
    }
    pub(super) fn graph_for_object(&self, id: ObjectId) -> DriveResult<AccessGraph> {
        let object: DriveObject = decoded(&self.read_objects.get(&id).ok_or(DriveError::Inaccessible)?)?;
        let mut graph = self.indexed_graph(id)?;
        graph.owner_scope = Some(object.owner);
        graph.tree_version = self.space_versions.get(&object.space).unwrap_or(0);
        Ok(graph)
    }
    pub(super) fn prepare_graph(&self, graph: &AccessGraph) -> DriveResult<GraphChanges> {
        self.prepare_graph_size(graph, None)
    }
    pub(super) fn prepare_graph_size(&self, graph: &AccessGraph, size: Option<(ObjectId, u64)>) -> DriveResult<GraphChanges> {
        let owner = graph.owner_scope.ok_or(DriveError::Internal)?;
        if graph.objects.values().any(|o| o.owner != owner) {
            return Err(DriveError::Internal);
        }
        let mut changes = changes(graph)?;
        for old in &changes.removed_objects {
            changes.removed_keys.extend(super::catalog_index::keys(old, self.catalog_size(old.id)?));
        }
        for (object, _) in &changes.objects {
            let bytes = match size {
                Some((id, bytes)) if id == object.id => bytes,
                _ => self.catalog_size(object.id)?,
            };
            changes.added_keys.extend(super::catalog_index::keys(object, bytes));
        }
        let mut spaces = BTreeSet::new();
        let mut folders = BTreeSet::new();
        let mut acl = BTreeSet::new();
        let mut discovery = BTreeSet::new();
        let mut counts = std::collections::BTreeMap::new();
        let mut roots = std::collections::BTreeMap::new();
        for (object, adding) in changes.removed_objects.iter().map(|o| (o, false)).chain(changes.objects.iter().map(|(o, _)| (o, true))) {
            spaces.insert(object.space);
            discovery.insert(object.owner);
            if let Some(parent) = object.parent {
                folders.insert(parent);
            }
            if object.kind == ObjectKind::Folder {
                folders.insert(object.id);
            }
            let count = counts.entry(object.owner).or_insert_with(|| self.catalog_counts.get(&object.owner).unwrap_or((0u64, 0u64)));
            count.0 = if adding { count.0.checked_add(1) } else { count.0.checked_sub(1) }.ok_or(DriveError::Internal)?;
            if matches!(object.kind, ObjectKind::File(_)) {
                count.1 = if adding { count.1.checked_add(1) } else { count.1.checked_sub(1) }.ok_or(DriveError::Internal)?;
            }
            // Shared-root cards can change sort position/name. Inherited access
            // is rechecked at the source; it is not copied to descendant grants.
            for entry in self.read_grants.range((object.id, 0)..=(object.id, u64::MAX)) {
                let grant: AccessGrant = decoded(&entry.value())?;
                if let Participant::User(p) = grant.subject {
                    discovery.insert(p);
                }
            }
        }
        for (grant, adding) in changes.removed_grants.iter().map(|g| (g, false)).chain(changes.grants.iter().map(|(g, _)| (g, true))) {
            let object = graph.objects.get(&grant.root).or_else(|| graph.original_objects.get(&grant.root)).ok_or(DriveError::Internal)?;
            spaces.insert(object.space);
            acl.insert(grant.root);
            if let Participant::User(p) = grant.subject {
                discovery.insert(p);
                if grant.active && grant.capabilities.read {
                    let key = (p, grant.root);
                    let count = roots.entry(key).or_insert_with(|| self.shared_roots.get(&key).unwrap_or(0u64));
                    *count = if adding { count.checked_add(1) } else { count.checked_sub(1) }.ok_or(DriveError::Internal)?;
                }
            }
        }
        let next = |v: u64| v.checked_add(1).ok_or(DriveError::Internal);
        changes.spaces = spaces.into_iter().map(|id| Ok((id, next(self.space_versions.get(&id).unwrap_or(0))?))).collect::<DriveResult<_>>()?;
        changes.folders = folders.into_iter().map(|id| Ok((id, next(self.folder_versions.get(&id).unwrap_or(0))?))).collect::<DriveResult<_>>()?;
        changes.acl = acl.into_iter().map(|id| Ok((id, next(self.acl_versions.get(&id).unwrap_or(0))?))).collect::<DriveResult<_>>()?;
        changes.discovery = discovery.into_iter().map(|p| Ok((p, next(self.discovery_versions.get(&p).unwrap_or(0))?))).collect::<DriveResult<_>>()?;
        changes.counts = counts.into_iter().collect();
        changes.roots = roots.into_iter().collect();
        Ok(changes)
    }
    pub(super) fn apply_graph(&mut self, changes: GraphChanges) {
        for key in changes.removed_keys {
            self.catalog_order.remove(&key);
        }
        for key in changes.added_keys {
            self.catalog_order.insert(key, 0);
        }
        for old in changes.removed_objects {
            self.read_objects.remove(&old.id);
            self.read_owners.remove(&(old.owner, old.id));
            if let Some(parent) = old.parent {
                self.read_children.remove(&(parent, old.id));
                if old.trashed_at.is_none() {
                    self.names.remove(&NameKey(parent, super::browse::normalized(&old.name)));
                }
            }
        }
        for (object, value) in changes.objects {
            self.read_objects.insert(object.id, value);
            self.read_owners.insert((object.owner, object.id), 0);
            if let Some(parent) = object.parent {
                self.read_children.insert((parent, object.id), 0);
                if object.trashed_at.is_none() {
                    self.names.insert(NameKey(parent, super::browse::normalized(&object.name)), object.id);
                }
            }
        }
        for old in changes.removed_grants {
            self.read_grants.remove(&(old.root, old.id));
            if let Participant::User(p) = old.subject {
                self.read_subjects.remove(&(p, old.id));
            }
        }
        for (grant, value) in changes.grants {
            self.read_grants.insert((grant.root, grant.id), value.clone());
            if let Participant::User(p) = grant.subject {
                self.read_subjects.insert((p, grant.id), value);
            }
        }
        for (id, version) in changes.spaces {
            self.space_versions.insert(id, version);
        }
        for (id, version) in changes.folders {
            self.folder_versions.insert(id, version);
        }
        for (id, version) in changes.acl {
            self.acl_versions.insert(id, version);
        }
        for (p, version) in changes.discovery {
            self.discovery_versions.insert(p, version);
        }
        for (p, counts) in changes.counts {
            self.catalog_counts.insert(p, counts);
        }
        for (key, count) in changes.roots {
            if count == 0 {
                self.shared_roots.remove(&key);
            } else {
                self.shared_roots.insert(key, count);
            }
        }
    }
    pub(super) fn read_graph(&self) -> AccessGraph {
        AccessGraph::default()
    }
    /// Fetch only this object's path and its grants. The existing evaluator still
    /// checks cycles, owner/space boundaries, trash, expiry and capability union.
    pub(super) fn read_path(&self, graph: &mut AccessGraph, id: ObjectId) -> DriveResult<()> {
        let mut next = Some(id);
        let mut seen = BTreeSet::new();
        while let Some(id) = next {
            if graph.objects.contains_key(&id) {
                break;
            }
            if seen.len() == MAX_DEPTH || !seen.insert(id) {
                return Err(DriveError::Inaccessible);
            }
            let object: DriveObject = decoded(&self.read_objects.get(&id).ok_or(DriveError::Inaccessible)?)?;
            for entry in self.read_grants.range((id, 0)..=(id, u64::MAX)) {
                let grant: AccessGrant = decoded(&entry.value())?;
                if let Participant::User(p) = grant.subject
                    && self.safety.blocked(object.owner, p)
                {
                    graph.blocked.insert((object.owner, p));
                }
                graph.original_grants.insert(grant.id, grant.clone());
                graph.grants.insert(grant.id, grant);
            }
            next = object.parent;
            graph.original_objects.insert(id, object.clone());
            graph.objects.insert(id, object);
        }
        Ok(())
    }
    pub(super) fn indexed_graph(&self, id: ObjectId) -> DriveResult<AccessGraph> {
        let mut graph = self.read_graph();
        self.read_path(&mut graph, id)?;
        Ok(graph)
    }
    /// Explicitly bounded recursive work, used only for subtree operations.
    pub(super) fn read_subtree(&self, graph: &mut AccessGraph, root: ObjectId) -> DriveResult<()> {
        const MAX_SUBTREE: usize = 4096;
        let mut pending = vec![root];
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                return Err(DriveError::Internal);
            }
            if seen.len() + pending.len() > MAX_SUBTREE {
                return Err(DriveError::Limit("Choose a smaller subtree for this operation".into()));
            }
            self.read_path(graph, id)?;
            for entry in self.read_children.range((id, ObjectId::MIN)..=(id, ObjectId::MAX)) {
                if seen.len() + pending.len() >= MAX_SUBTREE {
                    return Err(DriveError::Limit("Choose a smaller subtree for this operation".into()));
                }
                pending.push(entry.key().1);
            }
        }
        Ok(())
    }
    /// Folder-local contents and ancestor authority; unrelated accounts/folders
    /// never invalidate a folder cursor. Discovery views use the viewer's scope.
    pub(super) fn scope_version(&self, actor: &Actor, folder: Option<ObjectId>) -> DriveResult<u64> {
        let mut hash = Sha256::new();
        hash.update(actor.sponsor.as_slice());
        hash.update(self.policy_versions.get(&actor.sponsor).unwrap_or(0).to_be_bytes());
        if let Some(id) = folder {
            hash.update(self.folder_versions.get(&id).unwrap_or(0).to_be_bytes());
            let graph = self.indexed_graph(id)?;
            for id in graph.ancestry(id, true)? {
                let object = graph.objects.get(&id).ok_or(DriveError::Internal)?;
                hash.update(id.as_bytes());
                hash.update(object.version.to_be_bytes());
                hash.update(self.acl_versions.get(&id).unwrap_or(0).to_be_bytes());
            }
        } else {
            hash.update(self.discovery_versions.get(&actor.sponsor).unwrap_or(0).to_be_bytes());
            if let Some(bytes) = self.accounts.get(&actor.sponsor) {
                let account: Account = decoded(&bytes)?;
                hash.update(self.space_versions.get(&account.personal_space).unwrap_or(0).to_be_bytes());
            }
        }
        let bytes: [u8; 32] = hash.finalize().into();
        let mut prefix = [0; 8];
        prefix.copy_from_slice(&bytes[..8]);
        Ok(u64::from_be_bytes(prefix))
    }
    pub(super) fn root_ids(&self, actor: &Actor) -> DriveResult<BTreeSet<ObjectId>> {
        let mut ids = BTreeSet::new();
        if actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() {
            return Err(DriveError::Inaccessible);
        }
        if let Some(bytes) = self.accounts.get(&actor.sponsor) {
            let account: Account = decoded(&bytes)?;
            ids.insert(account.id);
        }
        for entry in self.shared_roots.range((actor.sponsor, ObjectId::MIN)..=(actor.sponsor, ObjectId::MAX)).take(257) {
            if ids.len() >= 257 {
                return Err(DriveError::Limit("Choose a folder to narrow this view".into()));
            }
            ids.insert(entry.key().1);
        }
        Ok(ids)
    }
    pub(super) fn indexed_page(&self, actor: &Actor, parent: Option<ObjectId>, cursor: Option<DriveCursor>, limit: u32, now: u64) -> DriveResult<DrivePage> {
        let mut graph = self.read_graph();
        graph.tree_version = self.scope_version(actor, parent)?;
        let cursor: Option<ListBookmark> = cursor.map(|c| self.open_bookmark(&c, b"list-v1", actor.sponsor)).transpose()?;
        let after = match cursor {
            Some(c) if c.actor == actor.sponsor && c.scope == parent && c.tree_version == graph.tree_version && c.expires_at.is_none_or(|t| now < t) => c.after,
            Some(_) => return Err(DriveError::Conflict),
            None => ObjectId::MIN,
        };
        if let Some(parent) = parent {
            self.read_path(&mut graph, parent)?;
            graph.require(actor, parent, Capabilities::READ, now, false)?;
            if graph.objects.get(&parent).is_none_or(|o| o.kind != ObjectKind::Folder) {
                return Err(DriveError::Inaccessible);
            }
        }
        let roots;
        let ids: Box<dyn Iterator<Item = ObjectId> + '_> = if let Some(parent) = parent {
            Box::new(self.read_children.range((Excluded((parent, after)), Included((parent, ObjectId::MAX)))).map(|e| e.key().1))
        } else {
            roots = self.root_ids(actor)?;
            Box::new(roots.into_iter().filter(move |id| *id > after))
        };
        let limit = limit.clamp(1, 50) as usize;
        let mut entries = Vec::new();
        let mut ids = ids.peekable();
        let mut scanned = None;
        for _ in 0..512 {
            let Some(id) = ids.next() else { break };
            scanned = Some(id);
            match self.read_path(&mut graph, id).and_then(|()| self.entry(&graph, actor, id, now)) {
                Ok(entry) => entries.push(entry),
                Err(DriveError::Inaccessible) => continue,
                Err(error) => return Err(error),
            }
            if entries.len() == limit {
                break;
            }
        }
        let expires_at = graph.grants.values().filter(|g| g.active && g.subject == actor.subject).filter_map(|g| g.expires_at).filter(|t| *t > now).min();
        let next = if ids.peek().is_some() {
            scanned
                .map(|after| {
                    self.seal_bookmark(
                        &ListBookmark { scope: parent, expires_at, actor: actor.sponsor, tree_version: graph.tree_version, after },
                        b"list-v1",
                        actor.sponsor,
                    )
                })
                .transpose()?
        } else {
            None
        };
        Ok(DrivePage { entries, next })
    }
    pub(super) fn drive_catalog(&self, actor: &Actor, root: ObjectId, now: u64) -> DriveResult<DriveCatalog> {
        use std::collections::{BTreeMap, VecDeque};
        const LIMIT: usize = 200;
        let first = self.drive_get(actor, root, now)?;
        let mut entries = BTreeMap::from([(root, first)]);
        let mut pending = VecDeque::from([root]);
        let mut complete_folders = Vec::new();
        let mut inspected = 1;
        while let Some(parent) = pending.pop_front() {
            if entries.get(&parent).is_none_or(|e| e.object.kind != ObjectKind::Folder) {
                continue;
            }
            let mut ids = self.read_children.range((parent, ObjectId::MIN)..=(parent, ObjectId::MAX)).map(|e| e.key().1).peekable();
            while inspected < LIMIT {
                let Some(id) = ids.next() else { break };
                inspected += 1;
                match self.drive_get(actor, id, now) {
                    Ok(entry) => {
                        if entry.object.kind == ObjectKind::Folder {
                            pending.push_back(id);
                        }
                        entries.insert(id, entry);
                    }
                    Err(DriveError::Inaccessible) => {}
                    Err(error) => return Err(error),
                }
            }
            // A scan budget counts misses too. Never call a partially scanned
            // folder complete simply because its first page returned no rows.
            if ids.peek().is_none() {
                complete_folders.push(parent);
            }
        }
        Ok(DriveCatalog { entries: entries.into_values().collect(), complete_folders })
    }
    pub(crate) fn drive_browse(&self, actor: &Actor, parent: ObjectId, cursor: Option<DriveCursor>, limit: u32, now: u64) -> DriveResult<DriveBrowse> {
        let graph = self.indexed_graph(parent)?;
        let folder = self.entry(&graph, actor, parent, now)?;
        if folder.object.kind != ObjectKind::Folder {
            return Err(DriveError::Inaccessible);
        }
        // Stop at the first unreadable ancestor; never expose a collaborator's
        // private parent names above the root they shared.
        let mut breadcrumbs = Vec::new();
        for id in graph.ancestry(parent, false)? {
            match self.entry(&graph, actor, id, now) {
                Ok(entry) => breadcrumbs.push(entry),
                Err(DriveError::Inaccessible) => break,
                Err(error) => return Err(error),
            }
        }
        breadcrumbs.reverse();
        Ok(DriveBrowse {
            folder,
            breadcrumbs,
            page: self.indexed_page(actor, Some(parent), cursor.clone(), limit, now)?,
            catalog: if cursor.is_none() { self.drive_catalog(actor, parent, now)? } else { DriveCatalog::default() },
        })
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
    #[test]
    fn browse_uses_only_indexed_scope_and_never_discloses_private_ancestors() {
        let memory = VectorMemory::default();
        let mut s = Store::new(memory.clone());
        let a = actor(1);
        let b = actor(2);
        let c = actor(3);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        s.drive_initialize(b.sponsor, 1).unwrap();
        let unrelated = s.drive_initialize(c.sponsor, 1).unwrap().object.id;
        let shared =
            s.drive_create_folder(&a, CreateFolder { parent: root, name: "shared".into(), request_id: "index-test-folder-1".into() }, 2).unwrap().object.id;
        let child =
            s.drive_create_folder(&a, CreateFolder { parent: shared, name: "child".into(), request_id: "index-test-folder-2".into() }, 3).unwrap().object.id;
        let grant = s
            .drive_grant(
                &a,
                GrantRequest {
                    root: shared,
                    id: None,
                    expected_version: None,
                    subject: b.subject.clone(),
                    capabilities: Capabilities::READ,
                    active: true,
                    expires_at: Some(100),
                    request_id: "index-test-grant-01".into(),
                },
                4,
            )
            .unwrap();
        let view = s.drive_browse(&b, shared, None, 50, 5).unwrap();
        assert_eq!(view.breadcrumbs.iter().map(|e| e.object.id).collect::<Vec<_>>(), vec![shared]);
        assert_eq!(view.page.entries[0].object.id, child);
        assert_eq!(view.catalog.entries.iter().map(|e| e.object.id).collect::<BTreeSet<_>>(), BTreeSet::from([shared, child]));
        assert!(view.catalog.complete_folders.contains(&child));
        assert_eq!(view.folder.sharing, Some(SharingStatus { direct: true, inherited: false }));
        assert_eq!(view.page.entries[0].sharing, Some(SharingStatus { direct: false, inherited: true }));
        assert!(s.drive_browse(&b, root, None, 50, 5).is_err());
        assert!(s.drive_browse(&b, shared, None, 50, 100).is_err());
        s.read_objects.insert(unrelated, b"must not be decoded".to_vec());
        assert_eq!(s.drive_browse(&b, shared, None, 50, 5).unwrap().page.entries.len(), 1);
        assert_eq!(s.drive_list(&b, None, None, 50, 5).unwrap().entries.len(), 2);
        let own =
            s.drive_create_folder(&a, CreateFolder { parent: root, name: "isolated write".into(), request_id: "index-isolated-write".into() }, 6).unwrap();
        assert_eq!(own.object.owner, a.sponsor);
        assert!(s.drive_initialize(actor(4).sponsor, 6).is_ok());
        s.drive_grant(
            &a,
            GrantRequest {
                root: shared,
                id: Some(grant.id),
                expected_version: Some(grant.version),
                subject: b.subject.clone(),
                capabilities: Capabilities::READ,
                active: false,
                expires_at: None,
                request_id: "index-test-revoke-1".into(),
            },
            6,
        )
        .unwrap();
        assert!(s.drive_browse(&b, shared, None, 50, 7).is_err());
        drop(s);
        let s = Store::new(memory);
        assert!(s.drive_browse(&b, shared, None, 50, 7).is_err());
        assert_eq!(s.drive_browse(&a, shared, None, 50, 7).unwrap().page.entries.len(), 1);
    }
    #[test]
    fn catalog_budget_is_bounded_and_partial_folders_are_not_claimed_complete() {
        for count in [200, 250] {
            let mut s = Store::new(VectorMemory::default());
            let a = actor(1);
            let root = s.drive_initialize(a.sponsor, 1).unwrap().object;
            let mut graph = s.graph_for_owner(a.sponsor).unwrap();
            // Deliberately exceed today's per-owner alpha cap to exercise the
            // future larger-tree path without changing product quotas.
            for _ in 1..count {
                let id = s.next_object_id().unwrap();
                graph.objects.insert(id, DriveObject { id, parent: Some(root.id), name: format!("folder-{id}"), ..root.clone() });
            }
            s.persist_graph(&graph).unwrap();
            let view = s.drive_browse(&a, root.id, None, 50, 2).unwrap();
            assert_eq!(view.catalog.entries.len(), 200);
            assert_eq!(view.catalog.complete_folders.contains(&root.id), count == 200);
            if count == 200 {
                assert_eq!(view.catalog.complete_folders.len(), 200);
            }
            assert!(view.page.next.is_some());
            assert_eq!(view.page.entries.len(), 50);
        }
    }
    #[test]
    fn uuid_catalog_and_indexes_survive_reopening() {
        let memory = VectorMemory::default();
        let a = actor(1);
        let mut s = Store::new(memory.clone());
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        let child = s.drive_create_folder(&a, CreateFolder { parent: root, name: "preserved".into(), request_id: "migration-folder-1".into() }, 2).unwrap();
        drop(s);
        let s = Store::new(memory.clone());
        assert_eq!(s.drive_browse(&a, root, None, 50, 3).unwrap().page.entries[0].object, child.object);
        drop(s);
        assert_eq!(Store::new(memory).drive_get(&a, child.object.id, 4).unwrap().object, child.object);
    }
}

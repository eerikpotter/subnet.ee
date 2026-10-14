//! Authoritative per-object stable storage with owner, parent and grant indexes.
//! The old graph cell is read only during the one-time schema migration.
use super::collaboration::{decoded, encoded};
use super::*;
use crate::permissions::{AccessGraph, Actor, MAX_DEPTH};
use sovereign_cloud::drive::*;
use std::collections::BTreeSet;
use std::ops::Bound::{Excluded, Included};

pub(super) struct GraphChanges {
    removed_objects: Vec<DriveObject>,
    objects: Vec<(DriveObject, Vec<u8>)>,
    removed_grants: Vec<AccessGrant>,
    grants: Vec<(AccessGrant, Vec<u8>)>,
    version: u64,
}
fn changes(previous: &AccessGraph, graph: &AccessGraph) -> DriveResult<GraphChanges> {
    Ok(GraphChanges {
        removed_objects: previous.objects.iter().filter(|(id, o)| graph.objects.get(id) != Some(o)).map(|(_, o)| o.clone()).collect(),
        objects: graph
            .objects
            .iter()
            .filter(|(id, o)| previous.objects.get(id) != Some(o))
            .map(|(_, o)| Ok((o.clone(), encoded(o)?)))
            .collect::<DriveResult<_>>()?,
        removed_grants: previous.grants.iter().filter(|(id, g)| graph.grants.get(id) != Some(g)).map(|(_, g)| g.clone()).collect(),
        grants: graph
            .grants
            .iter()
            .filter(|(id, g)| previous.grants.get(id) != Some(g))
            .map(|(_, g)| Ok((g.clone(), encoded(g)?)))
            .collect::<DriveResult<_>>()?,
        version: graph.tree_version,
    })
}
impl<M: Memory> Store<M> {
    pub(super) fn migrate_graph(&mut self) -> DriveResult<()> {
        let graph: AccessGraph = decoded(self.access_graph.get())?;
        let changes = changes(&AccessGraph::default(), &graph)?;
        let projects: Vec<Project> = self.projects.iter().map(|e| decoded(&e.value())).collect::<DriveResult<_>>()?;
        self.apply_graph(changes);
        for project in projects {
            self.project_owners.insert((project.owner, project.id), 0);
        }
        self.migrate_upload_indexes()?;
        self.read_ready.set(1);
        Ok(())
    }
    pub(super) fn graph_for_owner(&self, owner: Principal) -> DriveResult<AccessGraph> {
        let mut graph = self.read_graph();
        graph.owner_scope = Some(owner);
        for entry in self.read_owners.range((owner, 0)..=(owner, u64::MAX)) {
            self.read_path(&mut graph, entry.key().1)?;
        }
        if graph.objects.values().any(|o| o.owner != owner) {
            return Err(DriveError::Internal);
        }
        Ok(graph)
    }
    pub(super) fn graph_for_object(&self, id: u64) -> DriveResult<AccessGraph> {
        let object: DriveObject = decoded(&self.read_objects.get(&id).ok_or(DriveError::Inaccessible)?)?;
        self.graph_for_owner(object.owner)
    }
    pub(super) fn prepare_graph(&self, graph: &AccessGraph) -> DriveResult<GraphChanges> {
        let owner = graph.owner_scope.ok_or(DriveError::Internal)?;
        if graph.objects.values().any(|o| o.owner != owner) {
            return Err(DriveError::Internal);
        }
        changes(&self.graph_for_owner(owner)?, graph)
    }
    pub(super) fn apply_graph(&mut self, changes: GraphChanges) {
        for old in changes.removed_objects {
            self.read_objects.remove(&old.id);
            self.read_owners.remove(&(old.owner, old.id));
            if let Some(parent) = old.parent {
                self.read_children.remove(&(parent, old.id));
            }
        }
        for (object, value) in changes.objects {
            self.read_objects.insert(object.id, value);
            self.read_owners.insert((object.owner, object.id), 0);
            if let Some(parent) = object.parent {
                self.read_children.insert((parent, object.id), 0);
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
        self.read_version.set(changes.version);
    }
    pub(super) fn read_graph(&self) -> AccessGraph {
        AccessGraph { tree_version: *self.read_version.get(), ..AccessGraph::default() }
    }
    /// Fetch only this object's path and its grants. The existing evaluator still
    /// checks cycles, owner/space boundaries, trash, expiry and capability union.
    pub(super) fn read_path(&self, graph: &mut AccessGraph, id: u64) -> DriveResult<()> {
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
                graph.grants.insert(grant.id, grant);
            }
            next = object.parent;
            graph.objects.insert(id, object);
        }
        Ok(())
    }
    pub(super) fn indexed_graph(&self, id: u64) -> DriveResult<AccessGraph> {
        let mut graph = self.read_graph();
        self.read_path(&mut graph, id)?;
        Ok(graph)
    }
    pub(super) fn root_ids(&self, actor: &Actor) -> DriveResult<BTreeSet<u64>> {
        let mut ids = BTreeSet::new();
        if actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() {
            return Err(DriveError::Inaccessible);
        }
        if let Some(bytes) = self.accounts.get(&actor.sponsor) {
            let account: Account = decoded(&bytes)?;
            ids.insert(account.id);
        }
        for entry in self.read_subjects.range((actor.sponsor, 0)..=(actor.sponsor, u64::MAX)) {
            let grant: AccessGrant = decoded(&entry.value())?;
            if grant.active {
                ids.insert(grant.root);
            }
        }
        Ok(ids)
    }
    pub(super) fn indexed_page(&self, actor: &Actor, parent: Option<u64>, cursor: Option<DriveCursor>, limit: u32, now: u64) -> DriveResult<DrivePage> {
        let mut graph = self.read_graph();
        let after = match cursor {
            Some(c) if c.actor == actor.sponsor && c.scope == parent && c.tree_version == graph.tree_version => c.after,
            Some(_) => return Err(DriveError::Conflict),
            None => 0,
        };
        if let Some(parent) = parent {
            self.read_path(&mut graph, parent)?;
            graph.require(actor, parent, Capabilities::READ, now, false)?;
            if graph.objects.get(&parent).is_none_or(|o| o.kind != ObjectKind::Folder) {
                return Err(DriveError::Inaccessible);
            }
        }
        let roots;
        let ids: Box<dyn Iterator<Item = u64> + '_> = if let Some(parent) = parent {
            Box::new(self.read_children.range((Excluded((parent, after)), Included((parent, u64::MAX)))).map(|e| e.key().1))
        } else {
            roots = self.root_ids(actor)?;
            Box::new(roots.into_iter().filter(move |id| *id > after))
        };
        let limit = limit.clamp(1, 50) as usize;
        let mut entries = Vec::new();
        for id in ids {
            match self.read_path(&mut graph, id).and_then(|()| self.entry(&graph, actor, id, now)) {
                Ok(entry) => entries.push(entry),
                Err(DriveError::Inaccessible) => continue,
                Err(error) => return Err(error),
            }
            if entries.len() > limit {
                break;
            }
        }
        let more = entries.len() > limit;
        entries.truncate(limit);
        let next = if more {
            entries.last().map(|entry| DriveCursor { scope: parent, actor: actor.sponsor, tree_version: graph.tree_version, after: entry.object.id })
        } else {
            None
        };
        Ok(DrivePage { entries, next })
    }
    pub(super) fn visible_graph(&self, actor: &Actor, now: u64) -> DriveResult<AccessGraph> {
        let mut graph = self.read_graph();
        let mut pending = Vec::new();
        for id in self.root_ids(actor)? {
            if self.read_path(&mut graph, id).is_ok() && graph.require(actor, id, Capabilities::READ, now, true).is_ok() {
                pending.push(id);
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            self.read_path(&mut graph, id)?;
            pending.extend(self.read_children.range((id, 0)..=(id, u64::MAX)).map(|e| e.key().1));
        }
        Ok(graph)
    }
    pub(super) fn drive_catalog(&self, actor: &Actor, root: u64, now: u64) -> DriveResult<DriveCatalog> {
        use std::collections::{BTreeMap, VecDeque};
        const LIMIT: usize = 200;
        let first = self.drive_get(actor, root, now)?;
        let mut entries = BTreeMap::from([(root, first)]);
        let mut pending = VecDeque::from([root]);
        let mut complete_folders = Vec::new();
        while let Some(parent) = pending.pop_front() {
            if entries.get(&parent).is_none_or(|e| e.object.kind != ObjectKind::Folder) {
                continue;
            }
            if entries.len() >= LIMIT {
                // Even at the byte/entry budget, an empty folder can be proven
                // complete without adding another catalog entry.
                if self.indexed_page(actor, Some(parent), None, 1, now)?.entries.is_empty() {
                    complete_folders.push(parent);
                }
                continue;
            }
            let mut cursor = None;
            loop {
                let remaining = LIMIT - entries.len();
                if remaining == 0 {
                    break;
                }
                let page = self.indexed_page(actor, Some(parent), cursor, remaining.min(50) as u32, now)?;
                for entry in page.entries {
                    let id = entry.object.id;
                    if entry.object.kind == ObjectKind::Folder {
                        pending.push_back(id);
                    }
                    entries.insert(id, entry);
                }
                cursor = page.next;
                if cursor.is_none() {
                    complete_folders.push(parent);
                    break;
                }
            }
        }
        Ok(DriveCatalog { entries: entries.into_values().collect(), complete_folders })
    }
    pub(crate) fn drive_browse(&self, actor: &Actor, parent: u64, cursor: Option<DriveCursor>, limit: u32, now: u64) -> DriveResult<DriveBrowse> {
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
        assert_eq!(view.catalog.entries.iter().map(|e| e.object.id).collect::<Vec<_>>(), vec![shared, child]);
        assert!(view.catalog.complete_folders.contains(&child));
        assert_eq!(view.folder.sharing, Some(SharingStatus { direct: true, inherited: false }));
        assert_eq!(view.page.entries[0].sharing, Some(SharingStatus { direct: false, inherited: true }));
        assert!(s.drive_browse(&b, root, None, 50, 5).is_err());
        assert!(s.drive_browse(&b, shared, None, 50, 100).is_err());
        let original = s.access_graph.get().clone();
        s.access_graph.set(b"not readable as a whole graph".to_vec());
        s.read_objects.insert(unrelated, b"must not be decoded".to_vec());
        assert_eq!(s.drive_browse(&b, shared, None, 50, 5).unwrap().page.entries.len(), 1);
        assert_eq!(s.drive_list(&b, None, None, 50, 5).unwrap().entries.len(), 2);
        let own =
            s.drive_create_folder(&a, CreateFolder { parent: root, name: "isolated write".into(), request_id: "index-isolated-write".into() }, 6).unwrap();
        assert_eq!(own.object.owner, a.sponsor);
        assert!(s.drive_initialize(actor(4).sponsor, 6).is_ok());
        s.access_graph.set(original);
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
            for id in root.id + 1..root.id + count {
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
    fn legacy_graph_migrates_once_without_changing_ids_or_grants() {
        let memory = VectorMemory::default();
        let a = actor(1);
        let mut s = Store::new(memory.clone());
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        let child = s.drive_create_folder(&a, CreateFolder { parent: root, name: "preserved".into(), request_id: "migration-folder-1".into() }, 2).unwrap();
        s.access_graph.set(encoded(&s.graph_for_owner(a.sponsor).unwrap()).unwrap());
        // Recreate the pre-index stable schema: memory 9 has data, 14+ empty.
        s.read_objects.clear_new();
        s.read_children.clear_new();
        s.read_grants.clear_new();
        s.read_subjects.clear_new();
        s.read_owners.clear_new();
        s.read_ready.set(0);
        drop(s);
        let s = Store::new(memory.clone());
        assert_eq!(s.drive_browse(&a, root, None, 50, 3).unwrap().page.entries[0].object, child.object);
        drop(s);
        assert_eq!(Store::new(memory).drive_get(&a, child.object.id, 4).unwrap().object, child.object);
    }
}

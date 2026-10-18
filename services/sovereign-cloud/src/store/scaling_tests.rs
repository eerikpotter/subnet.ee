#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Behavioral and storage-cost checks for the partition boundary. These are not
//! a production throughput benchmark or a claim of 100,000-user capacity.
use super::collaboration::{decoded, encoded};
use super::*;
use crate::permissions::Actor;
use ic_stable_structures::VectorMemory;
use sovereign_cloud::drive::*;
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

#[derive(Clone, Default)]
struct MeteredMemory {
    inner: VectorMemory,
    reads: Rc<Cell<u64>>,
    writes: Rc<Cell<u64>>,
}
impl Memory for MeteredMemory {
    fn size(&self) -> u64 {
        self.inner.size()
    }
    fn grow(&self, pages: u64) -> i64 {
        self.inner.grow(pages)
    }
    fn read(&self, offset: u64, dst: &mut [u8]) {
        self.reads.set(self.reads.get() + 1);
        self.inner.read(offset, dst);
    }
    fn write(&self, offset: u64, src: &[u8]) {
        self.writes.set(self.writes.get() + 1);
        self.inner.write(offset, src);
    }
}
impl MeteredMemory {
    fn reset(&self) {
        self.reads.set(0);
        self.writes.set(0);
    }
}
fn actor(n: u8) -> Actor {
    Actor::user(Principal::self_authenticating([n; 32])).unwrap()
}
fn query(scope: BrowseScope) -> BrowseQuery {
    BrowseQuery {
        scope,
        text: String::new(),
        file_type: String::new(),
        owner: None,
        modified_after: None,
        modified_before: None,
        sort: SortField::Name,
        descending: false,
    }
}
fn folder<M: Memory>(s: &mut Store<M>, a: &Actor, parent: ObjectId, name: &str) -> DriveEntry {
    s.drive_create_folder(a, CreateFolder { parent, name: name.into(), request_id: format!("scaling-folder-{name}") }, 2).unwrap()
}
fn rename<M: Memory>(s: &mut Store<M>, a: &Actor, entry: &DriveEntry, name: &str) {
    s.drive_organize(
        a,
        OrganizeRequest {
            object: entry.object.id,
            expected_version: entry.object.version,
            change: OrganizationChange::Rename(name.into()),
            request_id: format!("scaling-rename-{name}"),
        },
        3,
    )
    .unwrap();
}
#[test]
fn uuid_generation_is_canonical_unique_and_not_reseeded_by_racing_initialization() {
    let mut s = Store::new(VectorMemory::default());
    let mut ids = BTreeSet::new();
    for _ in 0..100_000 {
        let id = s.next_object_id().unwrap();
        assert!(ids.insert(id));
        assert_eq!(id.to_string().parse::<ObjectId>().unwrap(), id);
        assert_eq!(id.as_bytes()[6] >> 4, 4);
        assert_eq!(id.as_bytes()[8] >> 6, 2);
    }
    s.seed_ids([42; 32]);
    assert!(!ids.contains(&s.next_object_id().unwrap()));
}
#[test]
fn rename_and_first_page_do_not_read_an_owners_entire_catalog() {
    let memory = MeteredMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(11);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object;
    let target = folder(&mut s, &a, root.id, "target");
    // Compare two populated B-trees, not a one-node tree against a mature one.
    let mut baseline = s.graph_for_object(root.id).unwrap();
    for n in 0..200 {
        let id = s.next_object_id().unwrap();
        baseline.objects.insert(id, DriveObject { id, parent: Some(root.id), name: format!("item-{n:05}"), ..root.clone() });
    }
    s.persist_graph(&baseline).unwrap();
    memory.reset();
    rename(&mut s, &a, &target, "before-growth");
    let small = (memory.reads.get(), memory.writes.get());
    let mut graph = s.graph_for_object(root.id).unwrap();
    // Fixture bypasses the temporary product quota, not the real indexes.
    for n in 200..5_000 {
        let id = s.next_object_id().unwrap();
        graph.objects.insert(id, DriveObject { id, parent: Some(root.id), name: format!("item-{n:05}"), ..root.clone() });
    }
    s.persist_graph(&graph).unwrap();
    let current = s.drive_get(&a, target.object.id, 3).unwrap();
    memory.reset();
    rename(&mut s, &a, &current, "after-growth");
    let large = (memory.reads.get(), memory.writes.get());
    println!("Stable-memory operations (reads, writes), 200 items: {small:?}; 5000 items: {large:?}");
    assert!(large.0 < small.0 * 4, "rename reads grew from {small:?} to {large:?}");
    assert!(large.1 < small.1 * 4, "rename writes grew from {small:?} to {large:?}");
    memory.reset();
    let first = s.drive_query(&a, query(BrowseScope::Folder(root.id)), None, 50, 4).unwrap();
    assert_eq!(first.items.len(), 50);
    assert!(first.next.is_some());
    let first_reads = memory.reads.get();
    memory.reset();
    let next = s.drive_query(&a, query(BrowseScope::Folder(root.id)), first.next, 50, 4).unwrap();
    assert_eq!(next.items.len(), 50);
    assert!(memory.reads.get() < first_reads * 3);
    // Even unreadable bytes elsewhere in the same owner never affect a rename.
    let unrelated = next.items[0].entry.object.id;
    s.read_objects.insert(unrelated, b"unrelated corrupt fixture".to_vec());
    let current = s.drive_get(&a, target.object.id, 4).unwrap();
    rename(&mut s, &a, &current, "isolated-change");
}
#[test]
fn filtered_empty_pages_continue_in_global_order_without_disclosing_private_positions() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(12);
    let b = actor(13);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let shared = folder(&mut s, &a, root.id, "zzz-public");
    s.drive_grant(
        &a,
        GrantRequest {
            root: shared.object.id,
            id: None,
            expected_version: None,
            subject: b.subject.clone(),
            capabilities: Capabilities::READ,
            active: true,
            expires_at: None,
            request_id: "scaling-share-public".into(),
        },
        3,
    )
    .unwrap();
    let mut graph = s.graph_for_object(root.id).unwrap();
    let mut private = Vec::new();
    for n in 0..1_100 {
        let id = s.next_object_id().unwrap();
        private.push(id);
        graph.objects.insert(id, DriveObject { id, parent: Some(root.id), name: format!("private-{n:05}"), ..root.clone() });
    }
    s.persist_graph(&graph).unwrap();
    let q = query(BrowseScope::All);
    let first = s.drive_query(&b, q.clone(), None, 50, 4).unwrap();
    assert!(first.items.is_empty());
    let cursor = first.next.unwrap();
    let bytes = serde_json::to_string(&cursor).unwrap();
    assert!(!bytes.contains("private"));
    for id in private {
        assert!(!bytes.contains(&id.to_string()));
    }
    let mut altered = cursor.clone();
    altered.token[20] ^= 1;
    assert_eq!(s.drive_query(&b, q.clone(), Some(altered), 50, 4).unwrap_err(), DriveError::Conflict);
    assert_eq!(s.drive_query(&a, q.clone(), Some(cursor.clone()), 50, 4).unwrap_err(), DriveError::Conflict);
    for token in [Vec::new(), vec![0; 5000]] {
        assert_eq!(s.drive_query(&b, q.clone(), Some(PageCursor { token }), 50, 4).unwrap_err(), DriveError::Conflict);
    }
    drop(s);
    let s = Store::new(memory);
    let second = s.drive_query(&b, q.clone(), Some(cursor), 50, 4).unwrap();
    assert!(second.items.is_empty());
    let third = s.drive_query(&b, q, second.next, 50, 4).unwrap();
    assert_eq!(third.items.iter().map(|i| i.entry.object.id).collect::<Vec<_>>(), vec![shared.object.id]);
    assert!(third.next.is_none());
}
#[test]
fn folder_cursors_ignore_siblings_and_other_accounts_but_reject_local_and_acl_changes() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(14);
    let b = actor(15);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let other_root = s.drive_initialize(b.sponsor, 1).unwrap().object.id;
    let work = folder(&mut s, &a, root, "work");
    let sibling = folder(&mut s, &a, root, "sibling");
    for name in ["one", "two", "three"] {
        folder(&mut s, &a, work.object.id, name);
    }
    let q = query(BrowseScope::Folder(work.object.id));
    let cursor = s.drive_query(&a, q.clone(), None, 1, 3).unwrap().next;
    folder(&mut s, &b, other_root, "unrelated-account");
    folder(&mut s, &a, sibling.object.id, "unrelated-sibling");
    assert!(s.drive_query(&a, q.clone(), cursor.clone(), 1, 4).is_ok());
    folder(&mut s, &a, work.object.id, "local-change");
    assert_eq!(s.drive_query(&a, q.clone(), cursor, 1, 4).unwrap_err(), DriveError::Conflict);
    let cursor = s.drive_query(&a, q.clone(), None, 1, 4).unwrap().next;
    s.drive_grant(
        &a,
        GrantRequest {
            root: work.object.id,
            id: None,
            expected_version: None,
            subject: b.subject.clone(),
            capabilities: Capabilities::READ,
            active: true,
            expires_at: None,
            request_id: "scaling-grant-folder".into(),
        },
        5,
    )
    .unwrap();
    assert_eq!(s.drive_query(&a, q, cursor, 1, 6).unwrap_err(), DriveError::Conflict);
    // Counts are derived indexes, never an ID-to-owner alias table.
    let account: Account = decoded(&s.accounts.get(&a.sponsor).unwrap()).unwrap();
    assert_eq!(account.personal_space, root);
    assert!(encoded(&account).unwrap().len() < 400);
}

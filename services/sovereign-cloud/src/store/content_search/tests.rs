use super::*;
use ic_stable_structures::VectorMemory;
fn actor(n: u8) -> Actor {
    Actor::user(Principal::self_authenticating([n; 32])).unwrap()
}
fn file(s: &mut Store<VectorMemory>, a: &Actor, parent: ObjectId, name: &str, text: &str) -> DriveEntry {
    s.drive_write_inline(
        a,
        InlineFile {
            parent,
            name: name.into(),
            bytes: text.as_bytes().to_vec(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            request_id: format!("search-file-{name}-request"),
        },
        2,
    )
    .unwrap()
}
fn query(s: &Store<VectorMemory>, a: &Actor, text: &str) -> ContentPage {
    s.content_query(a, ContentQuery { text: text.into(), folder: None, cursor: None, limit: 50 }, 50).unwrap()
}
fn process(s: &mut Store<VectorMemory>, f: &DriveEntry) {
    s.content_process(f.object.id, f.latest.as_ref().unwrap().id).unwrap();
}
fn grant(s: &mut Store<VectorMemory>, a: &Actor, b: &Actor, root: ObjectId) -> AccessGrant {
    s.drive_grant(
        a,
        GrantRequest {
            root,
            id: None,
            subject: b.subject.clone(),
            capabilities: Capabilities::READ,
            expires_at: Some(100),
            active: true,
            expected_version: None,
            request_id: format!("search-grant-{root}"),
        },
        3,
    )
    .unwrap()
}
#[test]
fn indexes_revision_not_filename_and_revocation_expiry_trash_and_reopen_filter_every_hit() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(1);
    let b = actor(2);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let f = file(&mut s, &a, root, "Report.txt", "Meretuul projekt kinnitatud");
    assert_eq!(s.content_status(&a, f.object.id, 3).unwrap().state, IndexState::Pending);
    assert!(query(&s, &a, "meretuul").hits.is_empty());
    process(&mut s, &f);
    assert_eq!(query(&s, &a, "MERETUUL kinnitatud").hits.len(), 1);
    assert!(query(&s, &a, "Report").hits.is_empty());
    assert!(query(&s, &b, "Meretuul").hits.is_empty());
    let g = grant(&mut s, &a, &b, f.object.id);
    assert_eq!(query(&s, &b, "Meretuul").hits[0].snippet, "Meretuul projekt kinnitatud");
    s.drive_hide_shared(&b, f.object.id, true, 4).unwrap();
    assert!(query(&s, &b, "Meretuul").hits.is_empty());
    s.drive_hide_shared(&b, f.object.id, false, 5).unwrap();
    assert_eq!(query(&s, &b, "Meretuul").hits.len(), 1);
    assert!(s.content_query(&b, ContentQuery { text: "Meretuul".into(), folder: None, cursor: None, limit: 50 }, 100).unwrap().hits.is_empty());
    assert!(s.content_status(&b, root, 50).is_err());
    assert!(s.content_rebuild(&b, f.object.id, 50).is_err());
    s.drive_grant(
        &a,
        GrantRequest {
            root: f.object.id,
            id: Some(g.id),
            subject: b.subject.clone(),
            capabilities: Capabilities::READ,
            expires_at: None,
            active: false,
            expected_version: Some(g.version),
            request_id: "search-revoke-request".into(),
        },
        6,
    )
    .unwrap();
    assert!(query(&s, &b, "Meretuul").hits.is_empty());
    let mut s = Store::new(memory);
    assert_eq!(query(&s, &a, "meretuul").hits.len(), 1);
    s.drive_organize(
        &a,
        OrganizeRequest {
            object: f.object.id,
            expected_version: f.object.version,
            change: OrganizationChange::Trash,
            request_id: "search-trash-request".into(),
        },
        7,
    )
    .unwrap();
    assert!(query(&s, &a, "meretuul").hits.is_empty());
}
#[test]
fn late_worker_never_republishes_old_revision_and_failed_job_does_not_block_next() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let first = file(&mut s, &a, root, "First.txt", "old words");
    process(&mut s, &first);
    let bytes = b"new words".to_vec();
    let stage = s
        .drive_begin(
            &a,
            DriveUploadRequest {
                target: UploadTarget::Replace { file: first.object.id, expected_revision: first.latest.as_ref().unwrap().id },
                bytes: bytes.len() as u64,
                sha256: Sha256::digest(&bytes).to_vec(),
                request_id: "search-replace-request".into(),
                automatic: None,
            },
            4,
        )
        .unwrap();
    s.drive_put(&a, stage.id, 0, bytes, 4).unwrap();
    s.drive_commit(&a, stage.id, 5).unwrap();
    assert!(query(&s, &a, "old").hits.is_empty());
    process(&mut s, &first);
    assert!(query(&s, &a, "old").hits.is_empty());
    s.content_process(first.object.id, stage.id).unwrap();
    assert_eq!(query(&s, &a, "new").hits.len(), 1);
    let next = file(&mut s, &a, root, "Second.txt", "second document");
    let third = file(&mut s, &a, root, "Third.txt", "third document");
    let (id, rev) = s.content_claim(10).unwrap();
    assert_eq!(id, next.object.id);
    assert!(s.content_claim(11).is_none());
    s.content_failed(id, rev).unwrap();
    assert_eq!(s.content_status(&a, id, 12).unwrap().state, IndexState::Failed);
    assert!(s.content_rebuild(&a, id, 12).is_err());
    assert_eq!(s.content_claim(12).unwrap().0, third.object.id);
}
#[test]
fn bounded_empty_continuation_is_private_bound_to_query_and_actor() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let b = actor(2);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let f = file(&mut s, &a, root, "Visible.txt", "needle");
    process(&mut s, &f);
    grant(&mut s, &a, &b, f.object.id);
    // Old derived positions can remain while authority/current heads have changed.
    for n in 1..=150 {
        s.search.postings.insert(Posting(root, "needle".into(), crate::test_id(n)), (999, 0));
    }
    let request = ContentQuery { text: "needle".into(), folder: None, cursor: None, limit: 50 };
    let first = s.content_query(&b, request.clone(), 50).unwrap();
    assert!(first.hits.is_empty());
    assert!(first.next.is_some());
    assert!(s.content_query(&a, ContentQuery { cursor: first.next.clone(), ..request.clone() }, 50).is_err());
    assert!(s.content_query(&b, ContentQuery { text: "other".into(), cursor: first.next.clone(), ..request.clone() }, 50).is_err());
    let last = s.content_query(&b, ContentQuery { cursor: first.next, ..request }, 50).unwrap();
    assert_eq!(last.hits.len(), 1);
    assert!(last.next.is_none());
}
#[test]
fn queued_backfill_survives_reopen_and_scope_search_never_crosses_folders() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(1);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let folder = s.drive_create_folder(&a, CreateFolder { parent: root, name: "Scope".into(), request_id: "search-scope-folder".into() }, 2).unwrap();
    let one = file(&mut s, &a, folder.object.id, "Inside.txt", "shared keyword");
    let two = file(&mut s, &a, root, "Outside.txt", "shared keyword");
    s.search.pending.clear_new();
    s.search.queue.clear_new();
    s.content_backfill().unwrap();
    let mut s = Store::new(memory);
    while let Some((id, rev)) = s.content_claim(50) {
        s.content_process(id, rev).unwrap();
    }
    assert_eq!(query(&s, &a, "keyword").hits.len(), 2);
    let page = s.content_query(&a, ContentQuery { text: "keyword".into(), folder: Some(folder.object.id), cursor: None, limit: 50 }, 50).unwrap();
    assert_eq!(page.hits[0].entry.object.id, one.object.id);
    assert_eq!(page.hits.len(), 1);
    s.search.forget(two.object.id);
    assert_eq!(query(&s, &a, "keyword").hits.len(), 1);
}

#[test]
fn team_search_uses_current_membership_not_the_file_founder() {
    use sovereign_cloud::spaces::*;
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let b = actor(2);
    s.drive_initialize(a.sponsor, 1).unwrap();
    s.drive_initialize(b.sponsor, 1).unwrap();
    let team = s.space_create(&a, CreateSpace { name: "Search team".into(), request_id: "search-team-create".into() }, 2).unwrap().space;
    let joined = s
        .space_change_member(
            &a,
            ChangeMember {
                space: team.id,
                principal: b.sponsor,
                role: Some(SpaceRole::Manager),
                expected_version: team.version,
                request_id: "search-team-join".into(),
            },
        )
        .unwrap();
    let f = file(&mut s, &a, team.id, "Team.txt", "teamneedle canonical text");
    process(&mut s, &f);
    assert_eq!(query(&s, &b, "teamneedle").hits.len(), 1);
    s.space_change_member(
        &b,
        ChangeMember { space: team.id, principal: a.sponsor, role: None, expected_version: joined.version, request_id: "search-team-remove-founder".into() },
    )
    .unwrap();
    assert!(query(&s, &a, "teamneedle").hits.is_empty());
    assert!(s.content_status(&a, f.object.id, 50).is_err());
    assert_eq!(query(&s, &b, "teamneedle").hits[0].entry.object.id, f.object.id);
}

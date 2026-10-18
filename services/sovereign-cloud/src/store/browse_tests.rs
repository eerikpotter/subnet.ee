#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use ic_stable_structures::VectorMemory;
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
fn folder(s: &mut Store<VectorMemory>, a: &Actor, parent: ObjectId, name: &str, n: u64) -> DriveEntry {
    s.drive_create_folder(a, CreateFolder { parent, name: name.into(), request_id: format!("browse-folder-{n:016}") }, n).unwrap()
}
fn file(s: &mut Store<VectorMemory>, a: &Actor, parent: ObjectId, name: &str, n: u64) -> DriveEntry {
    s.drive_write_inline(
        a,
        InlineFile {
            parent,
            name: name.into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: vec![42; n as usize],
            request_id: format!("browse-file-{n:016}"),
        },
        n,
    )
    .unwrap()
}
fn grant(s: &mut Store<VectorMemory>, a: &Actor, b: &Actor, root: ObjectId) -> AccessGrant {
    s.drive_grant(
        a,
        GrantRequest {
            root,
            id: None,
            expected_version: None,
            subject: b.subject.clone(),
            capabilities: Capabilities::READ,
            active: true,
            expires_at: Some(1000),
            request_id: "browse-sharing-0001".into(),
        },
        4,
    )
    .unwrap()
}
#[test]
fn sort_is_global_across_pages_and_cursors_bind_identity_query_and_mutations() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let b = actor(2);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    for n in (1..=65).rev() {
        folder(&mut s, &a, root, &format!("Folder {n:02}"), n + 1);
    }
    let q = query(BrowseScope::Folder(root));
    let first = s.drive_query(&a, q.clone(), None, 50, 100).unwrap();
    assert_eq!(first.items[0].entry.object.name, "Folder 01");
    assert_eq!(first.items[49].entry.object.name, "Folder 50");
    let next = s.drive_query(&a, q.clone(), first.next.clone(), 50, 100).unwrap();
    assert_eq!(next.items.len(), 15);
    assert_eq!(next.items[0].entry.object.name, "Folder 51");
    assert!(next.next.is_none());
    assert!(matches!(s.drive_query(&b, q.clone(), first.next.clone(), 50, 100), Err(DriveError::Conflict)));
    let mut desc = q.clone();
    desc.descending = true;
    assert!(matches!(s.drive_query(&a, desc.clone(), first.next.clone(), 50, 100), Err(DriveError::Conflict)));
    assert_eq!(s.drive_query(&a, desc, None, 1, 100).unwrap().items[0].entry.object.name, "Folder 65");
    folder(&mut s, &a, root, "A new folder", 90);
    assert!(matches!(s.drive_query(&a, q, first.next, 50, 100), Err(DriveError::Conflict)));
}
#[test]
fn search_finds_unloaded_shared_descendants_without_disclosing_private_ancestors() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let b = actor(2);
    let c = actor(3);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let unrelated = s.drive_initialize(c.sponsor, 1).unwrap().object.id;
    let private = folder(&mut s, &a, root, "Secret parent", 2).object.id;
    let shared = folder(&mut s, &a, private, "Shared", 3).object.id;
    let nested = folder(&mut s, &a, shared, "Nested", 4).object.id;
    let f = file(&mut s, &a, nested, "ÕUN report.txt", 5);
    file(&mut s, &a, private, "õun hidden.txt", 6);
    let g = grant(&mut s, &a, &b, shared);
    // A caller-scoped search must never decode another owner's catalog.
    s.read_objects.insert(unrelated, b"unrelated invalid data".to_vec());
    let mut q = query(BrowseScope::All);
    q.text = "o\u{303}un".into();
    q.file_type = "text".into();
    q.owner = Some(a.sponsor);
    q.modified_after = Some(5);
    q.modified_before = Some(6);
    let page = s.drive_query(&b, q.clone(), None, 50, 10).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].entry.object.id, f.object.id);
    assert!(page.breadcrumbs.is_empty());
    let view = s.drive_query(&b, query(BrowseScope::Folder(nested)), None, 50, 10).unwrap();
    assert_eq!(view.breadcrumbs.iter().map(|e| e.object.id).collect::<Vec<_>>(), vec![shared, nested]);
    let restricted = s.drive_query(&b, query(BrowseScope::Subtree(private)), None, 50, 10);
    assert!(matches!(restricted, Err(DriveError::Inaccessible)));
    q.file_type = "image".into();
    assert!(s.drive_query(&b, q.clone(), None, 50, 10).unwrap().items.is_empty());
    q.file_type = "text".into();
    assert!(s.drive_query(&b, q.clone(), None, 50, 1000).unwrap().items.is_empty());
    s.drive_grant(
        &a,
        GrantRequest {
            root: shared,
            id: Some(g.id),
            expected_version: Some(g.version),
            subject: b.subject.clone(),
            capabilities: Capabilities::READ,
            active: false,
            expires_at: None,
            request_id: "browse-revoke-0001".into(),
        },
        11,
    )
    .unwrap();
    assert!(s.drive_query(&b, q, None, 50, 12).unwrap().items.is_empty());
}
#[test]
fn stars_recents_preferences_are_private_persistent_and_never_authority() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(1);
    let b = actor(2);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let first = file(&mut s, &a, root, "first", 2);
    let second = file(&mut s, &a, root, "second", 3);
    grant(&mut s, &a, &b, root);
    s.drive_star(&b, first.object.id, true, 5).unwrap();
    s.drive_get(&b, first.object.id, 6).unwrap();
    assert!(s.drive_query(&b, query(BrowseScope::Recent), None, 50, 7).unwrap().items.is_empty());
    s.drive_open(&b, second.object.id, 8).unwrap();
    s.drive_open(&b, first.object.id, 9).unwrap();
    let preferences = BrowsePreferences { view: ViewMode::Grid, sort: SortField::Modified, descending: true };
    s.drive_set_preferences(&b, preferences.clone()).unwrap();
    drop(s);
    let mut s = Store::new(memory);
    assert_eq!(s.drive_preferences(&b).unwrap(), preferences);
    assert_eq!(s.drive_preferences(&a).unwrap(), BrowsePreferences::default());
    assert_eq!(s.drive_query(&b, query(BrowseScope::Starred), None, 50, 10).unwrap().items[0].entry.object.id, first.object.id);
    assert!(s.drive_query(&a, query(BrowseScope::Starred), None, 50, 10).unwrap().items.is_empty());
    let mut recent = query(BrowseScope::Recent);
    recent.sort = SortField::Opened;
    recent.descending = true;
    let page = s.drive_query(&b, recent.clone(), None, 1, 10).unwrap();
    assert_eq!(page.items[0].entry.object.id, first.object.id);
    s.drive_star(&b, first.object.id, false, 10).unwrap();
    assert!(matches!(s.drive_query(&b, recent, page.next, 1, 10), Err(DriveError::Conflict)));
    s.drive_organize(
        &a,
        OrganizeRequest {
            object: second.object.id,
            expected_version: second.object.version,
            change: OrganizationChange::Trash,
            request_id: "browse-trash-0001".into(),
        },
        11,
    )
    .unwrap();
    assert!(s.drive_open(&b, second.object.id, 12).is_err());
    assert!(s.drive_star(&b, second.object.id, true, 12).is_err());
    assert_eq!(s.drive_query(&a, query(BrowseScope::Trash), None, 50, 12).unwrap().items[0].entry.object.id, second.object.id);
    assert!(s.drive_query(&b, query(BrowseScope::Recent), None, 50, 1000).unwrap().items.is_empty());
}
#[test]
fn modified_size_filters_and_expiring_cursor_follow_current_metadata() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let b = actor(2);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let small = file(&mut s, &a, root, "z small", 2);
    let large = file(&mut s, &a, root, "a large", 5);
    grant(&mut s, &a, &b, root);
    let mut q = query(BrowseScope::All);
    q.sort = SortField::Size;
    q.descending = true;
    let page = s.drive_query(&b, q.clone(), None, 1, 10).unwrap();
    assert_eq!(page.items[0].entry.object.id, large.object.id);
    assert!(matches!(s.drive_query(&b, q.clone(), page.next, 1, 1000), Err(DriveError::Conflict)));
    q.sort = SortField::Modified;
    s.drive_organize(
        &a,
        OrganizeRequest {
            object: small.object.id,
            expected_version: small.object.version,
            change: OrganizationChange::Rename("renamed".into()),
            request_id: "browse-rename-0001".into(),
        },
        20,
    )
    .unwrap();
    assert_eq!(s.drive_query(&b, q, None, 1, 21).unwrap().items[0].entry.object.id, small.object.id);
    let mut bad = query(BrowseScope::All);
    bad.text = "x".repeat(161);
    assert!(matches!(s.drive_query(&a, bad, None, 50, 10), Err(DriveError::InvalidInput(_))));
}

#[test]
fn recent_tracking_cannot_bypass_the_star_storage_bound() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let entry = file(&mut s, &a, root, "Opened before starring", 2);
    let bytes = encoded(&ViewerItem { starred: true, ..ViewerItem::default() }).unwrap();
    // Model retained personal stars after their old targets have been purged.
    for id in 100_000..100_000 + MAX_CANDIDATES as u64 {
        s.viewer_items.insert((a.sponsor, crate::test_id(id)), bytes.clone());
    }
    s.viewer_settings.insert(
        a.sponsor,
        encoded(&ViewerSettings { item_count: MAX_CANDIDATES as u64, starred_count: MAX_CANDIDATES as u64, ..ViewerSettings::default() }).unwrap(),
    );
    s.drive_open(&a, entry.object.id, 3).unwrap();
    assert!(matches!(s.drive_star(&a, entry.object.id, true, 4), Err(DriveError::Limit(_))));
    assert!(!s.viewer_item(a.sponsor, entry.object.id).unwrap().starred);
    assert_eq!(s.drive_query(&a, query(BrowseScope::Recent), None, 50, 5).unwrap().items.len(), 1);
}

#[test]
fn suggestions_are_private_truthful_and_successful_opens_are_coalesced() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(1);
    let b = actor(2);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let f = file(&mut s, &a, root, "A plan.txt", 2);
    let folder = folder(&mut s, &a, root, "Projects", 3);
    grant(&mut s, &a, &b, root);
    let first = s.drive_suggested_home(&b, 10).unwrap();
    assert_eq!(first.files[0].reason, SuggestionReason::RecentlyCreated);
    assert!(s.viewer_item(b.sponsor, f.object.id).unwrap().opened_at.is_none());
    assert!(s.drive_acknowledge_open(&b, f.object.id, Some(999_999), 10).is_err());
    assert!(s.drive_acknowledge_open(&b, f.object.id, None, 10).is_err());
    let revision = f.latest.as_ref().unwrap().id;
    s.drive_acknowledge_open(&b, f.object.id, Some(revision), 10).unwrap();
    let version = s.viewer(&b).unwrap().version;
    s.drive_acknowledge_open(&b, f.object.id, Some(revision), 20).unwrap();
    assert_eq!(s.viewer(&b).unwrap().version, version);
    assert_eq!(s.viewer_item(b.sponsor, f.object.id).unwrap().opened_at, Some(10));
    s.drive_acknowledge_open(&b, folder.object.id, None, 20).unwrap();
    assert_eq!(s.drive_suggested_home(&b, 21).unwrap().folders[0].reason, SuggestionReason::RecentlyOpened);
    assert_eq!(s.drive_query(&b, query(BrowseScope::Recent), None, 50, 21).unwrap().items.len(), 1);
    assert_eq!(s.drive_suggested_home(&a, 21).unwrap().files[0].reason, SuggestionReason::RecentlyCreated);
    // Renaming is not a new content revision.
    s.drive_organize(
        &a,
        OrganizeRequest {
            object: f.object.id,
            expected_version: f.object.version,
            change: OrganizationChange::Rename("Renamed.txt".into()),
            request_id: "suggestions-rename-0001".into(),
        },
        22,
    )
    .unwrap();
    assert_eq!(s.drive_suggested_home(&b, 23).unwrap().files[0].reason, SuggestionReason::RecentlyOpened);
    let bytes = b"A new revision".to_vec();
    let upload = s
        .drive_begin(
            &a,
            DriveUploadRequest {
                automatic: None,
                target: UploadTarget::Replace { file: f.object.id, expected_revision: revision },
                bytes: bytes.len() as u64,
                sha256: Sha256::digest(&bytes).to_vec(),
                request_id: "suggestions-revision-0001".into(),
            },
            24,
        )
        .unwrap();
    s.drive_put(&a, upload.id, 0, bytes, 25).unwrap();
    s.drive_commit(&a, upload.id, 26).unwrap();
    let home = s.drive_suggested_home(&b, 27).unwrap();
    assert_eq!(home.files[0].reason, SuggestionReason::UpdatedSinceOpened);
    assert_eq!(home.files[0].reason_at, 26);
    s.drive_star(&b, f.object.id, true, 28).unwrap();
    s.drive_dismiss_suggestion(&b, f.object.id, 29).unwrap();
    assert!(s.drive_suggested_home(&b, 30).unwrap().files.is_empty());
    assert_eq!(s.drive_query(&b, query(BrowseScope::Starred), None, 50, 30).unwrap().items.len(), 1);
    assert_eq!(s.drive_suggested_home(&a, 30).unwrap().files.len(), 1);
    drop(s);
    let s = Store::new(memory);
    assert_eq!(s.viewer_item(b.sponsor, f.object.id).unwrap().opened_revision, Some(revision));
    assert!(s.drive_suggested_home(&b, 30).unwrap().files.is_empty());
    // Expired access removes all derived views, even though private summaries remain.
    assert!(s.drive_suggested_home(&b, 1000).unwrap().files.is_empty());
    assert!(s.drive_query(&b, query(BrowseScope::Item(f.object.id)), None, 50, 1000).is_err());
}

#[test]
fn dismissal_expiry_and_summary_retention_preserve_stars_and_folder_cursors() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let f = file(&mut s, &a, root, "Starred.txt", 2);
    file(&mut s, &a, root, "Other.txt", 3);
    s.drive_star(&a, f.object.id, true, 4).unwrap();
    let first = s.drive_query(&a, query(BrowseScope::Folder(root)), None, 1, 5).unwrap();
    s.drive_acknowledge_open(&a, f.object.id, Some(f.latest.unwrap().id), 6).unwrap();
    assert!(s.drive_query(&a, query(BrowseScope::Folder(root)), first.next, 1, 7).is_ok());
    s.drive_dismiss_suggestion(&a, f.object.id, 8).unwrap();
    let expires = s.viewer_item(a.sponsor, f.object.id).unwrap().hidden_until.unwrap();
    s.drive_dismiss_suggestion(&a, f.object.id, 9).unwrap();
    assert_eq!(s.viewer_item(a.sponsor, f.object.id).unwrap().hidden_until, Some(expires));
    assert!(!s.drive_suggested_home(&a, expires - 1).unwrap().files.iter().any(|i| i.item.entry.object.id == f.object.id));
    assert!(s.drive_suggested_home(&a, expires).unwrap().files.iter().any(|i| i.item.entry.object.id == f.object.id));
    // Simulate obsolete summaries from purged items; mutation cleanup is bounded
    // and preserves intentional stars independently of access/dismissal history.
    for id in 1000..1300 {
        s.save_viewer_item(
            &a,
            crate::test_id(id),
            ViewerItem { color: None, starred: true, opened_at: Some(id), opened_revision: Some(id), hidden_until: Some(expires + id) },
            10,
        )
        .unwrap();
    }
    s.drive_acknowledge_open(&a, f.object.id, None, expires).unwrap_err();
    s.drive_dismiss_suggestion(&a, f.object.id, expires).unwrap();
    let items: Vec<ViewerItem> = s.viewer_items.range((a.sponsor, ObjectId::MIN)..=(a.sponsor, ObjectId::MAX)).map(|e| decoded(&e.value()).unwrap()).collect();
    assert_eq!(items.iter().filter(|i| i.opened_at.is_some()).count(), 200);
    assert_eq!(items.iter().filter(|i| i.hidden_until.is_some()).count(), 200);
    assert_eq!(items.iter().filter(|i| i.starred).count(), 301);
    assert!(items.iter().filter(|i| i.opened_at.is_none()).all(|i| i.opened_revision.is_none()));
}

#[test]
fn suggestions_revalidate_revocation_and_trash_and_do_not_read_unrelated_catalogs() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(1);
    let b = actor(2);
    let c = actor(3);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let unrelated = s.drive_initialize(c.sponsor, 1).unwrap().object.id;
    let private = folder(&mut s, &a, root, "Private", 2).object.id;
    let f = file(&mut s, &a, private, "Shared file", 3);
    let g = grant(&mut s, &a, &b, f.object.id);
    s.read_objects.insert(unrelated, b"not this viewer's catalog".to_vec());
    s.drive_star(&b, f.object.id, true, 5).unwrap();
    let home = s.drive_suggested_home(&b, 6).unwrap();
    assert!(home.folders.is_empty());
    assert_eq!(home.files.len(), 1);
    assert_eq!(home.files[0].reason, SuggestionReason::Starred);
    s.drive_grant(
        &a,
        GrantRequest {
            root: f.object.id,
            id: Some(g.id),
            expected_version: Some(g.version),
            subject: b.subject.clone(),
            capabilities: Capabilities::READ,
            active: false,
            expires_at: None,
            request_id: "suggestion-revoke-0001".into(),
        },
        7,
    )
    .unwrap();
    assert!(s.drive_suggested_home(&b, 8).unwrap().files.is_empty());
    assert!(s.drive_acknowledge_open(&b, f.object.id, f.latest.map(|r| r.id), 8).is_err());
    assert!(s.drive_dismiss_suggestion(&b, f.object.id, 8).is_err());
    s.drive_organize(
        &a,
        OrganizeRequest { object: private, expected_version: 1, change: OrganizationChange::Trash, request_id: "suggestion-trash-0001".into() },
        9,
    )
    .unwrap();
    assert!(s.drive_suggested_home(&a, 10).unwrap().files.is_empty());
}

#[test]
fn revision_removal_updates_size_order_and_invalidates_only_its_folder() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(81);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let first = file(&mut s, &a, root, "first", 2);
    let second = file(&mut s, &a, root, "second", 5);
    let mut q = query(BrowseScope::Folder(root));
    q.sort = SortField::Size;
    q.descending = true;
    assert_eq!(s.drive_query(&a, q.clone(), None, 1, 6).unwrap().items[0].entry.object.id, second.object.id);
    let bytes = b"a larger current revision";
    let upload = s
        .drive_begin(
            &a,
            DriveUploadRequest {
                automatic: None,
                target: UploadTarget::Replace { file: first.object.id, expected_revision: first.latest.unwrap().id },
                bytes: bytes.len() as u64,
                sha256: Sha256::digest(bytes).to_vec(),
                request_id: "size-index-replace".into(),
            },
            7,
        )
        .unwrap();
    s.drive_put(&a, upload.id, 0, bytes.to_vec(), 8).unwrap();
    s.drive_commit(&a, upload.id, 9).unwrap();
    let page = s.drive_query(&a, q.clone(), None, 1, 10).unwrap();
    assert_eq!(page.items[0].entry.object.id, first.object.id);
    s.remove_upload(a.sponsor, upload.id, 11).unwrap();
    assert_eq!(s.drive_query(&a, q.clone(), page.next, 1, 12).unwrap_err(), DriveError::Conflict);
    let items = s.drive_query(&a, q, None, 50, 12).unwrap().items;
    assert_eq!(items.iter().map(|i| i.entry.object.id).collect::<Vec<_>>(), vec![second.object.id, first.object.id]);
    assert_eq!(items[1].entry.latest.as_ref().unwrap().bytes, 2);
}

#[test]
fn opening_and_starring_do_not_decode_or_rewrite_unrelated_personal_records() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(82);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let first = file(&mut s, &a, root, "first", 2);
    let second = file(&mut s, &a, root, "second", 3);
    s.drive_star(&a, first.object.id, true, 4).unwrap();
    s.viewer_items.insert((a.sponsor, first.object.id), b"unrelated corrupt fixture".to_vec());
    s.drive_open(&a, second.object.id, 5).unwrap();
    s.drive_star(&a, second.object.id, true, 6).unwrap();
    s.drive_star(&a, second.object.id, false, 7).unwrap();
    assert_eq!(s.viewer(&a).unwrap().starred_count, 1);
    assert_eq!(s.viewer_opened.len(), 1);
}

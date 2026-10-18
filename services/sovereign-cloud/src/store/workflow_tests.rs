use super::*;
use crate::permissions::Actor;
use ic_stable_structures::VectorMemory;
use sovereign_cloud::drive::*;
fn actor(n: u8) -> Actor {
    Actor::user(Principal::self_authenticating([n; 32])).unwrap()
}
fn file(s: &mut Store<VectorMemory>, a: &Actor, parent: ObjectId, name: &str) -> DriveEntry {
    s.drive_write_inline(
        a,
        InlineFile {
            parent,
            name: name.into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: b"old".to_vec(),
            request_id: format!("workflow-file-{name}"),
        },
        11,
    )
    .unwrap()
}
fn grant(s: &mut Store<VectorMemory>, a: &Actor, f: ObjectId, b: &Actor) -> AccessGrant {
    s.drive_initialize(b.sponsor, 12).unwrap();
    s.drive_grant(
        a,
        GrantRequest {
            request_id: format!("workflow-share-{f}"),
            id: None,
            expected_version: None,
            root: f,
            subject: b.subject.clone(),
            capabilities: Capabilities { create: false, ..Capabilities::ALL },
            expires_at: Some(1000),
            active: true,
        },
        12,
    )
    .unwrap()
}
fn replace(s: &mut Store<VectorMemory>, a: &Actor, f: &DriveEntry, bytes: &[u8]) -> DriveUpload {
    let u = s
        .drive_begin(
            a,
            DriveUploadRequest {
                automatic: None,
                target: UploadTarget::Replace { file: f.object.id, expected_revision: f.latest.as_ref().unwrap().id },
                bytes: bytes.len() as u64,
                sha256: Sha256::digest(bytes).to_vec(),
                request_id: "workflow-replace-01".into(),
            },
            13,
        )
        .unwrap();
    s.drive_put(a, u.id, 0, bytes.to_vec(), 14).unwrap();
    s.drive_commit(a, u.id, 15).unwrap()
}
#[test]
fn activity_is_bounded_currently_authorized_and_receipt_deduplicated() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(41);
    let b = actor(42);
    let root = s.drive_initialize(a.sponsor, 10).unwrap();
    let f = file(&mut s, &a, root.object.id, "private");
    let g = grant(&mut s, &a, f.object.id, &b);
    assert_eq!(s.drive_activity(&a, None, None, 50, 13).unwrap().items.len(), 2);
    let reader = s.drive_activity(&b, Some(f.object.id), None, 50, 13).unwrap();
    assert_eq!(reader.items.len(), 1);
    assert!(matches!(reader.items[0].event.kind, ActivityKind::Created));
    assert!(s.drive_activity(&b, None, None, 50, 13).unwrap().items.is_empty());
    let request = OrganizeRequest {
        object: f.object.id,
        expected_version: 1,
        change: OrganizationChange::Rename("Current name".into()),
        request_id: "workflow-rename-01".into(),
    };
    s.drive_organize(&a, request.clone(), 14).unwrap();
    s.drive_organize(&a, request, 15).unwrap();
    let page = s.drive_activity(&b, Some(f.object.id), None, 1, 16).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].name, "Current name");
    let next = s.drive_activity(&b, Some(f.object.id), page.before, 50, 16).unwrap();
    assert_eq!(next.items.len(), 1);
    assert_eq!(next.items[0].name, "Current name");
    assert!(s.drive_activity(&b, Some(f.object.id), None, 50, 1001).is_err());
    s.drive_grant(
        &a,
        GrantRequest {
            root: g.root,
            id: Some(g.id),
            expected_version: Some(g.version),
            subject: g.subject,
            capabilities: g.capabilities,
            expires_at: g.expires_at,
            active: false,
            request_id: "workflow-revoke-01".into(),
        },
        17,
    )
    .unwrap();
    assert!(s.drive_activity(&b, Some(f.object.id), None, 50, 18).is_err());
    for i in 0..105 {
        let v = s.drive_get(&a, f.object.id, 20).unwrap();
        s.drive_organize(
            &a,
            OrganizeRequest {
                object: f.object.id,
                expected_version: v.object.version,
                change: OrganizationChange::Rename(format!("Name {i}")),
                request_id: format!("workflow-change-{i:04}"),
            },
            20 + i,
        )
        .unwrap();
    }
    let page = s.drive_activity(&a, Some(f.object.id), None, 50, 200).unwrap();
    let last = s.drive_activity(&a, Some(f.object.id), page.before, 50, 200).unwrap();
    assert_eq!(page.items.len() + last.items.len(), 100);
    assert!(last.before.is_none());
    drop(s);
    let s = Store::new(memory);
    assert_eq!(s.drive_activity(&a, None, None, 50, 200).unwrap().items.len(), 50);
}
#[test]
fn version_controls_preserve_head_pins_and_accounting_across_retry_and_reopen() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(43);
    let b = actor(44);
    let root = s.drive_initialize(a.sponsor, 10).unwrap();
    let f = file(&mut s, &a, root.object.id, "versions");
    let revision = f.latest.as_ref().unwrap().id;
    grant(&mut s, &a, f.object.id, &b);
    let u = replace(&mut s, &a, &f, b"new content");
    let label = RevisionLabelRequest {
        object: f.object.id,
        revision,
        expected_version: 0,
        label: "Release".into(),
        keep: false,
        request_id: "workflow-label-001".into(),
    };
    assert!(s.drive_label_revision(&b, label.clone(), 16).is_err());
    let props = s.drive_label_revision(&a, label.clone(), 16).unwrap();
    assert!(props.keep);
    assert_eq!(s.drive_label_revision(&a, label, 17).unwrap(), props);
    let current = s.drive_get(&a, f.object.id, 18).unwrap();
    let mut delete = RevisionDeleteRequest {
        object: f.object.id,
        revision,
        expected_object_version: current.object.version,
        expected_properties_version: props.version,
        request_id: "workflow-delete-01".into(),
    };
    assert!(s.drive_delete_revision(&a, delete.clone(), 19).is_err());
    assert!(s.remove_upload(a.sponsor, revision, 22).is_err());
    assert!(s.drive_delete_revision(&b, delete.clone(), 19).is_err());
    let props = s
        .drive_label_revision(
            &a,
            RevisionLabelRequest {
                object: f.object.id,
                revision,
                expected_version: props.version,
                label: "".into(),
                keep: false,
                request_id: "workflow-unpin-001".into(),
            },
            20,
        )
        .unwrap();
    delete.expected_properties_version = props.version;
    let before = s.drive_storage(&a).unwrap();
    assert_eq!((before.current_bytes, before.history_bytes, before.pending_bytes), (11, 3, 0));
    let receipt = s.drive_delete_revision(&a, delete.clone(), 21).unwrap();
    assert_eq!(s.drive_delete_revision(&a, delete.clone(), 22).unwrap(), receipt);
    assert_eq!(s.drive_storage(&a).unwrap().used_bytes, 11);
    assert!(s.drive_revision_chunk(&a, f.object.id, revision, 0, 22).is_err());
    assert_eq!(s.drive_revision_chunk(&a, f.object.id, u.id, 0, 22).unwrap(), b"new content");
    // A legacy request cannot debit the already-deleted version a second time.
    s.remove_upload(a.sponsor, revision, 22).unwrap();
    assert_eq!(s.drive_storage(&a).unwrap().used_bytes, 11);
    let mut head_delete = delete.clone();
    head_delete.request_id = "workflow-head-delete".into();
    head_delete.revision = u.id;
    head_delete.expected_object_version = receipt.version;
    head_delete.expected_properties_version = 0;
    assert!(s.drive_delete_revision(&a, head_delete, 23).is_err());
    drop(s);
    let mut s = Store::new(memory);
    assert_eq!(s.drive_delete_revision(&a, delete, 24).unwrap(), receipt);
    assert_eq!(s.drive_storage(&a).unwrap().used_bytes, 11);
}
#[test]
fn storage_separates_live_history_trash_and_pending_without_other_accounts() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(45);
    let b = actor(46);
    let root = s.drive_initialize(a.sponsor, 10).unwrap();
    let f = file(&mut s, &a, root.object.id, "storage");
    grant(&mut s, &a, f.object.id, &b);
    replace(&mut s, &a, &f, b"new content");
    assert_eq!(s.drive_storage(&b).unwrap().used_bytes, 0);
    let head = s.drive_get(&a, f.object.id, 18).unwrap();
    s.drive_organize(
        &a,
        OrganizeRequest {
            object: f.object.id,
            expected_version: head.object.version,
            change: OrganizationChange::Trash,
            request_id: "workflow-storage-trash".into(),
        },
        19,
    )
    .unwrap();
    let pending = s
        .drive_begin(
            &a,
            DriveUploadRequest {
                automatic: None,
                target: UploadTarget::New {
                    parent: root.object.id,
                    name: "Pending.txt".into(),
                    format: FileFormat { media_type: "text/plain".into(), craft: None },
                },
                bytes: 7,
                sha256: Sha256::digest(b"pending").to_vec(),
                request_id: "workflow-pending-001".into(),
            },
            20,
        )
        .unwrap();
    let v = s.drive_storage(&a).unwrap();
    assert_eq!((v.current_bytes, v.history_bytes, v.trash_bytes, v.pending_bytes, v.used_bytes), (0, 0, 14, 7, 21));
    s.drive_abort(&a, pending.id).unwrap();
    assert_eq!(s.drive_storage(&a).unwrap().pending_bytes, 0);
}

#[test]
fn shortcuts_have_independent_identity_and_never_grant_target_access() {
    let mut s = Store::new(VectorMemory::default());
    let a = actor(47);
    let b = actor(48);
    let c = actor(49);
    let root = s.drive_initialize(a.sponsor, 10).unwrap();
    let br = s.drive_initialize(b.sponsor, 10).unwrap();
    let f = file(&mut s, &a, root.object.id, "original");
    let g = grant(&mut s, &a, f.object.id, &b);
    let request = CreateShortcut { parent: br.object.id, target: f.object.id, name: "Reference".into(), request_id: "shortcut-create-001".into() };
    let link = s.drive_create_shortcut(&b, request.clone(), 13).unwrap();
    assert_eq!(s.drive_create_shortcut(&b, request, 14).unwrap().object.id, link.object.id);
    assert_eq!(link.object.owner, b.sponsor);
    assert!(link.latest.is_none());
    assert_eq!(s.drive_storage(&b).unwrap().used_bytes, 0);
    grant(&mut s, &b, link.object.id, &c);
    assert!(s.drive_get(&c, link.object.id, 15).is_ok());
    assert!(s.drive_shortcut_target(&c, link.object.id, 15).is_err());
    assert!(
        s.drive_create_shortcut(
            &b,
            CreateShortcut { parent: br.object.id, target: link.object.id, name: "Chain".into(), request_id: "shortcut-chain-denied".into() },
            16
        )
        .is_err()
    );
    s.drive_grant(
        &a,
        GrantRequest {
            root: g.root,
            id: Some(g.id),
            expected_version: Some(g.version),
            subject: g.subject,
            capabilities: g.capabilities,
            expires_at: None,
            active: false,
            request_id: "shortcut-target-revoke".into(),
        },
        17,
    )
    .unwrap();
    assert!(s.drive_shortcut_target(&b, link.object.id, 18).is_err());
    assert!(s.drive_get(&b, link.object.id, 18).is_ok());
    s.drive_organize(
        &b,
        OrganizeRequest { object: link.object.id, expected_version: 1, change: OrganizationChange::Trash, request_id: "shortcut-trash-001".into() },
        19,
    )
    .unwrap();
    assert_eq!(s.drive_get(&a, f.object.id, 20).unwrap().latest.unwrap().bytes, 3);
}
#[test]
fn folder_colors_are_private_survive_summary_cleanup_and_reopen() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(50);
    let b = actor(51);
    let root = s.drive_initialize(a.sponsor, 10).unwrap();
    let folder =
        s.drive_create_folder(&a, CreateFolder { parent: root.object.id, name: "Folder".into(), request_id: "color-folder-create".into() }, 11).unwrap();
    grant(&mut s, &a, folder.object.id, &b);
    s.drive_folder_color(&a, folder.object.id, Some("gold".into()), 12).unwrap();
    s.drive_star(&a, folder.object.id, true, 13).unwrap();
    s.drive_star(&a, folder.object.id, false, 14).unwrap();
    let query = BrowseQuery {
        scope: BrowseScope::Item(folder.object.id),
        text: String::new(),
        file_type: String::new(),
        owner: None,
        modified_after: None,
        modified_before: None,
        sort: SortField::Name,
        descending: false,
    };
    assert_eq!(s.drive_query(&a, query.clone(), None, 50, 15).unwrap().items[0].color.as_deref(), Some("gold"));
    assert_eq!(s.drive_query(&b, query.clone(), None, 50, 15).unwrap().items[0].color, None);
    drop(s);
    let mut s = Store::new(memory);
    assert_eq!(s.drive_query(&a, query.clone(), None, 50, 16).unwrap().items[0].color.as_deref(), Some("gold"));
    s.drive_folder_color(&a, folder.object.id, None, 17).unwrap();
    assert_eq!(s.drive_query(&a, query, None, 50, 18).unwrap().items[0].color, None);
}

#[test]
fn requests_preserve_privacy_and_owner_decisions_are_idempotent() {
    use sovereign_cloud::people::*;
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(61);
    let b = actor(62);
    let stranger = actor(63);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    s.drive_initialize(stranger.sponsor, 1).unwrap();
    let f = file(&mut s, &a, root, "Private request.txt");
    assert!(s.drive_get(&b, f.object.id, 12).is_err());
    assert_eq!(s.drive_request_access(&b, crate::test_id(999999), "Hello".into(), 12), s.drive_request_access(&b, f.object.id, "Let me read".into(), 12));
    s.drive_request_access(&b, f.object.id, "Duplicate".into(), 13).unwrap();
    let inbox = s.drive_inbox(&a, 14).unwrap();
    assert_eq!(inbox.items.len(), 1);
    assert_eq!(inbox.items[0].message.as_deref(), Some("Let me read"));
    assert!(s.drive_inbox(&stranger, 14).unwrap().items.is_empty());
    let decision = AccessDecision {
        object: f.object.id,
        requester: b.sponsor,
        requested_at: 12,
        approve: true,
        editor: false,
        expires_at: Some(100),
        request_id: "request-approval-once".into(),
    };
    assert!(s.drive_decide_access(&stranger, decision.clone(), 15).is_err());
    s.drive_decide_access(&a, decision.clone(), 15).unwrap();
    assert!(s.drive_get(&b, f.object.id, 16).unwrap().capabilities.read);
    assert!(!s.drive_get(&b, f.object.id, 16).unwrap().capabilities.edit);
    assert!(!s.drive_inbox(&a, 16).unwrap().items[0].can_respond);
    assert_eq!(s.drive_inbox(&b, 16).unwrap().items.len(), 1);
    drop(s);
    let mut s = Store::new(memory);
    s.drive_decide_access(&a, decision, 17).unwrap();
    assert_eq!(s.drive_grants(&a, f.object.id).unwrap().len(), 1);
    assert_eq!(s.drive_inbox(&b, 17).unwrap().items.len(), 1);
    assert!(s.drive_get(&b, f.object.id, 100).is_err());
    assert!(s.drive_inbox(&b, 100).unwrap().items.is_empty());
}
#[test]
fn approving_editor_access_matches_the_sharing_dialog_preset() {
    use sovereign_cloud::people::AccessDecision;
    let mut s = Store::new(VectorMemory::default());
    let a = actor(83);
    let b = actor(84);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    let f = file(&mut s, &a, root, "Editor request.txt");
    s.drive_request_access(&b, f.object.id, String::new(), 12).unwrap();
    s.drive_decide_access(
        &a,
        AccessDecision {
            object: f.object.id,
            requester: b.sponsor,
            requested_at: 12,
            approve: true,
            editor: true,
            expires_at: None,
            request_id: "editor-request-approval".into(),
        },
        13,
    )
    .unwrap();
    assert_eq!(s.drive_get(&b, f.object.id, 14).unwrap().capabilities, Capabilities { read: true, edit: true, delete: true, create: false, propose: false });
}
#[test]
fn contacts_and_notifications_are_private_and_requests_are_rate_limited() {
    use sovereign_cloud::people::*;
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(71);
    let b = actor(72);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    s.drive_initialize(b.sponsor, 1).unwrap();
    s.drive_contact(&a, b.sponsor, "My editor".into()).unwrap();
    assert!(s.drive_contacts(&b).unwrap().is_empty());
    s.drive_notifications(&b, false).unwrap();
    let f = file(&mut s, &a, root, "Notice.txt");
    grant(&mut s, &a, f.object.id, &b);
    assert!(s.drive_inbox(&b, 20).unwrap().items.is_empty());
    s.drive_notifications(&b, true).unwrap();
    s.drive_notify(&a, f.object.id, b.sponsor, 21).unwrap();
    s.drive_notify(&a, f.object.id, b.sponsor, 22).unwrap();
    let inbox = s.drive_inbox(&b, 23).unwrap();
    assert_eq!(inbox.items.len(), 1);
    assert_eq!(inbox.items[0].notice.kind, NoticeKind::Reminder);
    s.drive_read_notice(&a, inbox.items[0].notice.id).unwrap();
    assert!(!s.drive_inbox(&b, 24).unwrap().items[0].notice.read);
    s.drive_read_notice(&b, inbox.items[0].notice.id).unwrap();
    assert!(s.drive_inbox(&b, 24).unwrap().items[0].notice.read);
    for n in 0..10 {
        s.drive_request_access(&b, crate::test_id(900 + n), String::new(), 25).unwrap();
    }
    assert!(s.drive_request_access(&b, crate::test_id(1000), String::new(), 25).is_err());
    drop(s);
    let s = Store::new(memory);
    assert_eq!(s.drive_contacts(&a).unwrap()[0].label, "My editor");
    assert!(s.drive_inbox(&b, 26).unwrap().items[0].notice.read);
}

#[test]
fn block_is_bidirectional_live_authority_and_hiding_is_only_a_personal_view() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(81);
    let b = actor(82);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let other = s.drive_initialize(b.sponsor, 1).unwrap().object.id;
    let fa = file(&mut s, &a, root, "Shared A.txt");
    let fb = file(&mut s, &b, other, "Shared B.txt");
    grant(&mut s, &a, fa.object.id, &b);
    grant(&mut s, &b, fb.object.id, &a);
    s.drive_hide_shared(&b, fa.object.id, true, 20).unwrap();
    assert!(s.drive_get(&b, fa.object.id, 21).is_ok());
    assert_eq!(s.drive_sharing_safety(&b, 21).unwrap().hidden.len(), 1);
    assert!(s.drive_sharing_safety(&a, 21).unwrap().hidden.is_empty());
    s.drive_report(&b, fa.object.id, "Unwanted share".into(), 21).unwrap();
    assert_eq!(s.sharing_reports(None).unwrap().len(), 1);
    s.drive_block(&b, a.sponsor, true).unwrap();
    assert!(s.drive_get(&b, fa.object.id, 22).is_err());
    assert!(s.drive_history(&b, fa.object.id, 22).is_err());
    assert!(s.drive_get(&a, fb.object.id, 22).is_err());
    assert!(s.drive_inbox(&b, 22).unwrap().items.is_empty());
    assert!(s.drive_inbox(&a, 22).unwrap().items.is_empty());
    assert!(s.drive_notify(&a, fa.object.id, b.sponsor, 22).is_err());
    assert!(
        s.drive_grant(
            &a,
            GrantRequest {
                root: fa.object.id,
                id: None,
                expected_version: None,
                subject: b.subject.clone(),
                capabilities: Capabilities::READ,
                active: true,
                expires_at: None,
                request_id: "blocked-new-grant-01".into()
            },
            22
        )
        .is_err()
    );
    s.drive_request_access(&a, fb.object.id, "Hello".into(), 23).unwrap();
    assert!(s.drive_inbox(&b, 23).unwrap().items.is_empty());
    assert!(s.drive_get(&a, fa.object.id, 23).is_ok());
    drop(s);
    let mut s = Store::new(memory);
    assert!(s.drive_get(&a, fb.object.id, 24).is_err());
    s.drive_block(&b, a.sponsor, false).unwrap();
    assert!(s.drive_get(&a, fb.object.id, 25).is_ok());
    assert!(s.drive_get(&b, fa.object.id, 25).is_ok());
    s.drive_hide_shared(&b, fa.object.id, false, 26).unwrap();
    assert!(s.drive_sharing_safety(&b, 26).unwrap().hidden.is_empty());
}

#[test]
fn automatic_retention_keeps_twenty_checkpoints_and_all_manual_named_versions() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    let a = actor(91);
    let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
    let mut f = file(&mut s, &a, root, "Autosave.txt");
    let manual = f.latest.as_ref().unwrap().id;
    let mut pinned = 0;
    for n in 0..26 {
        let bytes = vec![n];
        let request = DriveUploadRequest {
            automatic: Some(true),
            target: UploadTarget::Replace { file: f.object.id, expected_revision: f.latest.as_ref().unwrap().id },
            bytes: 1,
            sha256: Sha256::digest(&bytes).to_vec(),
            request_id: format!("autosave-policy-{n:04}"),
        };
        let stage = s.drive_begin(&a, request, 30 + n as u64).unwrap();
        s.drive_put(&a, stage.id, 0, bytes, 30 + n as u64).unwrap();
        let saved = s.drive_commit(&a, stage.id, 30 + n as u64).unwrap();
        if n == 0 {
            pinned = saved.committed.unwrap().id;
            s.drive_label_revision(
                &a,
                RevisionLabelRequest {
                    object: f.object.id,
                    revision: pinned,
                    expected_version: 0,
                    label: "Named automatic".into(),
                    keep: true,
                    request_id: "autosave-retained-name".into(),
                },
                31,
            )
            .unwrap();
        }
        f = s.drive_get(&a, f.object.id, 60).unwrap();
        assert_eq!(s.drive_commit(&a, stage.id, 60).unwrap().id, stage.id);
    }
    let versions = s.drive_versions(&a, f.object.id, 70).unwrap();
    assert_eq!(versions.len(), 22);
    assert_eq!(versions.iter().filter(|v| v.properties.automatic && !v.properties.keep).count(), 20);
    assert!(versions.iter().any(|v| v.revision.id == manual && !v.properties.automatic));
    assert!(versions.iter().any(|v| v.revision.id == pinned && v.properties.keep));
    assert_eq!(s.drive_storage(&a).unwrap().used_bytes, 24);
    drop(s);
    let s = Store::new(memory);
    assert_eq!(s.drive_versions(&a, f.object.id, 71).unwrap().len(), 22);
    assert_eq!(s.drive_storage(&a).unwrap().used_bytes, 24);
}

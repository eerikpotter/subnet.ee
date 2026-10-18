//! Populated UUID schema: activity, revisions and authority survive code upgrades.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use sovereign_cloud::drive::*;
use sovereign_cloud::people::*;
fn call<T: CandidType + DeserializeOwned>(p: &PocketIc, id: Principal, caller: Principal, method: &str, args: impl ArgumentEncoder) -> T {
    decode_one(&p.update_call(id, caller, method, encode_args(args).unwrap()).unwrap()).unwrap()
}
#[test]
fn populated_upgrade_keeps_manual_versions_and_preserves_new_activity_labels_and_deletion_receipts() {
    let current = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").unwrap()).unwrap();
    let p = PocketIcBuilder::new().with_application_subnet().build();
    let id = p.create_canister();
    p.add_cycles(id, 100_000_000_000_000);
    p.install_canister(id, current.clone(), encode_args(()).unwrap(), None);
    let a = Principal::self_authenticating([171; 32]);
    let b = Principal::self_authenticating([172; 32]);
    let root: DriveResult<DriveEntry> = call(&p, id, a, "drive_initialize", ());
    let root = root.unwrap();
    let b_root: DriveResult<DriveEntry> = call(&p, id, b, "drive_initialize", ());
    let b_root = b_root.unwrap();
    let f: DriveResult<DriveEntry> = call(
        &p,
        id,
        a,
        "drive_create_file",
        (InlineFile {
            parent: root.object.id,
            name: "Versioned.txt".into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: b"original".to_vec(),
            request_id: "workflow-upgrade-create".into(),
        },),
    );
    let f = f.unwrap();
    let old = f.latest.as_ref().unwrap().id;
    let u: DriveResult<DriveUpload> = call(
        &p,
        id,
        a,
        "drive_begin_upload",
        (DriveUploadRequest {
            automatic: None,
            target: UploadTarget::Replace { file: f.object.id, expected_revision: old },
            bytes: 7,
            sha256: Sha256::digest(b"updated").to_vec(),
            request_id: "workflow-upgrade-replace".into(),
        },),
    );
    let u = u.unwrap();
    let put: DriveResult<()> = call(&p, id, a, "drive_put_chunk", (u.id, 0u64, b"updated".to_vec()));
    put.unwrap();
    let commit: DriveResult<DriveUpload> = call(&p, id, a, "drive_commit_upload", (u.id,));
    commit.unwrap();
    let grant: DriveResult<AccessGrant> = call(
        &p,
        id,
        a,
        "drive_set_grant",
        (GrantRequest {
            root: f.object.id,
            id: None,
            expected_version: None,
            subject: Participant::User(b),
            capabilities: Capabilities { read: true, edit: true, ..Capabilities::NONE },
            expires_at: None,
            active: true,
            request_id: "workflow-upgrade-share".into(),
        },),
    );
    grant.unwrap();
    let owner_history: DriveResult<ActivityPage> = call(&p, id, a, "drive_activity", (Some(f.object.id), None::<u64>, 50u32));
    let owner_history = owner_history.unwrap();
    assert!(!owner_history.items.is_empty());
    let reader_history: DriveResult<ActivityPage> = call(&p, id, b, "drive_activity", (Some(f.object.id), None::<u64>, 50u32));
    let reader_history = reader_history.unwrap();
    p.upgrade_canister(id, current.clone(), encode_args(()).unwrap(), None).unwrap();
    let versions: DriveResult<Vec<RevisionInfo>> = call(&p, id, a, "drive_versions", (f.object.id,));
    let versions = versions.unwrap();
    assert_eq!(versions.len(), 2);
    assert!(versions.iter().all(|v| !v.properties.automatic));
    let history: DriveResult<ActivityPage> = call(&p, id, a, "drive_activity", (Some(f.object.id), None::<u64>, 50u32));
    assert_eq!(serde_json::to_value(history.unwrap()).unwrap(), serde_json::to_value(&owner_history).unwrap());
    let label = RevisionLabelRequest {
        object: f.object.id,
        revision: old,
        expected_version: 0,
        label: "Release".into(),
        keep: false,
        request_id: "workflow-upgrade-label".into(),
    };
    let denied: DriveResult<RevisionProperties> = call(&p, id, b, "drive_label_revision", (label.clone(),));
    assert!(denied.is_err());
    let kept: DriveResult<RevisionProperties> = call(&p, id, a, "drive_label_revision", (label.clone(),));
    let kept = kept.unwrap();
    assert!(kept.keep);
    p.upgrade_canister(id, current.clone(), encode_args(()).unwrap(), None).unwrap();
    let retry: DriveResult<RevisionProperties> = call(&p, id, a, "drive_label_revision", (label,));
    assert_eq!(retry.unwrap(), kept);
    let history: DriveResult<ActivityPage> = call(&p, id, a, "drive_activity", (Some(f.object.id), None::<u64>, 50u32));
    assert_eq!(history.unwrap().items.len(), owner_history.items.len() + 1);
    let history: DriveResult<ActivityPage> = call(&p, id, b, "drive_activity", (Some(f.object.id), None::<u64>, 50u32));
    assert_eq!(serde_json::to_value(history.unwrap()).unwrap(), serde_json::to_value(reader_history).unwrap());
    let unpin: DriveResult<RevisionProperties> = call(
        &p,
        id,
        a,
        "drive_label_revision",
        (RevisionLabelRequest {
            object: f.object.id,
            revision: old,
            expected_version: kept.version,
            label: "".into(),
            keep: false,
            request_id: "workflow-upgrade-unpin".into(),
        },),
    );
    let unpin = unpin.unwrap();
    let head: DriveResult<DriveEntry> = call(&p, id, a, "drive_get", (f.object.id,));
    let head = head.unwrap();
    let delete = RevisionDeleteRequest {
        object: f.object.id,
        revision: old,
        expected_object_version: head.object.version,
        expected_properties_version: unpin.version,
        request_id: "workflow-upgrade-delete".into(),
    };
    let receipt: DriveResult<MutationReceipt> = call(&p, id, a, "drive_delete_revision", (delete.clone(),));
    let receipt = receipt.unwrap();
    let shortcut: DriveResult<DriveEntry> = call(
        &p,
        id,
        b,
        "drive_create_shortcut",
        (CreateShortcut { parent: b_root.object.id, target: f.object.id, name: "Shortcut".into(), request_id: "workflow-shortcut-request".into() },),
    );
    let shortcut = shortcut.unwrap();
    let color: DriveResult<()> = call(&p, id, b, "drive_folder_color", (b_root.object.id, Some("gold".to_string())));
    color.unwrap();
    let contact: DriveResult<()> = call(&p, id, b, "drive_contact", (a, "My collaborator".to_string()));
    contact.unwrap();
    let hidden: DriveResult<()> = call(&p, id, b, "drive_hide_shared", (f.object.id, true));
    hidden.unwrap();
    let request: DriveResult<()> = call(&p, id, b, "drive_request_access", (root.object.id, "Please share this folder".to_string()));
    request.unwrap();
    let inbox: DriveResult<Inbox> = call(&p, id, a, "drive_inbox", ());
    let request = inbox.unwrap().items.into_iter().find(|i| i.can_respond).unwrap();
    p.upgrade_canister(id, current, encode_args(()).unwrap(), None).unwrap();
    let retry: DriveResult<MutationReceipt> = call(&p, id, a, "drive_delete_revision", (delete,));
    assert_eq!(retry.unwrap(), receipt);
    let usage: DriveResult<StorageOverview> = call(&p, id, a, "drive_storage", ());
    let usage = usage.unwrap();
    assert_eq!(usage.used_bytes, 7);
    assert_eq!(usage.history_bytes, 0);
    let bytes: DriveResult<Vec<u8>> = call(&p, id, a, "drive_current_chunk", (f.object.id, u.id, 0u64));
    assert_eq!(bytes.unwrap(), b"updated");
    let contacts: DriveResult<Vec<Contact>> = call(&p, id, b, "drive_contacts", ());
    assert_eq!(contacts.unwrap(), vec![Contact { principal: a, label: "My collaborator".into() }]);
    let owner_contacts: DriveResult<Vec<Contact>> = call(&p, id, a, "drive_contacts", ());
    assert!(owner_contacts.unwrap().is_empty());
    let safety: DriveResult<SharingSafety> = call(&p, id, b, "drive_sharing_safety", ());
    assert_eq!(safety.unwrap().hidden[0].object, f.object.id);
    let target: DriveResult<DriveEntry> = call(&p, id, b, "drive_shortcut_target", (shortcut.object.id,));
    assert_eq!(target.unwrap().object.id, f.object.id);
    let decision = AccessDecision {
        object: root.object.id,
        requester: b,
        requested_at: request.notice.at,
        approve: true,
        editor: false,
        expires_at: None,
        request_id: "workflow-access-approval".into(),
    };
    let allowed: DriveResult<()> = call(&p, id, a, "drive_decide_access", (decision.clone(),));
    allowed.unwrap();
    let retry: DriveResult<()> = call(&p, id, a, "drive_decide_access", (decision,));
    retry.unwrap();
    let inbox: DriveResult<Inbox> = call(&p, id, a, "drive_inbox", ());
    assert!(inbox.unwrap().items.iter().all(|i| !i.can_respond));
    let read: DriveResult<DriveEntry> = call(&p, id, b, "drive_get", (root.object.id,));
    assert!(read.unwrap().capabilities.read);
    let blocked: DriveResult<()> = call(&p, id, b, "drive_block", (a, true));
    blocked.unwrap();
    let target: DriveResult<DriveEntry> = call(&p, id, b, "drive_shortcut_target", (shortcut.object.id,));
    assert_eq!(target.unwrap_err(), DriveError::Inaccessible);
    let usage: DriveResult<StorageOverview> = call(&p, id, b, "drive_storage", ());
    assert_eq!(usage.unwrap().used_bytes, 0);
}

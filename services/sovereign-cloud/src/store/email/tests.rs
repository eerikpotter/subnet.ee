#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use ic_stable_structures::VectorMemory;

fn user(n: u8) -> Principal {
    Principal::self_authenticating([n; 32])
}
fn setup() -> Store<VectorMemory> {
    let mut s = Store::new(VectorMemory::default());
    s.email_configure(Some(MailConfig {
        api_key: format!("re_{}", "test".repeat(8)),
        from: "subnet.ee <info@subnet.ee>".into(),
        origin: "https://www.subnet.ee".into(),
    }))
    .unwrap();
    for n in 1..=3 {
        s.drive_initialize(user(n), 1).unwrap();
    }
    s
}
fn request(s: &mut Store<VectorMemory>, caller: Principal, email: &str, now: u64) -> u64 {
    let (id, prepare) = s.email_prepare(caller, EmailRequest { email: email.into(), request_id: format!("verification-request-{now}") }, now).unwrap();
    assert!(prepare);
    s.email_issue(caller, id, &12_345_678u64.to_be_bytes(), now).unwrap();
    id
}
fn bind(s: &mut Store<VectorMemory>, caller: Principal, email: &str, now: u64) {
    let id = request(s, caller, email, now);
    assert_eq!(s.email_verify(caller, id, "12345678".into(), now).unwrap(), EmailVerified::Added);
}
fn folder(s: &mut Store<VectorMemory>) -> ObjectId {
    let root = s.drive_initialize(user(1), 1).unwrap().object.id;
    s.drive_create_folder(&Actor::user(user(1)).unwrap(), CreateFolder { parent: root, name: "Private".into(), request_id: "email-test-folder".into() }, 2)
        .unwrap()
        .object
        .id
}
fn share(s: &mut Store<VectorMemory>, object: ObjectId, email: &str, now: u64) -> EmailInvitation {
    s.email_share(
        user(1),
        EmailShare { grant_expires_at: None, object, email: email.into(), role: EmailRole::Viewer, request_id: format!("share-email-request-{now}") },
        now,
    )
    .unwrap()
}

#[test]
fn multiple_emails_unique_binding_explicit_transfer_and_no_acl_transfer() {
    let mut s = setup();
    bind(&mut s, user(1), "owner@example.com", 2);
    bind(&mut s, user(2), "first@example.com", 3);
    bind(&mut s, user(2), "second@example.com", 2 * MINUTE);
    let object = folder(&mut s);
    share(&mut s, object, "first@example.com", 3 * MINUTE);
    let id = request(&mut s, user(3), " FIRST@example.com ", 4 * MINUTE);
    assert_eq!(s.email_verify(user(3), id, "12345678".into(), 4 * MINUTE).unwrap(), EmailVerified::TransferRequired);
    assert_eq!(s.email.address("first@example.com").unwrap().owner, Some(user(2)));
    assert!(s.email_confirm_transfer(user(2), id, 4 * MINUTE).is_err());
    s.email_confirm_transfer(user(3), id, 4 * MINUTE).unwrap();
    assert_eq!(s.email.emails(user(2)).unwrap(), vec!["second@example.com"]);
    assert_eq!(s.email.emails(user(3)).unwrap(), vec!["first@example.com"]);
    assert!(s.drive_get(&Actor::user(user(2)).unwrap(), object, 4 * MINUTE).is_ok());
    assert!(s.drive_get(&Actor::user(user(3)).unwrap(), object, 4 * MINUTE).is_err());
    share(&mut s, object, "first@example.com", 5 * MINUTE);
    assert!(s.drive_get(&Actor::user(user(3)).unwrap(), object, 5 * MINUTE).is_ok());
    s.email_remove(user(3), "first@example.com".into()).unwrap();
    assert!(s.drive_get(&Actor::user(user(3)).unwrap(), object, 5 * MINUTE).is_ok());
}
#[test]
fn proof_is_caller_bound_single_use_expiring_and_guess_limited() {
    let mut s = setup();
    let id = request(&mut s, user(1), "alice@example.com", 1);
    assert!(s.email_verify(user(2), id, "12345678".into(), 2).is_err());
    for _ in 0..5 {
        assert!(s.email_verify(user(1), id, "wrong".into(), 2).is_err());
    }
    assert!(s.email_verify(user(1), id, "12345678".into(), 3).is_err());
    let fresh = request(&mut s, user(1), "alice@example.com", 2 * MINUTE);
    assert!(s.email_verify(user(1), fresh, "12345678".into(), 13 * MINUTE).is_err());
    let fresh = request(&mut s, user(1), "alice@example.com", 14 * MINUTE);
    s.email_verify(user(1), fresh, "12345678".into(), 14 * MINUTE).unwrap();
    assert!(s.email_verify(user(1), fresh, "12345678".into(), 14 * MINUTE).is_err());
    assert!(s.email_profile(Principal::anonymous(), 1).is_err());
}
#[test]
fn transfer_does_not_cross_a_binding_change_after_proof() {
    let mut s = setup();
    bind(&mut s, user(1), "one@example.com", 1);
    let id = request(&mut s, user(2), "one@example.com", 2);
    assert_eq!(s.email_verify(user(2), id, "12345678".into(), 3).unwrap(), EmailVerified::TransferRequired);
    s.email_remove(user(1), "one@example.com".into()).unwrap();
    assert!(s.email_confirm_transfer(user(2), id, 4).is_err());
}
#[test]
fn pending_invitation_requires_correct_mailbox_and_current_authority() {
    let mut s = setup();
    bind(&mut s, user(1), "owner@example.com", 1);
    let object = folder(&mut s);
    let i = share(&mut s, object, "new@example.com", 2);
    assert!(!i.accepted);
    assert!(s.email_accept(user(2), i.id, 3).is_err());
    bind(&mut s, user(2), "other@example.com", 4);
    assert!(s.email_accept(user(2), i.id, 5).is_err());
    bind(&mut s, user(2), "new@example.com", 2 * MINUTE);
    assert_eq!(s.email_accept(user(2), i.id, 2 * MINUTE).unwrap(), object);
    let grant_count = s.read_grants.len();
    assert_eq!(s.email_accept(user(2), i.id, 2 * MINUTE).unwrap(), object);
    assert_eq!(s.read_grants.len(), grant_count);
    assert!(s.email_accept(user(3), i.id, 2 * MINUTE).is_err());
    let revoked = share(&mut s, object, "revoked@example.com", 3 * MINUTE);
    s.email_revoke_invitation(user(1), revoked.id, 3 * MINUTE).unwrap();
    bind(&mut s, user(3), "revoked@example.com", 4 * MINUTE);
    assert!(s.email_accept(user(3), revoked.id, 4 * MINUTE).is_err());
}
#[test]
fn pending_invitation_does_not_follow_rebound_email() {
    let mut s = setup();
    bind(&mut s, user(1), "owner@example.com", 1);
    let object = folder(&mut s);
    let i = share(&mut s, object, "new@example.com", 2);
    bind(&mut s, user(2), "new@example.com", 3);
    let id = request(&mut s, user(3), "new@example.com", 4);
    s.email_verify(user(3), id, "12345678".into(), 5).unwrap();
    s.email_confirm_transfer(user(3), id, 6).unwrap();
    assert!(s.email_accept(user(3), i.id, 7).is_err());
    assert!(s.email_accept(user(2), i.id, 7).is_err());
}
#[test]
fn retry_keeps_exact_payload_and_key_and_blocks_duplicate_parallel_send() {
    let mut s = setup();
    let id = request(&mut s, user(1), "one@example.com", 1);
    let DeliveryClaim { attempt, payload: bytes, idempotency: key, .. } = s.email_claim_delivery(user(1), id, 2).unwrap().unwrap();
    assert!(s.email_claim_delivery(user(2), id, 2).is_err());
    assert!(s.email_claim_delivery(user(1), id, 3).is_err());
    s.email_finish_delivery(id, attempt, DeliveryStatus::Unknown).unwrap();
    let DeliveryClaim { attempt: next, payload: retry, idempotency: retry_key, .. } = s.email_claim_delivery(user(1), id, 3 * MINUTE).unwrap().unwrap();
    assert_eq!(bytes, retry);
    assert_eq!(key, retry_key);
    s.email_finish_delivery(id, attempt, DeliveryStatus::Accepted).unwrap();
    assert_eq!(s.email.status(id, 3 * MINUTE).unwrap(), DeliveryStatus::Sending);
    s.email_finish_delivery(id, next, DeliveryStatus::Accepted).unwrap();
    assert!(s.email.job(id).unwrap().payload.is_none());
    assert!(s.email_claim_delivery(user(1), id, 4 * MINUTE).unwrap().is_none());
}
#[test]
fn stable_reopen_keeps_contacts_challenges_invitations_and_uncertain_delivery() {
    let memory = VectorMemory::default();
    let mut s = Store::new(memory.clone());
    s.email_configure(Some(MailConfig {
        api_key: format!("re_{}", "test".repeat(8)),
        from: "subnet.ee <info@subnet.ee>".into(),
        origin: "https://www.subnet.ee".into(),
    }))
    .unwrap();
    s.drive_initialize(user(1), 1).unwrap();
    bind(&mut s, user(1), "owner@example.com", 2);
    let object = folder(&mut s);
    let invite = share(&mut s, object, "new@example.com", 3);
    let DeliveryClaim { payload: bytes, idempotency: key, .. } = s.email_claim_delivery(user(1), invite.id, 4).unwrap().unwrap();
    let mut s = Store::new(memory);
    s.email_configure(Some(MailConfig {
        api_key: format!("re_{}", "test".repeat(8)),
        from: "subnet.ee <info@subnet.ee>".into(),
        origin: "https://www.subnet.ee".into(),
    }))
    .unwrap();
    assert_eq!(s.email.emails(user(1)).unwrap(), vec!["owner@example.com"]);
    let DeliveryClaim { payload: retry, idempotency: retry_key, .. } = s.email_claim_delivery(user(1), invite.id, 3 * MINUTE).unwrap().unwrap();
    assert_eq!(bytes, retry);
    assert_eq!(key, retry_key);
    assert_eq!(s.email_invitations(user(1), object, 3 * MINUTE).unwrap().len(), 1);
}
#[test]
fn normalization_preserves_aliases_and_rejects_injection_and_limits_resends() {
    assert_eq!(normalize_email(" Person+Tag@Example.COM ").unwrap(), "person+tag@example.com");
    for email in ["a@b", "a\r\nBcc:x@example.com", "a@@example.com", "..a@example.com", "a@-bad.com", "x <a@example.com>"] {
        assert!(normalize_email(email).is_err(), "{email}");
    }
    let mut s = setup();
    let id = request(&mut s, user(1), "a@example.com", 1);
    let repeat = s.email_prepare(user(1), EmailRequest { email: "a@example.com".into(), request_id: "verification-request-1".into() }, 2).unwrap();
    assert_eq!(repeat, (id, false));
    assert!(s.email_prepare(user(1), EmailRequest { email: "b@example.com".into(), request_id: "different-request-id".into() }, 2).is_err());
}

#[test]
fn repeated_sharing_updates_direct_role_and_old_acceptance_cannot_restore_revoked_access() {
    let mut s = setup();
    bind(&mut s, user(1), "owner@example.com", 1);
    bind(&mut s, user(2), "reader@example.com", 2);
    let object = folder(&mut s);
    let mut req =
        EmailShare { grant_expires_at: None, object, email: "reader@example.com".into(), role: EmailRole::Editor, request_id: "share-email-editor-001".into() };
    let invitation = s.email_share(user(1), req.clone(), 3).unwrap();
    req.role = EmailRole::Viewer;
    req.request_id = "share-email-viewer-002".into();
    s.email_share(user(1), req, 4).unwrap();
    let actor = Actor::user(user(1)).unwrap();
    let grants = s.drive_grants(&actor, object).unwrap();
    assert_eq!(grants.len(), 1);
    let grant = &grants[0];
    assert!(!grant.capabilities.edit);
    s.drive_grant(
        &actor,
        GrantRequest {
            id: Some(grant.id),
            expected_version: Some(grant.version),
            root: object,
            subject: grant.subject.clone(),
            capabilities: grant.capabilities,
            active: false,
            expires_at: None,
            request_id: "revoke-email-grant-001".into(),
        },
        5,
    )
    .unwrap();
    assert!(s.email_accept(user(2), invitation.id, 6).is_err());
    assert!(!s.drive_grants(&actor, object).unwrap()[0].active);
}
#[test]
fn invitations_expire_and_cannot_grant_access_to_trashed_targets() {
    let mut s = setup();
    bind(&mut s, user(1), "owner@example.com", 1);
    let object = folder(&mut s);
    let invitation = share(&mut s, object, "reader@example.com", 2);
    bind(&mut s, user(2), "reader@example.com", 3);
    assert!(s.email_accept(user(2), invitation.id, 8 * DAY).is_err());
    let actor = Actor::user(user(1)).unwrap();
    let entry = s.drive_get(&actor, object, 4).unwrap();
    s.drive_organize(
        &actor,
        OrganizeRequest { object, expected_version: entry.object.version, change: OrganizationChange::Trash, request_id: "trash-email-target-001".into() },
        5,
    )
    .unwrap();
    assert!(s.email_accept(user(2), invitation.id, 6).is_err());
    assert!(s.read_grants.is_empty());
}
#[test]
fn cleanup_erases_expired_payloads_and_receipts_without_removing_bindings_or_grants() {
    let mut s = setup();
    bind(&mut s, user(1), "owner@example.com", 1);
    bind(&mut s, user(2), "reader@example.com", 2);
    let object = folder(&mut s);
    let invitation = share(&mut s, object, "reader@example.com", 3);
    s.email_claim_delivery(user(1), invitation.id, 4).unwrap(); // Simulate an interrupted await.
    assert!(s.email_claim_delivery(user(1), invitation.id, DAY + 4).is_err());
    s.email.prune(DAY + 4).unwrap();
    assert!(s.email.job(invitation.id).unwrap().payload.is_none());
    for _ in 0..2 {
        s.email.prune(9 * DAY).unwrap();
    }
    assert!(s.email.jobs.is_empty());
    assert!(s.email.invitations.is_empty());
    assert!(s.email.owners.is_empty());
    assert!(s.email.rates.is_empty());
    assert_eq!(s.email.emails(user(2)).unwrap(), vec!["reader@example.com"]);
    assert!(s.drive_get(&Actor::user(user(2)).unwrap(), object, 9 * DAY).is_ok());
}

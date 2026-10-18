//! Real upgrade, management randomness and HTTPS adapter checks; provider replies are mocked.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use pocket_ic::common::rest::{CanisterHttpReply, CanisterHttpResponse, MockCanisterHttpResponse};
use pocket_ic::{CanisterSettings, EnvironmentVariable, PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use sovereign_cloud::{CloudResult, drive::*, email::*};
fn call<T: CandidType + DeserializeOwned>(pic: &PocketIc, id: Principal, caller: Principal, method: &str, args: impl ArgumentEncoder) -> T {
    decode_one(&pic.update_call(id, caller, method, encode_args(args).unwrap()).unwrap()).unwrap()
}
fn verify_mail(pic: &PocketIc, id: Principal, caller: Principal, email: &str, request: &str) {
    let (pending, code) = request_mail(pic, id, caller, email, request);
    let result: CloudResult<EmailVerified> = call(pic, id, caller, "email_verify", (pending.id, code));
    assert_eq!(result.unwrap(), EmailVerified::Added);
}
fn request_mail(pic: &PocketIc, id: Principal, caller: Principal, email: &str, request: &str) -> (EmailVerification, String) {
    let message =
        pic.submit_call(id, caller, "email_request", encode_args((EmailRequest { email: email.into(), request_id: request.into() },)).unwrap()).unwrap();
    let payload = deliver(pic, email);
    let text = payload["text"].as_str().unwrap();
    assert!(text.contains("https://www.subnet.ee/pilveruum/#email"));
    let code = text.split_whitespace().find(|s| s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit())).unwrap().to_string();
    let profile = decode_one::<CloudResult<EmailProfile>>(&pic.await_call(message).unwrap()).unwrap().unwrap();
    assert_eq!(profile.pending.as_ref().unwrap().delivery, DeliveryStatus::Accepted);
    (profile.pending.unwrap(), code)
}
// Only the controlled provider harness sees test codes; application APIs never return them.
fn deliver(pic: &PocketIc, email: &str) -> serde_json::Value {
    let mut http = None;
    for _ in 0..30 {
        pic.tick();
        if let Some(request) = pic.get_canister_http().into_iter().next() {
            http = Some(request);
            break;
        }
    }
    let http = http.unwrap_or_else(|| panic!("Expected management HTTP request for test recipient {email}"));
    assert_eq!(http.url, "https://api.resend.com/emails");
    assert!(format!("{:?}", http.replication).contains("NonReplicated"));
    let payload: serde_json::Value = serde_json::from_slice(&http.body).unwrap();
    assert_eq!(payload["to"][0], email);
    assert_eq!(payload["from"], "subnet.ee <info@subnet.ee>");
    pic.mock_canister_http_response(MockCanisterHttpResponse {
        subnet_id: http.subnet_id,
        request_id: http.request_id,
        response: CanisterHttpResponse::CanisterHttpReply(CanisterHttpReply { status: 200, headers: vec![], body: br#"{"id":"test-message"}"#.to_vec() }),
        additional_responses: vec![],
    });
    payload
}
#[test]
fn upgrade_preserves_files_and_new_email_state_with_actual_outcall_contract() {
    let after = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set email Wasm")).unwrap();
    // Match the ordinary local launcher: no experimental feature overrides.
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let id = pic.create_canister();
    pic.add_cycles(id, 100_000_000_000_000);
    pic.install_canister(id, after.clone(), encode_args(()).unwrap(), None);
    let a = Principal::self_authenticating([61; 32]);
    let b = Principal::self_authenticating([62; 32]);
    let root: DriveResult<DriveEntry> = call(&pic, id, a, "drive_initialize", ());
    let root = root.unwrap();
    let file: DriveResult<DriveEntry> = call(
        &pic,
        id,
        a,
        "drive_create_file",
        (InlineFile {
            parent: root.object.id,
            name: "Before email.txt".into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: b"original file".to_vec(),
            request_id: "email-upgrade-file".into(),
        },),
    );
    let file = file.unwrap();
    pic.upgrade_canister(id, after.clone(), encode_args(()).unwrap(), None).unwrap();
    pic.update_canister_settings(
        id,
        None,
        CanisterSettings {
            environment_variables: Some(vec![
                EnvironmentVariable { name: "RESEND_API_KEY".into(), value: format!("re_{}", "test".repeat(8)) },
                EnvironmentVariable { name: "SUBNET_MAIL_FROM".into(), value: "subnet.ee <info@subnet.ee>".into() },
                EnvironmentVariable { name: "SUBNET_MAIL_ORIGIN".into(), value: "https://www.subnet.ee".into() },
            ]),
            ..Default::default()
        },
    )
    .unwrap();
    let result: DriveResult<DriveEntry> = call(&pic, id, b, "drive_initialize", ());
    result.unwrap();
    verify_mail(&pic, id, a, "owner@example.com", "email-upgrade-owner");
    verify_mail(&pic, id, b, "reader@example.com", "email-upgrade-reader");
    pic.upgrade_canister(id, after, encode_args(()).unwrap(), None).unwrap();
    let profile: CloudResult<EmailProfile> = call(&pic, id, a, "email_profile", ());
    assert_eq!(profile.unwrap().emails, vec!["owner@example.com"]);
    let current: DriveResult<DriveEntry> = call(&pic, id, a, "drive_get", (file.object.id,));
    assert_eq!(current.unwrap().latest.unwrap().sha256, file.latest.as_ref().unwrap().sha256);
    let forbidden: DriveResult<DriveEntry> = call(&pic, id, b, "drive_get", (file.object.id,));
    assert!(forbidden.is_err());
    assert!(pic.update_call(id, Principal::anonymous(), "email_profile", encode_args(()).unwrap()).is_err());
    let bytes = pic.query_call(id, a, "drive_current_chunk", encode_args((file.object.id, file.latest.unwrap().id, 0u64)).unwrap()).unwrap();
    assert_eq!(decode_one::<DriveResult<Vec<u8>>>(&bytes).unwrap().unwrap(), b"original file");
}

#[test]
fn only_controller_can_read_rotate_or_remove_canister_mail_settings() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set email Wasm")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let owner = Principal::self_authenticating([81; 32]);
    let user = Principal::self_authenticating([82; 32]);
    let id = pic.create_canister_with_settings(Some(owner), None);
    pic.install_canister(id, wasm.clone(), encode_args(()).unwrap(), Some(owner));
    let variables = |key: &str| {
        vec![
            EnvironmentVariable { name: "RESEND_API_KEY".into(), value: key.into() },
            EnvironmentVariable { name: "SUBNET_MAIL_FROM".into(), value: "subnet.ee <info@subnet.ee>".into() },
            EnvironmentVariable { name: "SUBNET_MAIL_ORIGIN".into(), value: "https://www.subnet.ee".into() },
        ]
    };
    let first = format!("re_{}", "dummy".repeat(6));
    let config = CanisterSettings { environment_variables: Some(variables(&first)), ..Default::default() };
    for caller in [user, Principal::anonymous()] {
        assert!(pic.update_canister_settings(id, Some(caller), config.clone()).is_err());
        assert!(pic.canister_status(id, Some(caller)).is_err());
    }
    pic.update_canister_settings(id, Some(owner), config).unwrap();
    pic.upgrade_canister(id, wasm, encode_args(()).unwrap(), Some(owner)).unwrap();
    assert_eq!(pic.canister_status(id, Some(owner)).unwrap().settings.environment_variables, variables(&first));
    // Users see availability, never provider configuration or the credential.
    let root: DriveResult<DriveEntry> = call(&pic, id, user, "drive_initialize", ());
    root.unwrap();
    let profile: CloudResult<EmailProfile> = call(&pic, id, user, "email_profile", ());
    assert!(profile.unwrap().enabled);
    for caller in [user, Principal::anonymous()] {
        assert!(pic.canister_status(id, Some(caller)).is_err());
        assert!(pic.update_canister_settings(id, Some(caller), CanisterSettings { environment_variables: Some(vec![]), ..Default::default() }).is_err());
    }
    let rotated = format!("re_{}", "rotated".repeat(5));
    pic.update_canister_settings(id, Some(owner), CanisterSettings { environment_variables: Some(variables(&rotated)), ..Default::default() }).unwrap();
    assert_eq!(pic.canister_status(id, Some(owner)).unwrap().settings.environment_variables, variables(&rotated));
    pic.update_canister_settings(id, Some(owner), CanisterSettings { environment_variables: Some(vec![]), ..Default::default() }).unwrap();
    let profile: CloudResult<EmailProfile> = call(&pic, id, user, "email_profile", ());
    assert!(!profile.unwrap().enabled);
}

#[test]
fn email_sharing_acceptance_revocation_and_address_transfer_use_principal_permissions() {
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let id = pic.create_canister();
    pic.add_cycles(id, 100_000_000_000_000);
    pic.install_canister(id, std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").unwrap()).unwrap(), encode_args(()).unwrap(), None);
    pic.update_canister_settings(
        id,
        None,
        CanisterSettings {
            environment_variables: Some(vec![
                EnvironmentVariable { name: "RESEND_API_KEY".into(), value: format!("re_{}", "test".repeat(8)) },
                EnvironmentVariable { name: "SUBNET_MAIL_FROM".into(), value: "subnet.ee <info@subnet.ee>".into() },
                EnvironmentVariable { name: "SUBNET_MAIL_ORIGIN".into(), value: "https://www.subnet.ee".into() },
            ]),
            ..Default::default()
        },
    )
    .unwrap();
    let owner = Principal::self_authenticating([91; 32]);
    let reader = Principal::self_authenticating([92; 32]);
    let newcomer = Principal::self_authenticating([93; 32]);
    for user in [owner, reader, newcomer] {
        let root: DriveResult<DriveEntry> = call(&pic, id, user, "drive_initialize", ());
        root.unwrap();
    }
    let root: DriveResult<DriveEntry> = call(&pic, id, owner, "drive_initialize", ());
    let file: DriveResult<DriveEntry> = call(
        &pic,
        id,
        owner,
        "drive_create_file",
        (InlineFile {
            parent: root.unwrap().object.id,
            name: "Confidential.txt".into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: b"private content".to_vec(),
            request_id: "email-flow-file-create".into(),
        },),
    );
    let object = file.unwrap().object.id;
    verify_mail(&pic, id, owner, "owner@example.com", "owner-verification");
    verify_mail(&pic, id, reader, "reader@example.com", "reader-verification");
    let share = |email: &str, role, request: &str| {
        let message = pic
            .submit_call(
                id,
                owner,
                "email_share",
                encode_args((EmailShare { grant_expires_at: None, object, email: email.into(), role, request_id: request.into() },)).unwrap(),
            )
            .unwrap();
        let payload = deliver(&pic, email);
        assert!(!payload["text"].as_str().unwrap().contains("Confidential"));
        let invitation = decode_one::<CloudResult<EmailInvitation>>(&pic.await_call(message).unwrap()).unwrap().unwrap();
        assert_eq!(invitation.delivery, DeliveryStatus::Accepted);
        (invitation, payload["text"].as_str().unwrap().to_string())
    };
    let (known, text) = share("reader@example.com", EmailRole::Viewer, "share-known-reader");
    assert!(known.accepted);
    assert!(text.contains(&format!("/pilveruum/file/{object}")));
    let readable: DriveResult<DriveEntry> = call(&pic, id, reader, "drive_get", (object,));
    assert_eq!(readable.unwrap().capabilities, Capabilities::READ);
    let (pending, text) = share("new@example.com", EmailRole::Editor, "share-new-reader");
    assert!(!pending.accepted);
    assert!(text.contains(&format!("/pilveruum/#invite/{}", pending.id)));
    for user in [reader, newcomer] {
        let denied: CloudResult<sovereign_cloud::ObjectId> = call(&pic, id, user, "email_accept", (pending.id,));
        assert!(denied.is_err());
    }
    verify_mail(&pic, id, newcomer, "new@example.com", "new-verification");
    let denied: DriveResult<DriveEntry> = call(&pic, id, newcomer, "drive_get", (object,));
    assert!(denied.is_err(), "Mailbox verification alone must not accept an invitation");
    let accepted: CloudResult<sovereign_cloud::ObjectId> = call(&pic, id, newcomer, "email_accept", (pending.id,));
    assert_eq!(accepted.unwrap(), object);
    let editable: DriveResult<DriveEntry> = call(&pic, id, newcomer, "drive_get", (object,));
    let rights = editable.unwrap().capabilities;
    assert!(rights.read && rights.edit && rights.delete && !rights.create);
    let grants: DriveResult<Vec<AccessGrant>> = call(&pic, id, owner, "drive_grants", (object,));
    let grant = grants.unwrap().into_iter().find(|g| g.subject == Participant::User(newcomer)).unwrap();
    let revoked: DriveResult<AccessGrant> = call(
        &pic,
        id,
        owner,
        "drive_set_grant",
        (GrantRequest {
            request_id: "revoke-new-reader".into(),
            id: Some(grant.id),
            expected_version: Some(grant.version),
            root: object,
            subject: grant.subject,
            capabilities: grant.capabilities,
            expires_at: None,
            active: false,
        },),
    );
    revoked.unwrap();
    let retry: CloudResult<sovereign_cloud::ObjectId> = call(&pic, id, newcomer, "email_accept", (pending.id,));
    assert!(retry.is_err(), "Accept retry must not recreate revoked access");
    // Moving a mailbox changes future address resolution, never existing file grants.
    pic.advance_time(std::time::Duration::from_secs(61));
    let (proof, code) = request_mail(&pic, id, newcomer, "reader@example.com", "transfer-verification");
    let result: CloudResult<EmailVerified> = call(&pic, id, newcomer, "email_verify", (proof.id, code));
    assert_eq!(result.unwrap(), EmailVerified::TransferRequired);
    let old: CloudResult<EmailProfile> = call(&pic, id, reader, "email_profile", ());
    assert_eq!(old.unwrap().emails, vec!["reader@example.com"]);
    let moved: CloudResult<EmailVerified> = call(&pic, id, newcomer, "email_confirm_transfer", (proof.id,));
    assert_eq!(moved.unwrap(), EmailVerified::Added);
    let old: CloudResult<EmailProfile> = call(&pic, id, reader, "email_profile", ());
    assert!(old.unwrap().emails.is_empty());
    let old_access: DriveResult<DriveEntry> = call(&pic, id, reader, "drive_get", (object,));
    assert!(old_access.is_ok());
    let no_inherited_access: DriveResult<DriveEntry> = call(&pic, id, newcomer, "drive_get", (object,));
    assert!(no_inherited_access.is_err());
    share("reader@example.com", EmailRole::Viewer, "share-moved-address");
    let future_access: DriveResult<DriveEntry> = call(&pic, id, newcomer, "drive_get", (object,));
    assert_eq!(future_access.unwrap().capabilities, Capabilities::READ);
}

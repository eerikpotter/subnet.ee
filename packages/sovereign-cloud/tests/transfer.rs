use candid::{Principal, encode_one};
use futures::executor::block_on;
use sha2::{Digest, Sha256};
use sovereign_cloud::{CloudResult, FileClient, TransferEvent, Transport, UploadState, protocol::*};
use std::cell::RefCell;
use std::collections::VecDeque;

type Step = (&'static str, bool, CloudResult<Vec<u8>>);
struct Mock {
    steps: RefCell<VecDeque<Step>>,
}
impl Mock {
    fn reply<T: candid::CandidType>(value: T) -> CloudResult<Vec<u8>> {
        Ok(encode_one(Ok::<_, String>(value)).unwrap())
    }
    fn take(&self, method: &str, query: bool) -> CloudResult<Vec<u8>> {
        let (expected, expected_query, result) = self.steps.borrow_mut().pop_front().expect("unexpected network request");
        assert_eq!(method, expected);
        assert_eq!(query, expected_query, "manifest must use a consensus update");
        result
    }
}
impl Transport for Mock {
    async fn update(&self, method: &str, _: Vec<u8>) -> CloudResult<Vec<u8>> {
        self.take(method, false)
    }
    async fn query(&self, method: &str, _: Vec<u8>) -> CloudResult<Vec<u8>> {
        self.take(method, true)
    }
}
fn revision(bytes: &[u8]) -> Revision {
    Revision {
        id: 8,
        bytes: bytes.len() as u64,
        sha256: Sha256::digest(bytes).to_vec(),
        created_at: 1,
        author: Principal::anonymous(),
        chunk_count: bytes.len().div_ceil(CHUNK_BYTES) as u64,
    }
}
fn state() -> UploadState {
    UploadState {
        file: Some(Project { id: 1, owner: Principal::anonymous(), name: "x".into(), created_at: 0, revisions: vec![], pending_upload: None }),
        upload: Some(7),
        expected_revision: None,
        name: "x".into(),
        request_id: "one-snapshot".into(),
    }
}
#[test]
fn a_lost_commit_response_can_be_retried_without_resending_chunks() {
    let bytes = b"document";
    let client = FileClient::with_transport(Mock {
        steps: RefCell::new(VecDeque::from([("commit_upload", false, Err("connection lost".into())), ("commit_upload", false, Mock::reply(revision(bytes)))])),
    });
    let mut attempt = state();
    assert!(block_on(client.save(&mut attempt, bytes, |_| {})).is_err());
    assert_eq!(attempt.upload, Some(7));
    assert_eq!(block_on(client.save(&mut attempt, bytes, |_| {})).unwrap().id, 8);
}
#[test]
fn retry_cannot_report_success_for_different_snapshot_bytes() {
    let client = FileClient::with_transport(Mock { steps: RefCell::new(VecDeque::from([("commit_upload", false, Mock::reply(revision(b"original")))])) });
    assert!(block_on(client.save(&mut state(), b"modified", |_| {})).unwrap_err().contains("snapshot"));
}
#[test]
fn corrupt_query_content_never_reaches_the_editor() {
    let client = FileClient::with_transport(Mock {
        steps: RefCell::new(VecDeque::from([
            ("get_revision", false, Mock::reply(revision(b"original"))),
            ("get_chunk", true, Mock::reply(b"modified".to_vec())),
        ])),
    });
    assert!(block_on(client.download(1, 8, |_| {})).unwrap_err().contains("integrity"));
}
#[test]
fn impossible_manifest_is_rejected_before_querying_bytes() {
    let mut bad = revision(b"original");
    bad.bytes = MAX_DOCUMENT_BYTES + 1;
    let client = FileClient::with_transport(Mock { steps: RefCell::new(VecDeque::from([("get_revision", false, Mock::reply(bad))])) });
    assert!(block_on(client.download(1, 8, |_| {})).unwrap_err().contains("manifest"));
}
#[test]
fn valid_download_reports_completion_and_preserves_exact_bytes() {
    let bytes = vec![42; CHUNK_BYTES + 7];
    let client = FileClient::with_transport(Mock {
        steps: RefCell::new(VecDeque::from([
            ("get_revision", false, Mock::reply(revision(&bytes))),
            ("get_chunk", true, Mock::reply(bytes[..CHUNK_BYTES].to_vec())),
            ("get_chunk", true, Mock::reply(bytes[CHUNK_BYTES..].to_vec())),
        ])),
    });
    let mut completed = false;
    let result = block_on(client.download(1, 8, |event| {
        if let TransferEvent::Downloading { completed: 2, total: 2 } = event {
            completed = true;
        }
    }))
    .unwrap();
    assert!(completed);
    assert_eq!(result.1, bytes);
}

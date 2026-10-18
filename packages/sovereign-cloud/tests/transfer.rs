use candid::{Principal, encode_one};
use futures::executor::block_on;
use sha2::{Digest, Sha256};
use sovereign_cloud::{CloudResult, FileClient, TransferEvent, Transport, UploadState, protocol::*};
use std::cell::RefCell;
use std::collections::VecDeque;

fn object(n: u8) -> sovereign_cloud::ObjectId {
    sovereign_cloud::ObjectId::from_random_bytes([n; 16])
}

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
        file: Some(Project { id: object(1), owner: Principal::anonymous(), name: "x".into(), created_at: 0, revisions: vec![], pending_upload: None }),
        upload: Some(7),
        expected_revision: None,
        name: "x".into(),
        request_id: "one-snapshot".into(),
    }
}
#[test]
fn typed_save_rejects_files_outside_the_apps_format_before_uploading() {
    let client = FileClient::with_transport(Mock { steps: RefCell::new(VecDeque::from([("list_app_files", false, Mock::reply(Vec::<Project>::new()))])) });
    let error = block_on(client.save_typed(&mut state(), b"deck", &FileKind::for_app("deckcraft").unwrap(), |_| {})).unwrap_err();
    assert!(error.contains("different format"));
}

#[test]
fn typed_retry_keeps_the_committed_snapshot_and_does_not_recreate_the_file() {
    let mut attempt = state();
    let client = FileClient::with_transport(Mock {
        steps: RefCell::new(VecDeque::from([
            ("list_app_files", false, Mock::reply(vec![attempt.file.clone().unwrap()])),
            ("commit_upload", false, Mock::reply(revision(b"deck"))),
        ])),
    });
    assert_eq!(block_on(client.save_typed(&mut attempt, b"deck", &FileKind::for_app("deckcraft").unwrap(), |_| {})).unwrap().id, 8);
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
    assert!(block_on(client.download(object(1), 8, |_| {})).unwrap_err().contains("integrity"));
}
#[test]
fn impossible_manifest_is_rejected_before_querying_bytes() {
    let mut bad = revision(b"original");
    bad.bytes = MAX_DOCUMENT_BYTES + 1;
    let client = FileClient::with_transport(Mock { steps: RefCell::new(VecDeque::from([("get_revision", false, Mock::reply(bad))])) });
    assert!(block_on(client.download(object(1), 8, |_| {})).unwrap_err().contains("manifest"));
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
    let result = block_on(client.download(object(1), 8, |event| {
        if let TransferEvent::Downloading { completed: 2, total: 2 } = event {
            completed = true;
        }
    }))
    .unwrap();
    assert!(completed);
    assert_eq!(result.1, bytes);
}

#[derive(Default)]
struct ConcurrentState {
    active: usize,
    maximum: usize,
    started: Vec<u64>,
    finished: Vec<u64>,
    committed: bool,
    aborted: bool,
}
struct ConcurrentMock {
    state: std::rc::Rc<RefCell<ConcurrentState>>,
    fail: Option<u64>,
}
impl Transport for ConcurrentMock {
    async fn update(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>> {
        use sovereign_cloud::drive::*;
        let stage = DriveUpload { id: 7, file: object(8), committed: None };
        match method {
            "drive_begin_upload" => Ok(encode_one(Ok::<_, DriveError>(stage)).unwrap()),
            "drive_put_chunk" => {
                let (upload, index, bytes): (u64, u64, Vec<u8>) = candid::decode_args(&args).unwrap();
                assert_eq!(upload, 7);
                assert!(!bytes.is_empty());
                {
                    let mut s = self.state.borrow_mut();
                    s.active += 1;
                    s.maximum = s.maximum.max(s.active);
                    s.started.push(index);
                }
                let mut yields = if index % 4 == 0 { 3 } else { 1 };
                futures::future::poll_fn(|cx| {
                    if yields > 0 {
                        yields -= 1;
                        cx.waker().wake_by_ref();
                        std::task::Poll::Pending
                    } else {
                        std::task::Poll::Ready(())
                    }
                })
                .await;
                {
                    let mut s = self.state.borrow_mut();
                    s.active -= 1;
                    s.finished.push(index);
                }
                if self.fail == Some(index) { Err("lost chunk response".into()) } else { Ok(encode_one(Ok::<_, DriveError>(())).unwrap()) }
            }
            "drive_abort_upload" => {
                assert_eq!(self.state.borrow().active, 0);
                self.state.borrow_mut().aborted = true;
                Ok(encode_one(Ok::<_, DriveError>(())).unwrap())
            }
            "drive_commit_upload" => {
                assert_eq!(self.state.borrow().active, 0);
                self.state.borrow_mut().committed = true;
                Ok(encode_one(Ok::<_, DriveError>(stage)).unwrap())
            }
            _ => panic!("unexpected method {method}"),
        }
    }
    async fn query(&self, _: &str, _: Vec<u8>) -> CloudResult<Vec<u8>> {
        panic!("unexpected query")
    }
}
#[test]
fn drive_upload_is_bounded_reports_completed_chunks_and_drains_errors_before_retry() {
    use sovereign_cloud::drive::*;
    let target = UploadTarget::New { parent: object(1), name: "parallel.txt".into(), format: FileFormat { media_type: "text/plain".into(), craft: None } };
    let bytes = vec![42; CHUNK_BYTES * 5 + 1];
    for fail in [None, Some(1)] {
        let state = std::rc::Rc::new(RefCell::new(ConcurrentState::default()));
        let client = FileClient::with_transport(ConcurrentMock { state: state.clone(), fail });
        let mut progress = vec![];
        let result = block_on(client.drive_save(target.clone(), &bytes, "concurrent-upload-1".into(), |done, total| progress.push((done, total))));
        let s = state.borrow();
        assert_eq!(s.maximum, 4);
        assert_eq!(s.active, 0);
        assert_eq!(progress[0], (0, 6));
        assert_eq!(s.started.len(), s.finished.len());
        if fail.is_some() {
            assert!(result.is_err());
            assert!(!s.committed);
            assert_eq!(s.started.len(), 4);
        } else {
            assert!(result.is_ok());
            assert!(s.committed);
            assert_eq!(progress.last(), Some(&(6, 6)));
            assert_ne!(s.finished, s.started);
        }
    }
}

fn craft_entry() -> sovereign_cloud::drive::DriveEntry {
    use sovereign_cloud::drive::*;
    DriveEntry {
        object: DriveObject {
            id: object(21),
            space: object(9),
            owner: Principal::self_authenticating([11; 32]),
            parent: Some(object(7)),
            name: "Shared deck".into(),
            kind: ObjectKind::File(FileFormat { media_type: "application/octet-stream".into(), craft: FileKind::for_app("deckcraft") }),
            version: 1,
            created_at: 0,
            updated_at: 0,
            trashed_at: None,
        },
        capabilities: Capabilities::READ,
        latest: Some(revision(b"deck")),
        attribution: None,
        sharing: None,
    }
}
fn drive_reply<T: candid::CandidType>(value: T) -> CloudResult<Vec<u8>> {
    Ok(encode_one(Ok::<_, sovereign_cloud::drive::DriveError>(value)).unwrap())
}
#[test]
fn craft_open_allows_shared_read_but_rejects_wrong_space_format_and_trashed_entries() {
    let kind = FileKind::for_app("deckcraft").unwrap();
    for scenario in 0..5 {
        let mut entry = craft_entry();
        let space = if scenario == 1 { 10 } else { 9 };
        if scenario == 2 {
            entry.object.kind = sovereign_cloud::drive::ObjectKind::Folder;
        }
        if scenario == 3 {
            entry.object.trashed_at = Some(1);
        }
        if scenario == 4 {
            entry.capabilities.read = false;
        }
        let client = FileClient::with_transport(Mock { steps: RefCell::new(VecDeque::from([("drive_get", false, drive_reply(entry))])) });
        assert_eq!(block_on(client.drive_craft_file(object(space), object(21), &kind)).is_ok(), scenario == 0);
    }
}
#[test]
fn read_only_and_wrong_format_craft_replacements_never_begin_an_upload() {
    for (entry, kind) in [(craft_entry(), FileKind::for_app("deckcraft").unwrap()), (craft_entry(), FileKind::for_app("gridcraft").unwrap())] {
        let client = FileClient::with_transport(Mock { steps: RefCell::new(VecDeque::from([("drive_get", false, drive_reply(entry))])) });
        assert!(
            block_on(client.drive_save_craft(
                sovereign_cloud::drive::UploadTarget::Replace { file: object(21), expected_revision: 8 },
                b"changed",
                "craft-read-denied-001".into(),
                &kind,
                |_, _| {}
            ))
            .is_err()
        );
    }
}

#[test]
fn cancellation_drains_inflight_chunks_and_aborts_without_publishing() {
    use sovereign_cloud::drive::*;
    let state = std::rc::Rc::new(RefCell::new(ConcurrentState::default()));
    let client = FileClient::with_transport(ConcurrentMock { state: state.clone(), fail: Some(2) });
    let target = UploadTarget::New { parent: object(1), name: "cancel.txt".into(), format: FileFormat { media_type: "text/plain".into(), craft: None } };
    let result = block_on(client.drive_save_controlled(target, &vec![42; CHUNK_BYTES * 6], "cancel-upload-01".into(), |done, _| done == 0));
    assert_eq!(result.unwrap_err(), "Upload cancelled");
    let s = state.borrow();
    assert!(s.aborted);
    assert!(!s.committed);
    assert_eq!(s.active, 0);
    assert_eq!(s.started.len(), 4);
    assert_eq!(s.started.len(), s.finished.len());
}

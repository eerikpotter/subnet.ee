//! Legacy-compatible cloud envelope. File bytes are opaque to the service.
#![forbid(unsafe_code)]

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

pub const CHUNK_BYTES: usize = 512 * 1024;
pub const MAX_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;
pub const USER_QUOTA_BYTES: u64 = 256 * 1024 * 1024;
pub const TOTAL_QUOTA_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_PROJECTS: u64 = 1000;
pub const MAX_USER_PROJECTS: usize = 20;
pub const MAX_REVISIONS: usize = 20;
pub type CloudResult<T> = Result<T, String>;

/// Client vocabulary only: the V1 wire name remains Project for compatibility.
pub type FileMetadata = Project;

/// IDs are scoped to a logical service. Resolve that service to a canister separately.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileRef {
    pub service: String,
    pub object: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct Revision {
    pub id: u64,
    pub bytes: u64,
    pub sha256: Vec<u8>,
    pub created_at: u64,
    pub author: Principal,
    pub chunk_count: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Project {
    pub id: u64,
    pub owner: Principal,
    pub name: String,
    pub created_at: u64,
    pub revisions: Vec<Revision>,
    pub pending_upload: Option<u64>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct BeginUpload {
    pub project_id: u64,
    pub expected_revision: Option<u64>,
    pub bytes: u64,
    pub sha256: Vec<u8>,
    pub request_id: String,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct Upload {
    pub id: u64,
    pub request: BeginUpload,
    pub created_at: u64,
    pub committed: Option<Revision>,
}

/// Account IDs and space IDs are stable within a logical service, not a deployment address.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct Account {
    pub id: u64,
    pub owner: Principal,
    pub personal_space: u64,
    pub created_at: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct AccountContext {
    pub account: Account,
    pub used_bytes: u64,
    pub quota_bytes: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileKind {
    pub format_id: String,
    pub format_version: u32,
    pub created_by_app: String,
}
impl FileKind {
    pub fn photocraft() -> Self {
        Self { format_id: "photocraft.pcraft".into(), format_version: 1, created_by_app: "photocraft".into() }
    }
    pub fn for_app(app: &str) -> Option<Self> {
        let format = match app {
            "photocraft" => "photocraft.pcraft",
            "pdfcraft" => "pdfcraft.pdf",
            "wordcraft" => "wordcraft.wcraft",
            "deckcraft" => "deckcraft.deckcraft",
            "soundcraft" => "soundcraft.scraft-bundle",
            _ => return None,
        };
        Some(Self { format_id: format.into(), format_version: 1, created_by_app: app.into() })
    }
    /// Format metadata is a compatibility hint, never an authorization credential.
    pub fn validate(&self) -> CloudResult<()> {
        if Self::for_app(&self.created_by_app).as_ref() == Some(self) { Ok(()) } else { Err("Unsupported file format or version".into()) }
    }
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct FileSummary {
    pub id: u64,
    pub space_id: u64,
    pub name: String,
    pub kind: FileKind,
    pub latest: Revision,
    pub version_count: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct FilePage {
    pub files: Vec<FileSummary>,
    pub next_cursor: Option<u64>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct FileDetails {
    pub project: Project,
    pub space_id: u64,
    pub kind: FileKind,
}

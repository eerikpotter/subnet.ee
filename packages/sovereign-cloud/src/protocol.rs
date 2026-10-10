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

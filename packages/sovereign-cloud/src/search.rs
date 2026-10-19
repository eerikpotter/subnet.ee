//! Revision-bound derived search; canonical files and live ACL remain authoritative.
use crate::{
    ObjectId,
    drive::{DriveEntry, PageCursor},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexState {
    Pending,
    Ready,
    Empty,
    Partial,
    Unsupported,
    TooLarge,
    Unreadable,
    Failed,
}
#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct ContentStatus {
    pub object: ObjectId,
    pub revision: u64,
    pub extractor_version: u32,
    pub state: IndexState,
    pub sections: u32,
}
#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct ContentQuery {
    pub text: String,
    pub folder: Option<ObjectId>,
    pub cursor: Option<PageCursor>,
    pub limit: u32,
}
#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct ContentHit {
    pub entry: DriveEntry,
    pub revision: u64,
    pub location: String,
    pub snippet: String,
    pub partial: bool,
}
#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct ContentPage {
    pub hits: Vec<ContentHit>,
    pub next: Option<PageCursor>,
}

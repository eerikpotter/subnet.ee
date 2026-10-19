//! Explicit public read contract: no account, recipient, private path or history data.
use crate::{ObjectId, drive::*, protocol::Revision};
use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct PublicLink {
    pub enabled: bool,
    pub expires_at: Option<u64>,
    pub version: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct SetPublicLink {
    pub object: ObjectId,
    pub enabled: bool,
    pub expires_at: Option<u64>,
    pub expected_version: u64,
    pub request_id: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct PublicLinkState {
    pub direct: PublicLink,
    pub inherited: bool,
    pub effective: bool,
    pub allowed: bool,
}
/// Verified current bytes, without the private author identity.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ContentManifest {
    pub id: u64,
    pub bytes: u64,
    pub sha256: Vec<u8>,
    pub chunk_count: u64,
}
impl From<Revision> for ContentManifest {
    fn from(r: Revision) -> Self {
        Self { id: r.id, bytes: r.bytes, sha256: r.sha256, chunk_count: r.chunk_count }
    }
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct PublicItem {
    pub id: ObjectId,
    pub name: String,
    pub kind: ObjectKind,
    pub updated_at: u64,
    pub latest: Option<ContentManifest>,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct PublicPage {
    pub entries: Vec<PublicItem>,
    pub next: Option<PageCursor>,
}

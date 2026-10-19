//! Team authority belongs to a space, never to its founder's principal.
use crate::{ObjectId, drive::Capabilities};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum SpaceRole {
    Manager,
    Editor,
    Viewer,
}
impl SpaceRole {
    pub fn capabilities(self) -> Capabilities {
        match self {
            Self::Viewer => Capabilities::READ,
            Self::Manager | Self::Editor => Capabilities::ALL,
        }
    }
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct TeamSpace {
    pub id: ObjectId,
    pub name: String,
    pub created_by: Principal,
    pub created_at: u64,
    pub version: u64,
    pub external_sharing: bool,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct SpaceMember {
    pub principal: Principal,
    pub role: SpaceRole,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct SpaceView {
    pub space: TeamSpace,
    pub role: SpaceRole,
    pub used_bytes: u64,
    pub quota_bytes: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct CreateSpace {
    pub name: String,
    pub request_id: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ChangeMember {
    pub space: ObjectId,
    pub principal: Principal,
    pub role: Option<SpaceRole>,
    pub expected_version: u64,
    pub request_id: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ChangeSpace {
    pub space: ObjectId,
    pub name: String,
    pub external_sharing: bool,
    pub expected_version: u64,
    pub request_id: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct SpaceMembers {
    pub space: TeamSpace,
    pub members: Vec<SpaceMember>,
    pub next: Option<Principal>,
}
/// Presentation only; server methods always evaluate current authority again.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct SpaceAccess {
    pub team_name: Option<String>,
    pub manage: bool,
}

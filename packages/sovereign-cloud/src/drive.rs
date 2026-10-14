//! Shared drive vocabulary. Transport payloads never confer an authenticated actor.
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct Capabilities {
    pub read: bool,
    pub propose: bool,
    pub create: bool,
    pub edit: bool,
    pub delete: bool,
}
impl Capabilities {
    pub const NONE: Self = Self { read: false, propose: false, create: false, edit: false, delete: false };
    pub const READ: Self = Self { read: true, ..Self::NONE };
    pub const PROPOSE: Self = Self { propose: true, ..Self::READ };
    pub const EDIT: Self = Self { read: true, propose: true, create: true, edit: true, delete: false };
    pub const ALL: Self = Self { delete: true, ..Self::EDIT };
    pub fn union(self, other: Self) -> Self {
        Self {
            read: self.read || other.read,
            propose: self.propose || other.propose,
            create: self.create || other.create,
            edit: self.edit || other.edit,
            delete: self.delete || other.delete,
        }
    }
    pub fn intersect(self, other: Self) -> Self {
        Self {
            read: self.read && other.read,
            propose: self.propose && other.propose,
            create: self.create && other.create,
            edit: self.edit && other.edit,
            delete: self.delete && other.delete,
        }
        .with_read_prerequisite()
    }
    pub fn includes(self, other: Self) -> bool {
        self.intersect(other) == other
    }
    pub fn valid(self) -> bool {
        self.read || self == Self::NONE
    }
    fn with_read_prerequisite(self) -> Self {
        if self.read { self } else { Self::NONE }
    }
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum Participant {
    User(Principal),
    Agent(u64),
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct AgentConnection {
    pub id: u64,
    pub sponsor: Principal,
    pub name: String,
    pub active: bool,
    pub created_at: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileFormat {
    pub media_type: String,
    pub craft: Option<crate::protocol::FileKind>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum ObjectKind {
    Folder,
    File(FileFormat),
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct DriveObject {
    pub id: u64,
    pub space: u64,
    pub owner: Principal,
    pub parent: Option<u64>,
    pub name: String,
    pub kind: ObjectKind,
    pub version: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub trashed_at: Option<u64>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct AccessGrant {
    pub id: u64,
    pub root: u64,
    pub subject: Participant,
    pub capabilities: Capabilities,
    pub expires_at: Option<u64>,
    pub active: bool,
    pub version: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum DriveError {
    Inaccessible,
    InvalidInput(String),
    Conflict,
    NameConflict,
    Limit(String),
    Expired,
    Internal,
}
pub type DriveResult<T> = Result<T, DriveError>;

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct Attribution {
    pub actor: Participant,
    pub sponsor: Option<Principal>,
    pub credential: Option<String>,
    pub mutation: String,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DriveEntry {
    pub object: DriveObject,
    pub capabilities: Capabilities,
    pub latest: Option<crate::protocol::Revision>,
    pub attribution: Option<Attribution>,
    #[serde(default)]
    pub sharing: Option<SharingStatus>,
}

#[derive(Clone, Debug, Default, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct SharingStatus {
    pub direct: bool,
    pub inherited: bool,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct GrantRequest {
    pub request_id: String,
    pub id: Option<u64>,
    pub expected_version: Option<u64>,
    pub root: u64,
    pub subject: Participant,
    pub capabilities: Capabilities,
    pub expires_at: Option<u64>,
    pub active: bool,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct InlineFile {
    pub parent: u64,
    pub name: String,
    pub format: FileFormat,
    pub bytes: Vec<u8>,
    pub request_id: String,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct CreateFolder {
    pub parent: u64,
    pub name: String,
    pub request_id: String,
}

/// Opaque to callers: scope and actor are validated on each page request.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DriveCursor {
    pub scope: Option<u64>,
    pub actor: Principal,
    pub tree_version: u64,
    pub after: u64,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DrivePage {
    pub entries: Vec<DriveEntry>,
    pub next: Option<DriveCursor>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub enum OrganizationChange {
    Rename(String),
    Move(u64),
    Trash,
    Restore,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct OrganizeRequest {
    pub object: u64,
    pub expected_version: u64,
    pub request_id: String,
    pub change: OrganizationChange,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct MutationReceipt {
    pub object: u64,
    pub version: u64,
}

impl std::fmt::Display for DriveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inaccessible => f.write_str("Not found or access unavailable"),
            Self::InvalidInput(message) | Self::Limit(message) => f.write_str(message),
            Self::Conflict => f.write_str("This item changed; refresh and try again"),
            Self::NameConflict => f.write_str("An item with this name already exists"),
            Self::Expired => f.write_str("This operation expired; start again"),
            Self::Internal => f.write_str("Cloud operation could not be completed"),
        }
    }
}

/// Uploads remain private staging data until commit. A replacement keeps its format.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum UploadTarget {
    New { parent: u64, name: String, format: FileFormat },
    Replace { file: u64, expected_revision: u64 },
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct DriveUploadRequest {
    pub target: UploadTarget,
    pub bytes: u64,
    pub sha256: Vec<u8>,
    pub request_id: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DriveUpload {
    pub id: u64,
    pub file: u64,
    pub committed: Option<crate::protocol::Revision>,
}

/// A consistent folder view from one authorized consensus call.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DriveBrowse {
    pub catalog: DriveCatalog,
    pub folder: DriveEntry,
    pub breadcrumbs: Vec<DriveEntry>,
    pub page: DrivePage,
}

/// Initial personal-drive view and account usage in one consensus response.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DriveHome {
    pub account: crate::protocol::AccountContext,
    pub view: DriveBrowse,
}

/// Bounded metadata prefetch; completeness is explicit, never inferred from absence.
#[derive(Clone, Debug, Default, CandidType, Deserialize, Serialize)]
pub struct DriveCatalog {
    pub entries: Vec<DriveEntry>,
    pub complete_folders: Vec<u64>,
}

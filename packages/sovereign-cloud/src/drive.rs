//! Shared drive vocabulary. Transport payloads never confer an authenticated actor.
use crate::ObjectId;
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

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
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
    Shortcut(ObjectId),
    File(FileFormat),
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct DriveObject {
    pub id: ObjectId,
    pub space: ObjectId,
    pub owner: Principal,
    pub parent: Option<ObjectId>,
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
    pub root: ObjectId,
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

/// Caller-visible references only; inaccessible ancestors never supply names.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ObjectLocation {
    pub id: ObjectId,
    pub name: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct AccessSource {
    pub inherited: bool,
    pub location: Option<ObjectLocation>,
    pub capabilities: Capabilities,
    pub expires_at: Option<u64>,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DriveDetails {
    pub entry: DriveEntry,
    pub locations: Vec<ObjectLocation>,
    pub access: Vec<AccessSource>,
    pub revision_count: u64,
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
    pub root: ObjectId,
    pub subject: Participant,
    pub capabilities: Capabilities,
    pub expires_at: Option<u64>,
    pub active: bool,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct InlineFile {
    pub parent: ObjectId,
    pub name: String,
    pub format: FileFormat,
    pub bytes: Vec<u8>,
    pub request_id: String,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct CreateFolder {
    pub parent: ObjectId,
    pub name: String,
    pub request_id: String,
}

/// Opaque continuation. Only the service interprets or authenticates its bytes.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct PageCursor {
    pub token: Vec<u8>,
}
pub type DriveCursor = PageCursor;

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DrivePage {
    pub entries: Vec<DriveEntry>,
    pub next: Option<DriveCursor>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub enum OrganizationChange {
    Rename(String),
    Move(ObjectId),
    MoveReviewed { parent: ObjectId, tree_version: u64 },
    Trash,
    Restore,
    RestoreReviewed { parent: ObjectId, name: String, tree_version: u64 },
}

/// Derived owner-visible impact. No grants or ACL snapshots are persisted.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct MoveAccessChange {
    pub subject: Participant,
    pub gains: bool,
    pub loses: bool,
    pub objects: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct MovePreview {
    pub object_version: u64,
    pub tree_version: u64,
    pub affected_objects: u64,
    pub affected_subjects: u64,
    pub changes: Vec<MoveAccessChange>,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct OrganizeRequest {
    pub object: ObjectId,
    pub expected_version: u64,
    pub request_id: String,
    pub change: OrganizationChange,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct MutationReceipt {
    pub object: ObjectId,
    pub version: u64,
}

/// Explicitly trashed roots and the versions the owner confirmed. Descendants
/// are included; objects trashed after this snapshot are not implicitly added.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct PurgeRequest {
    pub items: Vec<MutationReceipt>,
    pub request_id: String,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct PurgeReceipt {
    pub deleted_objects: u64,
    pub freed_bytes: u64,
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
    New { parent: ObjectId, name: String, format: FileFormat },
    Replace { file: ObjectId, expected_revision: u64 },
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct DriveUploadRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automatic: Option<bool>,
    pub target: UploadTarget,
    pub bytes: u64,
    pub sha256: Vec<u8>,
    pub request_id: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct DriveUpload {
    pub id: u64,
    pub file: ObjectId,
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
    pub complete_folders: Vec<ObjectId>,
}

/// A single retrieval contract for browsing, metadata search and personal views.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum BrowseScope {
    Item(ObjectId),
    Folder(ObjectId),
    Subtree(ObjectId),
    All,
    Shared,
    Recent,
    Starred,
    Trash,
}
#[derive(Clone, Copy, Debug, Default, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum SortField {
    #[default]
    Name,
    Modified,
    Size,
    Opened,
}
#[derive(Clone, Copy, Debug, Default, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    List,
    Grid,
}
#[derive(Clone, Debug, Default, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowsePreferences {
    pub view: ViewMode,
    pub sort: SortField,
    pub descending: bool,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowseQuery {
    pub scope: BrowseScope,
    pub text: String,
    /// Empty means any; folder, file, image, audio, video, pdf, text or a Craft app ID.
    pub file_type: String,
    pub owner: Option<Principal>,
    pub modified_after: Option<u64>,
    pub modified_before: Option<u64>,
    pub sort: SortField,
    pub descending: bool,
}
pub type BrowseCursor = PageCursor;
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct BrowseItem {
    pub entry: DriveEntry,
    #[serde(default)]
    pub color: Option<String>,
    pub starred: bool,
    pub opened_at: Option<u64>,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct BrowsePage {
    pub items: Vec<BrowseItem>,
    pub next: Option<BrowseCursor>,
    pub folder: Option<DriveEntry>,
    pub breadcrumbs: Vec<DriveEntry>,
    pub preferences: BrowsePreferences,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct BrowseHome {
    pub account: crate::protocol::AccountContext,
    pub page: BrowsePage,
}

/// Derived explanations, not stored scores or permission-bearing records.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum SuggestionReason {
    UpdatedSinceOpened,
    RecentlyOpened,
    Starred,
    RecentlyCreated,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct SuggestedItem {
    pub item: BrowseItem,
    pub reason: SuggestionReason,
    pub reason_at: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct SuggestedHome {
    pub files: Vec<SuggestedItem>,
    pub folders: Vec<SuggestedItem>,
}

/// Successful mutations, separate from private read/open summaries. Names and
/// locations are resolved with current authority at read time, never snapshotted.
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActivityKind {
    Created,
    ContentSaved,
    Renamed,
    Moved,
    Trashed,
    Restored,
    SharingChanged,
    RevisionLabeled,
    RevisionDeleted,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ActivityEvent {
    pub id: u64,
    pub object: ObjectId,
    pub actor: Participant,
    pub kind: ActivityKind,
    pub at: u64,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ActivityItem {
    pub event: ActivityEvent,
    pub name: String,
    pub trashed: bool,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ActivityPage {
    pub items: Vec<ActivityItem>,
    pub before: Option<u64>,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct StorageItem {
    pub object: ObjectId,
    pub name: String,
    pub current_bytes: u64,
    pub history_bytes: u64,
    pub revision_count: u64,
    pub trashed: bool,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct StorageOverview {
    pub current_bytes: u64,
    pub history_bytes: u64,
    pub trash_bytes: u64,
    pub pending_bytes: u64,
    pub used_bytes: u64,
    pub quota_bytes: u64,
    pub files: Vec<StorageItem>,
}

#[derive(Clone, Debug, Default, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct RevisionProperties {
    pub version: u64,
    pub label: String,
    pub keep: bool,
    pub automatic: bool,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct RevisionInfo {
    pub revision: crate::protocol::Revision,
    pub properties: RevisionProperties,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct RevisionLabelRequest {
    pub object: ObjectId,
    pub revision: u64,
    pub expected_version: u64,
    pub label: String,
    pub keep: bool,
    pub request_id: String,
}
#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct RevisionDeleteRequest {
    pub object: ObjectId,
    pub revision: u64,
    pub expected_object_version: u64,
    pub expected_properties_version: u64,
    pub request_id: String,
}

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct CreateShortcut {
    pub parent: ObjectId,
    pub target: ObjectId,
    pub name: String,
    pub request_id: String,
}

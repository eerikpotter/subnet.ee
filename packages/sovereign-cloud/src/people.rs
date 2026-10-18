//! Account-private contacts and in-app communication, never a source of authority.
use crate::ObjectId;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct Contact {
    pub principal: Principal,
    pub label: String,
}
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub enum NoticeKind {
    Shared,
    AccessRequested,
    AccessDeclined,
    Reminder,
}
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct Notice {
    pub id: u64,
    pub object: ObjectId,
    pub sender: Principal,
    pub kind: NoticeKind,
    pub at: u64,
    pub read: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct InboxItem {
    pub notice: Notice,
    pub name: Option<String>,
    pub message: Option<String>,
    pub can_respond: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct Inbox {
    pub items: Vec<InboxItem>,
    pub notifications: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct AccessDecision {
    pub object: ObjectId,
    pub requester: Principal,
    pub requested_at: u64,
    pub approve: bool,
    pub editor: bool,
    pub expires_at: Option<u64>,
    pub request_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct HiddenItem {
    pub object: ObjectId,
    pub name: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct SharingSafety {
    pub blocked: Vec<Principal>,
    pub hidden: Vec<HiddenItem>,
}
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Deserialize, Serialize)]
pub struct SharingReport {
    pub reporter: Principal,
    pub object: ObjectId,
    pub owner: Principal,
    pub reason: String,
    pub at: u64,
}

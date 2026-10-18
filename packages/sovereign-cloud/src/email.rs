//! Optional Subnet account contacts. Internet Identity remains the login identity.
use crate::ObjectId;
use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeliveryStatus {
    Queued,
    Sending,
    Accepted,
    Retryable,
    Failed,
    Unknown,
    Expired,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct EmailVerification {
    pub id: u64,
    pub email: String,
    pub expires_at: u64,
    pub delivery: DeliveryStatus,
    pub transfer_required: bool,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct EmailProfile {
    pub enabled: bool,
    pub emails: Vec<String>,
    pub pending: Option<EmailVerification>,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmailVerified {
    Added,
    TransferRequired,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct EmailRequest {
    pub email: String,
    pub request_id: String,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmailRole {
    Viewer,
    Editor,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct EmailShare {
    #[serde(default)]
    pub grant_expires_at: Option<u64>,
    pub object: ObjectId,
    pub email: String,
    pub role: EmailRole,
    pub request_id: String,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct EmailInvitation {
    #[serde(default)]
    pub grant_expires_at: Option<u64>,
    pub id: u64,
    pub object: ObjectId,
    pub email: String,
    pub role: EmailRole,
    pub expires_at: u64,
    pub active: bool,
    pub accepted: bool,
    pub delivery: DeliveryStatus,
}

//! Candid subset from Internet Identity release-2026-10-13.
//! Optional fields omitted by the client are decoded as None by II.
#![allow(dead_code)]
use candid::{CandidType, Principal};
use serde::Deserialize;

#[derive(CandidType)]
pub enum StaticCaptchaTrigger {
    CaptchaDisabled,
}
#[derive(CandidType)]
pub enum CaptchaTrigger {
    Static(StaticCaptchaTrigger),
}
#[derive(CandidType)]
pub struct CaptchaConfig {
    pub max_unsolved_captchas: u64,
    pub captcha_trigger: CaptchaTrigger,
}
#[derive(CandidType)]
pub struct Init {
    pub captcha_config: Option<CaptchaConfig>,
}

#[derive(CandidType, Deserialize)]
pub struct Challenge {
    pub challenge_key: String,
}
#[derive(CandidType)]
pub struct ChallengeResult {
    pub key: String,
    pub chars: String,
}
#[derive(CandidType, Deserialize)]
pub enum Purpose {
    #[serde(rename = "authentication")]
    Authentication,
}
#[derive(CandidType, Deserialize)]
pub enum KeyType {
    #[serde(rename = "unknown")]
    Unknown,
}
#[derive(CandidType, Deserialize)]
pub enum Protection {
    #[serde(rename = "unprotected")]
    Unprotected,
}
#[derive(CandidType)]
pub struct Device {
    pub pubkey: Vec<u8>,
    pub alias: String,
    pub purpose: Purpose,
    pub key_type: KeyType,
    pub protection: Protection,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum RegisterResponse {
    #[serde(rename = "registered")]
    Registered { user_number: u64 },
    #[serde(rename = "canister_full")]
    CanisterFull,
    #[serde(rename = "bad_challenge")]
    BadChallenge,
}
#[derive(CandidType)]
pub struct McpConfig {
    pub enabled: bool,
    pub url: Option<String>,
}
#[derive(CandidType, Deserialize, Debug, Clone, Copy)]
pub enum Permissions {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "queries")]
    Queries,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct PreparedRegistration {
    pub user_key: Vec<u8>,
    pub expiration: u64,
    pub trusted_url: String,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct Registration {
    pub expiration: u64,
    pub permissions: Permissions,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct Prepared {
    pub user_key: Vec<u8>,
    pub expiration: u64,
    pub account_number: Option<u64>,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum Error {
    InternalCanisterError(String),
    Unauthorized(Principal),
    NoSuchDelegation,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct Delegation {
    pub pubkey: Vec<u8>,
    pub expiration: u64,
    pub targets: Option<Vec<Principal>>,
    pub permissions: Option<String>,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct Signed {
    pub delegation: Delegation,
    pub signature: Vec<u8>,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum GetDelegationResponse {
    #[serde(rename = "signed_delegation")]
    Signed(Signed),
    #[serde(rename = "no_such_delegation")]
    NoSuchDelegation,
}

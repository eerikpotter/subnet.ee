//! Internal client for trusted first-party apps. No editor, file format or UI dependency.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(all(target_arch = "wasm32", feature = "browser"))]
pub mod browser;
pub mod config;
pub mod protocol;
pub mod transfer;

pub use protocol::CloudResult;
pub use transfer::{FileClient, TransferEvent, Transport, UploadState, args};

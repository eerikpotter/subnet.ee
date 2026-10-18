//! Nonce-free authenticated bookmarks (RFC 5297). Filtered positions never
//! disclose private UUIDs, names or ordering keys, and cannot be forged.
use super::*;
use aes_siv::{KeyInit, siv::Aes256Siv};
use sovereign_cloud::drive::{DriveError, DriveResult, PageCursor};

impl<M: Memory> Store<M> {
    pub(super) fn seal_bookmark<T: Serialize>(&self, value: &T, purpose: &[u8], actor: Principal) -> DriveResult<PageCursor> {
        let bytes = serde_json::to_vec(value).map_err(|_| DriveError::Internal)?;
        if bytes.len() > 4000 {
            return Err(DriveError::Internal);
        }
        let mut cipher = Aes256Siv::new_from_slice(self.bookmark_key.get()).map_err(|_| DriveError::Internal)?;
        let token = cipher.encrypt([purpose, actor.as_slice()], &bytes).map_err(|_| DriveError::Internal)?;
        Ok(PageCursor { token })
    }
    pub(super) fn open_bookmark<T: DeserializeOwned>(&self, cursor: &PageCursor, purpose: &[u8], actor: Principal) -> DriveResult<T> {
        if !(16..=4096).contains(&cursor.token.len()) {
            return Err(DriveError::Conflict);
        }
        let mut cipher = Aes256Siv::new_from_slice(self.bookmark_key.get()).map_err(|_| DriveError::Internal)?;
        let bytes = cipher.decrypt([purpose, actor.as_slice()], &cursor.token).map_err(|_| DriveError::Conflict)?;
        serde_json::from_slice(&bytes).map_err(|_| DriveError::Conflict)
    }
}

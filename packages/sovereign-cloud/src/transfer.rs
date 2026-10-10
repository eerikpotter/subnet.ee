//! V1 file transfer. Checkpoints retain retry state; metadata comes from consensus updates.
use crate::protocol::*;
use candid::{CandidType, decode_one, encode_args};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

#[allow(async_fn_in_trait)]
pub trait Transport {
    async fn update(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>>;
    async fn query(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>>;
}

#[derive(Clone)]
pub struct FileClient<T> {
    transport: T,
}

#[derive(Clone, Debug)]
pub struct UploadState {
    pub file: Option<Project>,
    pub upload: Option<u64>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub request_id: String,
}

#[derive(Clone, Debug)]
pub enum TransferEvent {
    Checkpoint(UploadState),
    Uploading { completed: usize, total: usize },
    Downloading { completed: u64, total: u64 },
    Committing,
}

pub fn args<T: candid::utils::ArgumentEncoder>(args: T) -> CloudResult<Vec<u8>> {
    encode_args(args).map_err(|e| e.to_string())
}

impl<T: Transport> FileClient<T> {
    pub fn with_transport(transport: T) -> Self {
        Self { transport }
    }
    pub async fn update<R: CandidType + DeserializeOwned>(&self, method: &str, args: Vec<u8>) -> CloudResult<R> {
        let bytes = self.transport.update(method, args).await?;
        decode_one::<CloudResult<R>>(&bytes).map_err(|e| e.to_string())?
    }
    pub async fn projects(&self) -> CloudResult<Vec<Project>> {
        self.update("list_projects", args(())?).await
    }
    pub async fn save(&self, state: &mut UploadState, bytes: &[u8], mut notify: impl FnMut(TransferEvent)) -> CloudResult<Revision> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err("File size is outside the service limit".into());
        }
        if state.file.is_none() {
            state.file = Some(self.update("create_project", args((state.name.clone(),))?).await?);
            notify(TransferEvent::Checkpoint(state.clone()));
        }
        let file = state.file.as_ref().ok_or("Cloud file is unavailable")?;
        if let Some(id) = state.upload {
            match self.update::<Revision>("commit_upload", args((id,))?).await {
                Ok(revision) => {
                    if revision.bytes != bytes.len() as u64 || revision.sha256 != Sha256::digest(bytes).to_vec() {
                        return Err("Retry snapshot differs from the committed file".into());
                    }
                    return Ok(revision);
                }
                Err(e) if e == "Upload incomplete" => {}
                Err(e) => return Err(e),
            }
        } else {
            let request = BeginUpload {
                project_id: file.id,
                expected_revision: state.expected_revision,
                bytes: bytes.len() as u64,
                sha256: Sha256::digest(bytes).to_vec(),
                request_id: state.request_id.clone(),
            };
            let upload: Upload = self.update("begin_upload", args((request,))?).await?;
            state.upload = Some(upload.id);
            notify(TransferEvent::Checkpoint(state.clone()));
        }
        let id = state.upload.ok_or("Missing upload ID")?;
        let total = bytes.len().div_ceil(CHUNK_BYTES);
        for (index, chunk) in bytes.chunks(CHUNK_BYTES).enumerate() {
            notify(TransferEvent::Uploading { completed: index, total });
            self.update::<()>("put_chunk", args((id, index as u64, chunk.to_vec()))?).await?;
            notify(TransferEvent::Uploading { completed: index + 1, total });
        }
        notify(TransferEvent::Committing);
        self.update("commit_upload", args((id,))?).await
    }
    pub async fn download(&self, file: u64, revision: u64, mut notify: impl FnMut(TransferEvent)) -> CloudResult<(Revision, Vec<u8>)> {
        let manifest: Revision = self.update("get_revision", args((file, revision))?).await?;
        if manifest.id != revision
            || manifest.bytes > MAX_DOCUMENT_BYTES
            || manifest.sha256.len() != 32
            || manifest.chunk_count != manifest.bytes.div_ceil(CHUNK_BYTES as u64)
        {
            return Err("Invalid file manifest".into());
        }
        let mut bytes = Vec::new();
        for index in 0..manifest.chunk_count {
            notify(TransferEvent::Downloading { completed: index, total: manifest.chunk_count });
            let response = self.transport.query("get_chunk", args((file, revision, index))?).await?;
            let chunk = decode_one::<CloudResult<Vec<u8>>>(&response).map_err(|e| e.to_string())??;
            let expected = (manifest.bytes - index * CHUNK_BYTES as u64).min(CHUNK_BYTES as u64) as usize;
            if chunk.len() != expected {
                return Err("Invalid file chunk".into());
            }
            bytes.extend(chunk);
            notify(TransferEvent::Downloading { completed: index + 1, total: manifest.chunk_count });
        }
        if bytes.len() as u64 != manifest.bytes || Sha256::digest(&bytes).to_vec() != manifest.sha256 {
            return Err("File integrity check failed".into());
        }
        Ok((manifest, bytes))
    }
}

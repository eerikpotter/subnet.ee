//! V1 file transfer. Checkpoints retain retry state; metadata comes from consensus updates.
use crate::ObjectId;
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
    /// Drive failures remain typed on the wire; the UI receives their readable description.
    pub async fn drive_call<R: CandidType + DeserializeOwned>(&self, method: &str, args: Vec<u8>) -> CloudResult<R> {
        let bytes = self.transport.update(method, args).await?;
        decode_one::<crate::drive::DriveResult<R>>(&bytes).map_err(|e| e.to_string())?.map_err(|e| e.to_string())
    }
    pub async fn drive_save(
        &self,
        target: crate::drive::UploadTarget,
        bytes: &[u8],
        request_id: String,
        mut progress: impl FnMut(usize, usize),
    ) -> CloudResult<crate::drive::DriveUpload> {
        self.drive_save_controlled(target, bytes, request_id, |done, total| {
            progress(done, total);
            true
        })
        .await
    }
    /// Returning false cancels before publication. In-flight chunk calls are drained
    /// before abort; once a commit has been sent its result remains authoritative.
    pub async fn drive_save_controlled(
        &self,
        target: crate::drive::UploadTarget,
        bytes: &[u8],
        request_id: String,
        progress: impl FnMut(usize, usize) -> bool,
    ) -> CloudResult<crate::drive::DriveUpload> {
        self.drive_save_with_policy(target, bytes, request_id, None, progress).await
    }
    pub async fn drive_save_with_policy(
        &self,
        target: crate::drive::UploadTarget,
        bytes: &[u8],
        request_id: String,
        automatic: Option<bool>,
        mut progress: impl FnMut(usize, usize) -> bool,
    ) -> CloudResult<crate::drive::DriveUpload> {
        use crate::drive::*;
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err("Files must be at most 64 MiB".into());
        }
        let total = bytes.len().div_ceil(CHUNK_BYTES);
        if !progress(0, total) {
            return Err("Upload cancelled".into());
        }
        let request = DriveUploadRequest { automatic, target, bytes: bytes.len() as u64, sha256: Sha256::digest(bytes).to_vec(), request_id };
        let stage: DriveUpload = self.drive_call("drive_begin_upload", args((request,))?).await?;
        if stage.committed.is_some() {
            return Ok(stage);
        }
        // Drain each bounded batch even after an error: no earlier upload request
        // remains in flight when the caller retries the identical snapshot.
        let mut completed = 0;
        let mut cancelled = false;
        for (batch, chunks) in bytes.chunks(CHUNK_BYTES * 4).enumerate() {
            if !progress(completed, total) {
                cancelled = true;
                break;
            }
            let mut pending = futures::stream::FuturesUnordered::new();
            for (offset, chunk) in chunks.chunks(CHUNK_BYTES).enumerate() {
                pending.push(self.drive_call::<()>("drive_put_chunk", args((stage.id, (batch * 4 + offset) as u64, chunk.to_vec()))?));
            }
            let mut error = None;
            use futures::StreamExt;
            while let Some(result) = pending.next().await {
                match result {
                    Ok(()) => {
                        completed += 1;
                        cancelled |= !progress(completed, total);
                    }
                    Err(e) => {
                        error.get_or_insert(e);
                    }
                }
            }
            if cancelled {
                break;
            }
            if let Some(error) = error {
                return Err(error);
            }
        }
        if cancelled || !progress(completed, total) {
            self.drive_call::<()>("drive_abort_upload", args((stage.id,))?).await?;
            return Err("Upload cancelled".into());
        }
        self.drive_call("drive_commit_upload", args((stage.id,))?).await
    }
    pub async fn drive_download(&self, file: ObjectId, revision: u64) -> CloudResult<Vec<u8>> {
        let entry: crate::drive::DriveEntry = self.drive_call("drive_get", args((file,))?).await?;
        let current = entry.latest.ok_or("File unavailable")?;
        let manifest = if current.id == revision {
            current
        } else {
            let history: Vec<Revision> = self.drive_call("drive_history", args((file,))?).await?;
            history.into_iter().find(|r| r.id == revision).ok_or("Revision unavailable")?
        };
        self.download_manifest(file, manifest.into(), "drive_revision_chunk").await
    }
    pub async fn public_download(&self, file: ObjectId) -> CloudResult<Vec<u8>> {
        let item: crate::public_link::PublicItem = self.drive_call("public_get", args((file,))?).await?;
        self.download_manifest(file, item.latest.ok_or("File unavailable")?, "public_chunk").await
    }
    async fn download_manifest(&self, file: ObjectId, manifest: crate::public_link::ContentManifest, method: &str) -> CloudResult<Vec<u8>> {
        let revision = manifest.id;
        if manifest.bytes > MAX_DOCUMENT_BYTES || manifest.sha256.len() != 32 || manifest.chunk_count != manifest.bytes.div_ceil(CHUNK_BYTES as u64) {
            return Err("Invalid file manifest".into());
        }
        let mut bytes = Vec::new();
        for index in 0..manifest.chunk_count {
            let response = self.transport.query(method, args((file, revision, index))?).await?;
            let chunk = decode_one::<crate::drive::DriveResult<Vec<u8>>>(&response).map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            let expected = (manifest.bytes - index * CHUNK_BYTES as u64).min(CHUNK_BYTES as u64) as usize;
            if chunk.len() != expected {
                return Err("Invalid file chunk".into());
            }
            bytes.extend(chunk);
        }
        if Sha256::digest(&bytes).to_vec() != manifest.sha256 {
            return Err("File integrity check failed".into());
        }
        Ok(bytes)
    }
    /// A Craft host validates the authenticated drive reference, never personal ownership.
    pub async fn drive_craft_file(&self, space: ObjectId, file: ObjectId, kind: &FileKind) -> CloudResult<crate::drive::DriveEntry> {
        let entry: crate::drive::DriveEntry = self.drive_call("drive_get", args((file,))?).await?;
        validate_craft_entry(&entry, space, kind)?;
        Ok(entry)
    }
    pub async fn drive_save_craft(
        &self,
        target: crate::drive::UploadTarget,
        bytes: &[u8],
        request_id: String,
        kind: &FileKind,
        progress: impl FnMut(usize, usize),
    ) -> CloudResult<crate::drive::DriveUpload> {
        self.drive_save_craft_with_policy(target, bytes, request_id, kind, false, progress).await
    }
    pub async fn drive_save_craft_with_policy(
        &self,
        target: crate::drive::UploadTarget,
        bytes: &[u8],
        request_id: String,
        kind: &FileKind,
        automatic: bool,
        mut progress: impl FnMut(usize, usize),
    ) -> CloudResult<crate::drive::DriveUpload> {
        use crate::drive::*;
        kind.validate()?;
        match &target {
            UploadTarget::New { format, .. } if format.craft.as_ref() == Some(kind) => {}
            UploadTarget::Replace { file, .. } => {
                let entry: DriveEntry = self.drive_call("drive_get", args((*file,))?).await?;
                validate_craft_entry(&entry, entry.object.space, kind)?;
                if !entry.capabilities.edit {
                    return Err("This file is read-only. Use Save As to save a copy.".into());
                }
            }
            _ => return Err("This file format is not supported by this editor".into()),
        }
        // The server rechecks Create/Edit, revocation and expected revision at commit.
        self.drive_save_with_policy(target, bytes, request_id, automatic.then_some(true), |done, total| {
            progress(done, total);
            true
        })
        .await
    }
    pub async fn projects(&self) -> CloudResult<Vec<Project>> {
        self.update("list_projects", args(())?).await
    }
    pub async fn projects_for(&self, kind: &FileKind) -> CloudResult<Vec<Project>> {
        kind.validate()?;
        self.update("list_app_files", args((kind.clone(),))?).await
    }
    pub async fn account(&self) -> CloudResult<AccountContext> {
        self.update("get_or_create_account", args(())?).await
    }
    pub async fn files(&self, space: ObjectId, after: Option<ObjectId>, limit: u32) -> CloudResult<FilePage> {
        self.update("list_files", args((space, after, limit))?).await
    }
    pub async fn file(&self, space: ObjectId, file: ObjectId) -> CloudResult<FileDetails> {
        self.update("get_file", args((space, file))?).await
    }
    pub async fn save(&self, state: &mut UploadState, bytes: &[u8], notify: impl FnMut(TransferEvent)) -> CloudResult<Revision> {
        self.save_inner(state, bytes, None, notify).await
    }
    pub async fn save_typed(&self, state: &mut UploadState, bytes: &[u8], kind: &FileKind, notify: impl FnMut(TransferEvent)) -> CloudResult<Revision> {
        kind.validate()?;
        if let Some(file) = &state.file
            && !self.projects_for(kind).await?.iter().any(|p| p.id == file.id)
        {
            return Err("This cloud file has a different format or is unavailable".into());
        }
        self.save_inner(state, bytes, Some(kind), notify).await
    }
    async fn save_inner(&self, state: &mut UploadState, bytes: &[u8], kind: Option<&FileKind>, mut notify: impl FnMut(TransferEvent)) -> CloudResult<Revision> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err("File size is outside the service limit".into());
        }
        if state.file.is_none() {
            state.file = Some(match kind {
                Some(kind) => self.update("create_file", args((state.name.clone(), kind.clone()))?).await?,
                None => self.update("create_project", args((state.name.clone(),))?).await?,
            });
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
    pub async fn download(&self, file: ObjectId, revision: u64, mut notify: impl FnMut(TransferEvent)) -> CloudResult<(Revision, Vec<u8>)> {
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

fn validate_craft_entry(entry: &crate::drive::DriveEntry, space: ObjectId, kind: &FileKind) -> CloudResult<()> {
    kind.validate()?;
    if entry.object.space != space || !entry.capabilities.read || entry.object.trashed_at.is_some() {
        return Err("This file is not available to the signed-in account".into());
    }
    if !matches!(&entry.object.kind, crate::drive::ObjectKind::File(format) if format.craft.as_ref() == Some(kind)) {
        return Err("This file format is not supported by this editor".into());
    }
    Ok(())
}

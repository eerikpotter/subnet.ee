//! Public policy is independent from named grants. Every read resolves live ancestry.
use super::collaboration::{decoded, encoded, retry};
use super::*;
use crate::permissions::{AccessGraph, Actor};
use sovereign_cloud::{drive::*, public_link::*};
use std::ops::Bound::{Excluded, Included};

#[derive(Serialize, serde::Deserialize)]
struct Bookmark {
    parent: ObjectId,
    after: ObjectId,
    version: u64,
}
impl AccessGraph {
    /// Latest live expiry; MAX means no expiry. Used by both reads and move previews.
    pub(crate) fn public_deadline(&self, id: ObjectId, now: u64, trash: bool) -> DriveResult<u128> {
        let object = self.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        let path = self.ancestry(id, trash)?;
        if self.teams.get(&object.space).is_some_and(|s| !s.external_sharing) {
            return Ok(0);
        }
        Ok(path
            .iter()
            .filter_map(|id| self.public_links.get(id))
            .filter(|p| p.enabled && p.expires_at.is_none_or(|e| e > now))
            .map(|p| p.expires_at.map(u128::from).unwrap_or(u128::MAX))
            .max()
            .unwrap_or(0))
    }
}
impl<M: Memory> Store<M> {
    pub(crate) fn public_link_state(&self, actor: &Actor, id: ObjectId, now: u64) -> DriveResult<PublicLinkState> {
        let graph = self.indexed_graph(id)?;
        graph.require(actor, id, Capabilities::READ, now, false)?;
        graph.owner(actor, id)?;
        let object = graph.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        Ok(PublicLinkState {
            direct: graph.public_links.get(&id).cloned().unwrap_or_default(),
            inherited: object.parent.is_some_and(|p| graph.public_deadline(p, now, false).unwrap_or(0) > 0),
            effective: graph.public_deadline(id, now, false)? > 0,
            allowed: graph.teams.get(&object.space).is_none_or(|s| s.external_sharing),
        })
    }
    pub(crate) fn public_link_set(&mut self, actor: &Actor, request: SetPublicLink, now: u64) -> DriveResult<PublicLinkState> {
        let state = self.public_link_state(actor, request.object, now)?;
        let (key, digest) = retry(actor, "public_link", &request.request_id, &request)?;
        if let Some(bytes) = self.drive_retries.get(&key) {
            let prior: String = decoded(&bytes)?;
            return if prior == digest { Ok(state) } else { Err(DriveError::Conflict) };
        }
        if state.direct.version != request.expected_version {
            return Err(DriveError::Conflict);
        }
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        let graph = self.indexed_graph(request.object)?;
        if matches!(graph.objects[&request.object].kind, ObjectKind::Shortcut(_)) {
            return Err(DriveError::InvalidInput("Share the original item, not its shortcut".into()));
        }
        if request.enabled && (!state.allowed || request.expires_at.is_some_and(|e| e <= now)) {
            return Err(DriveError::InvalidInput("Public sharing is disabled or expiry is in the past".into()));
        }
        let policy =
            PublicLink { enabled: request.enabled, expires_at: request.expires_at, version: state.direct.version.checked_add(1).ok_or(DriveError::Internal)? };
        let version = self.acl_versions.get(&request.object).unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        let space = graph.objects[&request.object].space;
        let space_version = self.space_versions.get(&space).unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        let bytes = encoded(&policy)?;
        let receipt = encoded(&digest)?;
        self.public_links.insert(request.object, bytes);
        self.acl_versions.insert(request.object, version);
        self.space_versions.insert(space, space_version);
        self.drive_retries.insert(key, receipt);
        self.public_link_state(actor, request.object, now)
    }
    pub(crate) fn public_get(&self, id: ObjectId, now: u64) -> DriveResult<PublicItem> {
        let graph = self.indexed_graph(id)?;
        if graph.public_deadline(id, now, false)? == 0 {
            return Err(DriveError::Inaccessible);
        }
        let object = graph.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        let latest = match object.kind {
            ObjectKind::Shortcut(_) => return Err(DriveError::Inaccessible),
            ObjectKind::Folder => None,
            ObjectKind::File(_) => {
                let project: Project = decoded(&self.projects.get(&id).ok_or(DriveError::Inaccessible)?)?;
                Some(project.revisions.last().ok_or(DriveError::Inaccessible)?.clone().into())
            }
        };
        Ok(PublicItem { id, name: object.name.clone(), kind: object.kind.clone(), updated_at: object.updated_at, latest })
    }
    pub(crate) fn public_list(&self, parent: ObjectId, cursor: Option<PageCursor>, limit: u32, now: u64) -> DriveResult<PublicPage> {
        let item = self.public_get(parent, now)?;
        if item.kind != ObjectKind::Folder {
            return Err(DriveError::Inaccessible);
        }
        // The same scoped invalidation as authenticated folder pages, with no account authority.
        let actor = Actor { subject: Participant::User(Principal::anonymous()), sponsor: Principal::anonymous(), credential: None };
        let version = self.scope_version(&actor, Some(parent))?;
        let after = if let Some(cursor) = cursor {
            let b: Bookmark = self.open_bookmark(&cursor, b"public-folder", Principal::anonymous())?;
            if b.parent != parent || b.version != version {
                return Err(DriveError::Conflict);
            }
            b.after
        } else {
            ObjectId::MIN
        };
        let limit = limit.clamp(1, 50) as usize;
        let mut entries = Vec::new();
        let mut last = None;
        let mut source = self.read_children.range((Excluded((parent, after)), Included((parent, ObjectId::MAX)))).peekable();
        for _ in 0..128 {
            let Some(entry) = source.next() else { break };
            let id = entry.key().1;
            last = Some(id);
            match self.public_get(id, now) {
                Ok(item) => entries.push(item),
                Err(DriveError::Inaccessible) => {}
                Err(e) => return Err(e),
            };
            if entries.len() == limit {
                break;
            }
        }
        let next = if source.peek().is_some() {
            last.map(|after| self.seal_bookmark(&Bookmark { parent, after, version }, b"public-folder", Principal::anonymous())).transpose()?
        } else {
            None
        };
        Ok(PublicPage { entries, next })
    }
    pub(crate) fn public_chunk(&self, id: ObjectId, revision: u64, index: u64, now: u64) -> DriveResult<Vec<u8>> {
        let head = self.public_get(id, now)?.latest.ok_or(DriveError::Inaccessible)?;
        if head.id != revision {
            return Err(DriveError::Conflict);
        }
        if index >= head.chunk_count {
            return Err(DriveError::InvalidInput("Invalid chunk index".into()));
        }
        self.chunks.get(&(revision, index)).ok_or(DriveError::Internal)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use ic_stable_structures::VectorMemory;
    fn actor(n: u8) -> Actor {
        Actor::user(Principal::self_authenticating([n; 32])).unwrap()
    }
    fn policy(s: &mut Store<VectorMemory>, a: &Actor, id: ObjectId, enabled: bool, expiry: Option<u64>, now: u64) {
        let version = s.public_link_state(a, id, now).unwrap().direct.version;
        s.public_link_set(
            a,
            SetPublicLink { object: id, enabled, expires_at: expiry, expected_version: version, request_id: format!("public-policy-{id}-{version}") },
            now,
        )
        .unwrap();
    }
    #[test]
    fn public_folder_is_bounded_revocable_and_has_no_private_projection_or_old_content() {
        let memory = VectorMemory::default();
        let mut s = Store::new(memory.clone());
        let a = actor(1);
        let b = actor(2);
        let root = s.drive_initialize(a.sponsor, 1).unwrap().object.id;
        let folder = s.drive_create_folder(&a, CreateFolder { parent: root, name: "Public".into(), request_id: "public-folder-create".into() }, 2).unwrap();
        let mut files = Vec::new();
        for n in 0..3 {
            files.push(
                s.drive_write_inline(
                    &a,
                    InlineFile {
                        parent: folder.object.id,
                        name: format!("{n}.txt"),
                        format: FileFormat { media_type: "text/plain".into(), craft: None },
                        bytes: b"hello world".to_vec(),
                        request_id: format!("public-file-create-{n}"),
                    },
                    3,
                )
                .unwrap(),
            );
        }
        assert!(s.public_get(files[0].object.id, 4).is_err());
        assert!(
            s.public_link_set(
                &b,
                SetPublicLink { object: folder.object.id, enabled: true, expires_at: None, expected_version: 0, request_id: "public-unauthorized".into() },
                4
            )
            .is_err()
        );
        policy(&mut s, &a, folder.object.id, true, Some(100), 4);
        assert!(s.public_get(root, 5).is_err());
        let p = s.public_list(folder.object.id, None, 1, 5).unwrap();
        assert_eq!(p.entries.len(), 1);
        assert!(p.next.is_some());
        let next = s.public_list(folder.object.id, p.next.clone(), 1, 5).unwrap();
        assert_ne!(p.entries[0].id, next.entries[0].id);
        let json = serde_json::to_value(&p.entries[0]).unwrap();
        assert!(json.get("owner").is_none());
        assert!(json.get("parent").is_none());
        assert!(json["latest"].get("author").is_none());
        let f = &files[0];
        let r = f.latest.as_ref().unwrap();
        assert_eq!(s.public_chunk(f.object.id, r.id, 0, 5).unwrap(), b"hello world");
        assert!(s.public_chunk(f.object.id, r.id + 1, 0, 5).is_err());
        assert!(s.drive_get(&b, f.object.id, 5).is_err()); // Public URLs do not create named grants.
        assert!(s.drive_history(&b, f.object.id, 5).is_err());
        assert!(s.public_get(f.object.id, 100).is_err());
        policy(&mut s, &a, folder.object.id, false, None, 6);
        assert!(s.public_chunk(f.object.id, r.id, 0, 7).is_err());
        assert!(s.public_list(folder.object.id, p.next, 1, 7).is_err());
        policy(&mut s, &a, f.object.id, true, None, 8);
        assert!(s.public_get(folder.object.id, 8).is_err());
        assert!(s.public_get(f.object.id, 8).is_ok());
        let mut s = Store::new(memory);
        assert!(s.public_get(f.object.id, 9).is_ok());
        s.drive_organize(
            &a,
            OrganizeRequest {
                object: f.object.id,
                expected_version: f.object.version,
                request_id: "public-trash-file".into(),
                change: OrganizationChange::Trash,
            },
            10,
        )
        .unwrap();
        assert!(s.public_get(f.object.id, 11).is_err());
    }
    #[test]
    fn team_policy_and_public_move_audience_share_the_live_authority_model() {
        use sovereign_cloud::spaces::*;
        let mut s = Store::new(VectorMemory::default());
        let a = actor(1);
        let b = actor(2);
        s.drive_initialize(a.sponsor, 1).unwrap();
        s.drive_initialize(b.sponsor, 1).unwrap();
        let space = s.space_create(&a, CreateSpace { name: "Team".into(), request_id: "public-team-create".into() }, 2).unwrap();
        let root = space.space.id;
        let folder = s.drive_create_folder(&a, CreateFolder { parent: root, name: "Public".into(), request_id: "public-team-folder".into() }, 3).unwrap();
        let private = s.drive_create_folder(&a, CreateFolder { parent: root, name: "Private".into(), request_id: "private-team-folder".into() }, 3).unwrap();
        policy(&mut s, &a, folder.object.id, true, None, 4);
        let impact = s.drive_move_preview(&a, private.object.id, folder.object.id, 5).unwrap();
        assert!(impact.changes.iter().any(|c| c.subject == Participant::User(Principal::anonymous()) && c.gains));
        s.space_change(
            &a,
            ChangeSpace {
                space: root,
                name: "Team".into(),
                external_sharing: false,
                expected_version: space.space.version,
                request_id: "public-team-policy".into(),
            },
            6,
        )
        .unwrap();
        assert!(s.public_get(folder.object.id, 7).is_err());
        assert!(!s.public_link_state(&a, folder.object.id, 7).unwrap().allowed);
    }
}

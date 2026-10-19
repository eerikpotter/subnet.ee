//! Team membership and accounting. Personal accounts keep their existing storage
//! representation; all Drive operations select accounting by logical space.
use super::collaboration::{decoded, encoded, name, retry};
use super::*;
use crate::permissions::{AccessGraph, Actor};
use sovereign_cloud::{drive::*, spaces::*};
use std::ops::Bound::{Excluded, Included};

pub(super) const MAX_MEMBERS: usize = 128;
const MAX_SPACES: usize = 32;
pub(super) struct SpaceStore<M: Memory> {
    records: StableBTreeMap<ObjectId, Vec<u8>, Vm<M>>,
    members: StableBTreeMap<(ObjectId, Principal), Vec<u8>, Vm<M>>,
    users: StableBTreeMap<(Principal, ObjectId), u8, Vm<M>>,
    usage: StableBTreeMap<ObjectId, u64, Vm<M>>,
    pub(super) counts: StableBTreeMap<ObjectId, (u64, u64), Vm<M>>,
    pub(super) pending: StableBTreeMap<(ObjectId, u64), u8, Vm<M>>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use ic_stable_structures::VectorMemory;
    fn actor(n: u8) -> Actor {
        Actor::user(Principal::self_authenticating([n; 32])).unwrap()
    }
    fn member(s: &mut Store<VectorMemory>, manager: &Actor, space: ObjectId, who: &Actor, role: Option<SpaceRole>) {
        let version = s.spaces.get(space).unwrap().version;
        s.space_change_member(
            manager,
            ChangeMember { space, principal: who.sponsor, role, expected_version: version, request_id: format!("member-change-{version}-{}", who.sponsor) },
        )
        .unwrap();
    }
    fn file(space: ObjectId) -> InlineFile {
        InlineFile {
            parent: space,
            name: "Team notes.txt".into(),
            request_id: "team-notes-create-001".into(),
            bytes: b"independent team data".to_vec(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
        }
    }
    #[test]
    fn team_survives_founder_departure_with_separate_quota_no_personal_api_escape_and_upgrade() {
        let memory = VectorMemory::default();
        let mut s = Store::new(memory.clone());
        let (founder, manager, viewer, outsider) = (actor(1), actor(2), actor(3), actor(4));
        for a in [&founder, &manager, &viewer, &outsider] {
            s.drive_initialize(a.sponsor, 1).unwrap();
        }
        let request = CreateSpace { name: "Project team".into(), request_id: "team-creation-00001".into() };
        let space = s.space_create(&founder, request.clone(), 2).unwrap().space.id;
        assert_eq!(s.space_create(&founder, request, 3).unwrap().space.id, space);
        assert!(
            s.space_change_member(
                &founder,
                ChangeMember { space, principal: founder.sponsor, role: None, expected_version: 1, request_id: "remove-last-manager".into() }
            )
            .is_err()
        );
        member(&mut s, &founder, space, &manager, Some(SpaceRole::Manager));
        member(&mut s, &manager, space, &viewer, Some(SpaceRole::Viewer));
        assert!(s.drive_write_inline(&viewer, file(space), 4).is_err());
        let saved = s.drive_write_inline(&manager, file(space), 4).unwrap();
        let id = saved.object.id;
        let bytes = saved.latest.as_ref().unwrap().bytes;
        assert_eq!(s.account(founder.sponsor, 5).unwrap().used_bytes, 0);
        assert_eq!(s.account(manager.sponsor, 5).unwrap().used_bytes, 0);
        assert_eq!(s.space_get(&viewer, space).unwrap().used_bytes, bytes);
        assert!(s.project(founder.sponsor, id).is_err());
        assert!(s.list(founder.sponsor).unwrap().is_empty());
        assert!(s.drive_get(&outsider, id, 5).is_err());
        assert!(s.space_members(&outsider, space, None, 50).is_err());
        assert!(!s.drive_get(&viewer, id, 5).unwrap().space_access.unwrap().manage);
        member(&mut s, &manager, space, &founder, None);
        assert!(s.drive_get(&founder, id, 6).is_err());
        assert!(s.drive_grants(&founder, id).is_err());
        assert!(s.space_list(&founder).unwrap().is_empty());
        assert!(s.drive_get(&manager, id, 6).unwrap().space_access.unwrap().manage);
        drop(s);
        let mut s = Store::new(memory);
        assert_eq!(s.drive_current_chunk(&viewer, id, saved.latest.unwrap().id, 0, 7).unwrap(), b"independent team data");
        assert!(s.drive_get(&founder, id, 7).is_err());
        let version = s
            .drive_organize(
                &manager,
                OrganizeRequest { object: id, expected_version: 1, request_id: "team-trash-file-001".into(), change: OrganizationChange::Trash },
                8,
            )
            .unwrap()
            .version;
        let receipt = s
            .drive_purge_trash(&manager, PurgeRequest { items: vec![MutationReceipt { object: id, version }], request_id: "team-purge-file-001".into() })
            .unwrap();
        assert_eq!(receipt.freed_bytes, bytes);
        assert_eq!(s.space_get(&manager, space).unwrap().used_bytes, 0);
    }

    #[test]
    fn member_revocation_cancels_pending_authority_and_external_policy_revalidates_every_read() {
        let mut s = Store::new(VectorMemory::default());
        let (admin, editor, guest) = (actor(1), actor(2), actor(3));
        for a in [&admin, &editor, &guest] {
            s.drive_initialize(a.sponsor, 1).unwrap();
        }
        let team = s.space_create(&admin, CreateSpace { name: "Budget".into(), request_id: "budget-team-creation".into() }, 2).unwrap().space.id;
        member(&mut s, &admin, team, &editor, Some(SpaceRole::Editor));
        let content = b"budget numbers";
        let upload = s
            .drive_begin(
                &editor,
                DriveUploadRequest {
                    target: UploadTarget::New { parent: team, name: "Budget.txt".into(), format: FileFormat { media_type: "text/plain".into(), craft: None } },
                    bytes: content.len() as u64,
                    sha256: Sha256::digest(content).to_vec(),
                    request_id: "budget-upload-request".into(),
                    automatic: None,
                },
                3,
            )
            .unwrap();
        s.drive_put(&editor, upload.id, 0, content.to_vec(), 4).unwrap();
        member(&mut s, &admin, team, &editor, None);
        assert!(s.drive_commit(&editor, upload.id, 5).is_err());
        assert_eq!(s.drive_pending(&admin).unwrap().len(), 1);
        s.drive_abort(&editor, upload.id).unwrap();
        assert_eq!(s.space_get(&admin, team).unwrap().used_bytes, 0);
        let saved = s.drive_write_inline(&admin, file(team), 6).unwrap();
        s.drive_grant(
            &admin,
            GrantRequest {
                request_id: "budget-guest-grant".into(),
                id: None,
                expected_version: None,
                root: saved.object.id,
                subject: guest.subject.clone(),
                capabilities: Capabilities::READ,
                expires_at: None,
                active: true,
            },
            7,
        )
        .unwrap();
        assert!(s.drive_get(&guest, saved.object.id, 8).is_ok());
        let version = s.spaces.get(team).unwrap().version;
        s.space_change(
            &admin,
            ChangeSpace {
                space: team,
                name: "Budget".into(),
                external_sharing: false,
                expected_version: version,
                request_id: "budget-disable-external".into(),
            },
            9,
        )
        .unwrap();
        assert!(s.drive_get(&guest, saved.object.id, 10).is_err());
        assert!(s.drive_current_chunk(&guest, saved.object.id, saved.latest.unwrap().id, 0, 10).is_err());
        assert!(s.drive_get(&admin, saved.object.id, 10).is_ok());
    }
}
impl<M: Memory> SpaceStore<M> {
    pub(super) fn new(m: &MemoryManager<M>) -> Self {
        Self {
            records: StableBTreeMap::init(m.get(MemoryId::new(62))),
            members: StableBTreeMap::init(m.get(MemoryId::new(63))),
            users: StableBTreeMap::init(m.get(MemoryId::new(64))),
            usage: StableBTreeMap::init(m.get(MemoryId::new(65))),
            counts: StableBTreeMap::init(m.get(MemoryId::new(66))),
            pending: StableBTreeMap::init(m.get(MemoryId::new(67))),
        }
    }
    pub(super) fn is_team(&self, id: ObjectId) -> bool {
        self.records.contains_key(&id)
    }
    pub(super) fn get(&self, id: ObjectId) -> DriveResult<TeamSpace> {
        decoded(&self.records.get(&id).ok_or(DriveError::Inaccessible)?)
    }
    pub(super) fn role(&self, space: ObjectId, user: Principal) -> DriveResult<Option<SpaceRole>> {
        self.members.get(&(space, user)).map(|b| decoded(&b)).transpose()
    }
    pub(super) fn ids(&self, user: Principal) -> Vec<ObjectId> {
        self.users.range((user, ObjectId::MIN)..=(user, ObjectId::MAX)).take(MAX_SPACES + 1).map(|e| e.key().1).collect()
    }
    pub(super) fn load(&self, graph: &mut AccessGraph, id: ObjectId) -> DriveResult<()> {
        if graph.teams.contains_key(&id) || !self.is_team(id) {
            return Ok(());
        }
        graph.teams.insert(id, self.get(id)?);
        for (i, e) in self.members.range((id, Principal::management_canister())..=(id, Principal::from_slice(&[255; 29]))).enumerate() {
            if i >= MAX_MEMBERS {
                return Err(DriveError::Internal);
            }
            graph.members.insert(*e.key(), decoded(&e.value())?);
        }
        Ok(())
    }
}
impl<M: Memory> Store<M> {
    pub(super) fn space_request_manager(&self, space: ObjectId, owner: Principal) -> DriveResult<Principal> {
        if !self.spaces.is_team(space) {
            return Ok(owner);
        }
        for e in self.spaces.members.range((space, Principal::management_canister())..=(space, Principal::from_slice(&[255; 29]))).take(MAX_MEMBERS) {
            if decoded::<SpaceRole>(&e.value())? == SpaceRole::Manager {
                return Ok(e.key().1);
            }
        }
        Err(DriveError::Internal)
    }
    pub(super) fn manages_space(&self, actor: &Actor, space: ObjectId, personal_owner: Principal) -> bool {
        if actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() {
            return false;
        }
        if self.spaces.is_team(space) {
            self.spaces.role(space, actor.sponsor).is_ok_and(|r| r == Some(SpaceRole::Manager))
        } else {
            actor.sponsor == personal_owner
        }
    }
    pub(super) fn pending_in_space(&self, space: ObjectId, owner: Principal) -> Vec<u64> {
        if self.spaces.is_team(space) {
            self.spaces.pending.range((space, 0)..=(space, u64::MAX)).take(21).map(|e| e.key().1).collect()
        } else {
            self.pending_owners.range((owner, 0)..=(owner, u64::MAX)).take(21).map(|e| e.key().1).collect()
        }
    }
    pub(super) fn add_pending_space(&mut self, space: ObjectId, owner: Principal, id: u64) {
        if self.spaces.is_team(space) {
            self.spaces.pending.insert((space, id), 0);
        } else {
            self.pending_owners.insert((owner, id), 0);
        }
    }
    pub(super) fn remove_pending_space(&mut self, space: ObjectId, owner: Principal, id: u64) {
        if self.spaces.is_team(space) {
            self.spaces.pending.remove(&(space, id));
        } else {
            self.pending_owners.remove(&(owner, id));
        }
    }
    pub(super) fn space_usage(&self, space: ObjectId, personal_owner: Principal) -> u64 {
        if self.spaces.is_team(space) { self.spaces.usage.get(&space).unwrap_or(0) } else { self.usage.get(&personal_owner).unwrap_or(0) }
    }
    pub(super) fn set_space_usage(&mut self, space: ObjectId, personal_owner: Principal, value: u64) {
        if self.spaces.is_team(space) {
            self.spaces.usage.insert(space, value);
        } else {
            self.usage.insert(personal_owner, value);
        }
    }
    pub(super) fn personal_project_index(&mut self, owner: Principal, space: ObjectId, file: ObjectId) {
        if !self.spaces.is_team(space) {
            self.project_owners.insert((owner, file), 0);
        }
    }
    pub(super) fn space_file_count(&self, space: ObjectId, owner: Principal) -> DriveResult<usize> {
        if self.spaces.is_team(space) {
            Ok(self.spaces.counts.get(&space).unwrap_or_default().1 as usize)
        } else {
            self.owned_project_count(owner).map_err(|_| DriveError::Internal)
        }
    }
    pub(crate) fn space_list(&self, actor: &Actor) -> DriveResult<Vec<SpaceView>> {
        self.spaces.ids(actor.sponsor).into_iter().map(|id| self.space_get(actor, id)).collect()
    }
    pub(crate) fn space_get(&self, actor: &Actor, id: ObjectId) -> DriveResult<SpaceView> {
        let role = self.spaces.role(id, actor.sponsor)?.ok_or(DriveError::Inaccessible)?;
        Ok(SpaceView { space: self.spaces.get(id)?, role, used_bytes: self.spaces.usage.get(&id).unwrap_or(0), quota_bytes: USER_QUOTA_BYTES })
    }
    pub(crate) fn space_create(&mut self, actor: &Actor, request: CreateSpace, now: u64) -> DriveResult<SpaceView> {
        let (key, hash) = retry(actor, "space-create", &request.request_id, &request)?;
        if let Some(b) = self.drive_retries.get(&key) {
            let (saved, id): (String, ObjectId) = decoded(&b)?;
            if saved != hash {
                return Err(DriveError::Conflict);
            }
            return self.space_get(actor, id);
        }
        if self.spaces.ids(actor.sponsor).len() >= MAX_SPACES || self.spaces.records.len() >= 10_000 || self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Team space limit reached".into()));
        }
        if self.accounts.get(&actor.sponsor).is_none() {
            return Err(DriveError::Inaccessible);
        }
        let id = self.next_object_id().map_err(|_| DriveError::Internal)?;
        let space = TeamSpace { id, name: name(&request.name)?, created_by: actor.sponsor, created_at: now, version: 1, external_sharing: true };
        let mut graph = self.read_graph();

        graph.teams.insert(id, space.clone());
        graph.members.insert((id, actor.sponsor), SpaceRole::Manager);
        graph.objects.insert(
            id,
            DriveObject {
                id,
                space: id,
                owner: actor.sponsor,
                parent: None,
                name: space.name.clone(),
                kind: ObjectKind::Folder,
                version: 1,
                created_at: now,
                updated_at: now,
                trashed_at: None,
            },
        );
        let changes = self.prepare_graph(&graph)?;
        let bytes = encoded(&space)?;
        let role = encoded(&SpaceRole::Manager)?;
        let receipt = encoded(&(hash, id))?;
        self.spaces.records.insert(id, bytes);
        self.spaces.members.insert((id, actor.sponsor), role);
        self.spaces.users.insert((actor.sponsor, id), 0);
        self.apply_graph(changes);
        self.drive_retries.insert(key, receipt);
        self.space_get(actor, id)
    }
    pub(crate) fn space_members(&self, actor: &Actor, id: ObjectId, after: Option<Principal>, limit: u32) -> DriveResult<SpaceMembers> {
        self.space_get(actor, id)?;
        let start = after.map(|p| Excluded((id, p))).unwrap_or(Included((id, Principal::management_canister())));
        let mut iter = self.spaces.members.range((start, Included((id, Principal::from_slice(&[255; 29]))))).peekable();
        let mut members = Vec::new();
        for _ in 0..limit.clamp(1, 50) {
            let Some(e) = iter.next() else { break };
            members.push(SpaceMember { principal: e.key().1, role: decoded(&e.value())? });
        }
        let next = if iter.peek().is_some() { members.last().map(|m| m.principal) } else { None };
        Ok(SpaceMembers { space: self.spaces.get(id)?, members, next })
    }
    pub(crate) fn space_change_member(&mut self, actor: &Actor, request: ChangeMember) -> DriveResult<TeamSpace> {
        let mut graph = self.graph_for_object(request.space)?;
        if request.principal == actor.sponsor && request.role.is_none() {
            self.space_get(actor, request.space)?;
        } else {
            graph.owner(actor, request.space)?;
        }
        let mut space = self.spaces.get(request.space)?;
        let (key, hash) = retry(actor, "space-member", &request.request_id, &request)?;
        if let Some(b) = self.drive_retries.get(&key) {
            let (saved, value): (String, TeamSpace) = decoded(&b)?;
            return if saved == hash { Ok(value) } else { Err(DriveError::Conflict) };
        }
        if space.version != request.expected_version {
            return Err(DriveError::Conflict);
        }
        if request.principal == Principal::anonymous() || (request.role.is_some() && !self.accounts.contains_key(&request.principal)) {
            return Err(DriveError::InvalidInput("Choose an existing signed-in account".into()));
        }
        if request.role.is_some()
            && !graph.members.contains_key(&(space.id, request.principal))
            && (graph.members.len() >= MAX_MEMBERS || self.spaces.ids(request.principal).len() >= MAX_SPACES)
        {
            return Err(DriveError::Limit("Team membership limit reached".into()));
        }
        if let Some(role) = request.role {
            graph.members.insert((space.id, request.principal), role);
        } else {
            graph.members.remove(&(space.id, request.principal));
        }
        if !graph.members.iter().any(|((s, _), r)| *s == space.id && *r == SpaceRole::Manager) {
            return Err(DriveError::InvalidInput("Keep at least one team manager".into()));
        }
        space.version = space.version.checked_add(1).ok_or(DriveError::Internal)?;
        let bytes = encoded(&space)?;
        let member = request.role.map(|r| encoded(&r)).transpose()?;
        let receipt = encoded(&(hash, &space))?;
        let discovery = self.discovery_versions.get(&request.principal).unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        let acl = self.acl_versions.get(&space.id).unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        self.spaces.records.insert(space.id, bytes);
        if let Some(bytes) = member {
            self.spaces.members.insert((space.id, request.principal), bytes);
            self.spaces.users.insert((request.principal, space.id), 0);
        } else {
            self.spaces.members.remove(&(space.id, request.principal));
            self.spaces.users.remove(&(request.principal, space.id));
        }
        self.discovery_versions.insert(request.principal, discovery);
        self.acl_versions.insert(space.id, acl);
        self.drive_retries.insert(key, receipt);
        Ok(space)
    }
    pub(crate) fn space_change(&mut self, actor: &Actor, request: ChangeSpace, now: u64) -> DriveResult<TeamSpace> {
        let mut graph = self.graph_for_object(request.space)?;
        graph.owner(actor, request.space)?;
        let mut space = self.spaces.get(request.space)?;
        let (key, hash) = retry(actor, "space-settings", &request.request_id, &request)?;
        if let Some(b) = self.drive_retries.get(&key) {
            let (saved, value): (String, TeamSpace) = decoded(&b)?;
            return if saved == hash { Ok(value) } else { Err(DriveError::Conflict) };
        }
        if space.version != request.expected_version {
            return Err(DriveError::Conflict);
        }
        space.name = name(&request.name)?;
        space.external_sharing = request.external_sharing;
        space.version = space.version.checked_add(1).ok_or(DriveError::Internal)?;
        let root = graph.objects.get_mut(&space.id).ok_or(DriveError::Internal)?;
        root.name = space.name.clone();
        root.updated_at = now;
        root.version = root.version.checked_add(1).ok_or(DriveError::Internal)?;
        let changes = self.prepare_graph(&graph)?;
        let bytes = encoded(&space)?;
        let receipt = encoded(&(hash, &space))?;
        if self.drive_retries.len() >= 20_000 {
            return Err(DriveError::Limit("Request receipt limit reached".into()));
        }
        self.spaces.records.insert(space.id, bytes);
        self.apply_graph(changes);
        self.drive_retries.insert(key, receipt);
        Ok(space)
    }
}

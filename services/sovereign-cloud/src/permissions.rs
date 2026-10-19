//! One evaluator for signed account calls. Restricted connection support is deferred.
use candid::Principal;
use serde::{Deserialize, Serialize};
use sovereign_cloud::ObjectId;
use sovereign_cloud::drive::*;
use sovereign_cloud::spaces::{SpaceAccess, SpaceRole, TeamSpace};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_DEPTH: usize = 64;
pub const MAX_GRANTS: usize = 20_000;

/// Deliberately not deserializable or part of the public Candid interface.
#[derive(Clone, Debug)]
pub(crate) struct Actor {
    pub(crate) subject: Participant,
    pub(crate) sponsor: Principal,
    pub(crate) credential: Option<String>,
}
impl Actor {
    pub(crate) fn user(caller: Principal) -> DriveResult<Self> {
        if caller == Principal::anonymous() {
            return Err(DriveError::Inaccessible);
        }
        Ok(Self { subject: Participant::User(caller), sponsor: caller, credential: None })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct AccessGraph {
    #[serde(skip)]
    pub(crate) public_links: BTreeMap<ObjectId, sovereign_cloud::public_link::PublicLink>,
    #[serde(skip)]
    pub(crate) teams: BTreeMap<ObjectId, TeamSpace>,
    #[serde(skip)]
    pub(crate) members: BTreeMap<(ObjectId, Principal), SpaceRole>,
    #[serde(skip)]
    pub(crate) original_objects: BTreeMap<ObjectId, DriveObject>,
    #[serde(skip)]
    pub(crate) original_grants: BTreeMap<u64, AccessGrant>,
    #[serde(skip)]
    pub(crate) blocked: BTreeSet<(Principal, Principal)>,
    pub(crate) objects: BTreeMap<ObjectId, DriveObject>,
    pub(crate) grants: BTreeMap<u64, AccessGrant>,
    pub(crate) agents: BTreeMap<u64, AgentConnection>,
    pub(crate) tree_version: u64,
}
impl AccessGraph {
    /// Called only after transport credential verification; credentials are not actor IDs.
    #[cfg(test)]
    pub(crate) fn agent_actor(&self, id: u64, credential: String) -> DriveResult<Actor> {
        let agent = self.agents.get(&id).filter(|a| a.active).ok_or(DriveError::Inaccessible)?;
        Ok(Actor { subject: Participant::Agent(id), sponsor: agent.sponsor, credential: Some(credential) })
    }
    pub(crate) fn ancestry(&self, id: ObjectId, include_trash: bool) -> DriveResult<Vec<ObjectId>> {
        let first = self.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = Some(id);
        while let Some(current) = next {
            if result.len() == MAX_DEPTH || !seen.insert(current) {
                return Err(DriveError::Inaccessible);
            }
            let object = self.objects.get(&current).ok_or(DriveError::Inaccessible)?;
            if object.space != first.space || object.owner != first.owner || (!include_trash && object.trashed_at.is_some()) {
                return Err(DriveError::Inaccessible);
            }
            if current != id && object.kind != ObjectKind::Folder {
                return Err(DriveError::Inaccessible);
            }
            result.push(current);
            next = object.parent;
        }
        if result.last() != Some(&first.space) {
            return Err(DriveError::Inaccessible);
        }
        Ok(result)
    }
    fn granted(&self, subject: &Participant, path: &[ObjectId], now: u64) -> Capabilities {
        self.grants
            .values()
            .filter(|g| g.active && &g.subject == subject && path.contains(&g.root) && g.expires_at.is_none_or(|expiry| now < expiry))
            .fold(Capabilities::NONE, |rights, grant| rights.union(grant.capabilities))
    }
    pub(crate) fn effective(&self, actor: &Actor, id: ObjectId, now: u64, include_trash: bool) -> DriveResult<Capabilities> {
        let path = self.ancestry(id, include_trash)?;
        let object = self.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        let team = self.teams.get(&object.space);
        let membership = self.members.get(&(object.space, actor.sponsor));
        if team.is_none() && object.owner != actor.sponsor && self.blocked.contains(&(object.owner, actor.sponsor)) {
            return Err(DriveError::Inaccessible);
        }
        let human = match team {
            Some(team) => {
                if membership.is_none() && !team.external_sharing {
                    return Err(DriveError::Inaccessible);
                }
                membership.map_or(Capabilities::NONE, |r| r.capabilities()).union(self.granted(&Participant::User(actor.sponsor), &path, now))
            }
            None if object.owner == actor.sponsor => Capabilities::ALL,
            None => self.granted(&Participant::User(actor.sponsor), &path, now),
        };
        let mut rights = match &actor.subject {
            Participant::User(principal) if *principal == actor.sponsor && actor.credential.is_none() => human,
            Participant::Agent(agent) => {
                self.agents.get(agent).filter(|a| a.active && a.sponsor == actor.sponsor).ok_or(DriveError::Inaccessible)?;
                if actor.credential.is_none() {
                    return Err(DriveError::Inaccessible);
                }
                human.intersect(self.granted(&actor.subject, &path, now))
            }
            _ => return Err(DriveError::Inaccessible),
        };
        if object.kind != ObjectKind::Folder {
            rights.create = false;
        }
        Ok(rights.intersect(Capabilities::ALL))
    }
    pub(crate) fn require(&self, actor: &Actor, id: ObjectId, required: Capabilities, now: u64, include_trash: bool) -> DriveResult<()> {
        if !required.valid() || !self.effective(actor, id, now, include_trash)?.includes(required.union(Capabilities::READ)) {
            return Err(DriveError::Inaccessible);
        }
        Ok(())
    }
    /// Derived presentation metadata, never an authorization decision. Recipients
    /// only learn about their own grants; owners can see any human sharing.
    pub(crate) fn sharing(&self, actor: &Actor, id: ObjectId, now: u64) -> DriveResult<SharingStatus> {
        self.require(actor, id, Capabilities::READ, now, false)?;
        let object = self.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        let path = self.ancestry(id, false)?;
        let mut status = SharingStatus::default();
        for grant in self.grants.values() {
            let Participant::User(recipient) = grant.subject else { continue };
            if recipient == object.owner
                || (self.owner(actor, id).is_err() && recipient != actor.sponsor)
                || !grant.active
                || !grant.capabilities.read
                || grant.expires_at.is_some_and(|expiry| expiry <= now)
                || !path.contains(&grant.root)
            {
                continue;
            }
            if grant.root == id {
                status.direct = true;
            } else {
                status.inherited = true;
            }
        }
        Ok(status)
    }
    pub(crate) fn owner(&self, actor: &Actor, id: ObjectId) -> DriveResult<()> {
        let object = self.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        let authorized = if self.teams.contains_key(&object.space) {
            self.members.get(&(object.space, actor.sponsor)) == Some(&SpaceRole::Manager)
        } else {
            object.owner == actor.sponsor
        };
        if !authorized || actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() {
            return Err(DriveError::Inaccessible);
        }
        Ok(())
    }
    pub(crate) fn space_access(&self, actor: &Actor, id: ObjectId) -> Option<SpaceAccess> {
        let object = self.objects.get(&id)?;
        Some(SpaceAccess { team_name: self.teams.get(&object.space).map(|s| s.name.clone()), manage: self.owner(actor, id).is_ok() })
    }
    pub(crate) fn set_grant(&mut self, actor: &Actor, grant: AccessGrant, expected: Option<u64>, now: u64) -> DriveResult<AccessGrant> {
        let object = self.objects.get(&grant.root).ok_or(DriveError::Inaccessible)?;
        match grant.subject {
            Participant::User(principal) => {
                self.owner(actor, grant.root)?;
                if principal == Principal::anonymous() {
                    return Err(DriveError::InvalidInput("Choose a signed-in recipient".into()));
                }
            }
            Participant::Agent(id) => {
                let agent = self.agents.get(&id).filter(|a| a.active).ok_or(DriveError::Inaccessible)?;
                if actor.subject != Participant::User(agent.sponsor) || actor.credential.is_some() {
                    return Err(DriveError::Inaccessible);
                }
                if grant.active && !self.effective(actor, grant.root, now, false)?.includes(grant.capabilities) {
                    return Err(DriveError::Inaccessible);
                }
            }
        }
        if !grant.capabilities.valid()
            || (object.kind != ObjectKind::Folder && grant.capabilities.create)
            || (grant.active && grant.expires_at.is_some_and(|expiry| expiry <= now))
        {
            return Err(DriveError::InvalidInput("Invalid capabilities or expiry".into()));
        }
        let old = self.grants.get(&grant.id);
        if old.map(|g| g.version) != expected {
            return Err(DriveError::Conflict);
        }
        if old.is_some_and(|g| {
            g.root != grant.root || g.subject != grant.subject || (grant.active && (!g.active || g.expires_at.is_some_and(|expiry| expiry <= now)))
        }) {
            return Err(DriveError::InvalidInput("Create a new grant instead of repurposing a revoked grant".into()));
        }
        if old.is_none() && self.grants.len() >= MAX_GRANTS {
            return Err(DriveError::Limit("Sharing limit reached".into()));
        }
        let next_version = expected.unwrap_or(0).checked_add(1).ok_or(DriveError::Internal)?;
        let tree_version = self.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        let grant = AccessGrant { version: next_version, ..grant };
        self.grants.insert(grant.id, grant.clone());
        self.tree_version = tree_version;
        Ok(grant)
    }
    #[cfg(test)]
    pub(crate) fn disconnect(&mut self, actor: &Actor, id: u64) -> DriveResult<()> {
        let agent = self.agents.get(&id).ok_or(DriveError::Inaccessible)?;
        if actor.subject != Participant::User(agent.sponsor) || actor.credential.is_some() {
            return Err(DriveError::Inaccessible);
        }
        let version = self.tree_version.checked_add(1).ok_or(DriveError::Internal)?;
        for grant in self.grants.values_mut().filter(|g| g.subject == Participant::Agent(id)) {
            grant.active = false;
        }
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.active = false;
        }
        self.tree_version = version;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    fn principal(n: u8) -> Principal {
        Principal::self_authenticating([n; 32])
    }
    fn fixture() -> (AccessGraph, Actor, Actor) {
        let mut graph = AccessGraph::default();
        for (id, parent) in [(1, None), (2, Some(1)), (3, Some(1)), (4, Some(2))] {
            let id = crate::test_id(id);
            let parent = parent.map(crate::test_id);
            graph.objects.insert(
                id,
                DriveObject {
                    id,
                    space: crate::test_id(1),
                    owner: principal(1),
                    parent,
                    name: format!("Folder {id}"),
                    kind: ObjectKind::Folder,
                    version: 1,
                    created_at: 1,
                    updated_at: 1,
                    trashed_at: None,
                },
            );
        }
        graph.agents.insert(5, AgentConnection { id: 5, sponsor: principal(2), name: "Bob's AI".into(), active: true, created_at: 1 });
        (graph, Actor::user(principal(1)).unwrap(), Actor::user(principal(2)).unwrap())
    }
    fn grant(id: u64, root: ObjectId, subject: Participant, capabilities: Capabilities) -> AccessGrant {
        AccessGrant { id, root, subject, capabilities, expires_at: None, active: true, version: 0 }
    }
    #[test]
    fn sharing_status_tracks_live_human_grants_without_revealing_other_recipients() {
        let (mut graph, alice, bob) = fixture();
        assert_eq!(graph.sharing(&alice, crate::test_id(2), 10).unwrap(), SharingStatus::default());
        graph.set_grant(&alice, grant(10, crate::test_id(2), bob.subject.clone(), Capabilities::READ), None, 10).unwrap();
        assert_eq!(graph.sharing(&alice, crate::test_id(2), 11).unwrap(), SharingStatus { direct: true, inherited: false });
        assert_eq!(graph.sharing(&bob, crate::test_id(4), 11).unwrap(), SharingStatus { direct: false, inherited: true });
        graph.set_grant(&alice, grant(11, crate::test_id(4), Participant::User(principal(3)), Capabilities::READ), None, 11).unwrap();
        assert_eq!(graph.sharing(&bob, crate::test_id(4), 12).unwrap(), SharingStatus { direct: false, inherited: true });
        assert_eq!(graph.sharing(&alice, crate::test_id(4), 12).unwrap(), SharingStatus { direct: true, inherited: true });
        // Moving out of the shared parent removes inherited sharing, not its direct grant.
        graph.objects.get_mut(&crate::test_id(4)).unwrap().parent = Some(crate::test_id(3));
        assert_eq!(graph.sharing(&alice, crate::test_id(4), 13).unwrap(), SharingStatus { direct: true, inherited: false });
        assert!(graph.sharing(&bob, crate::test_id(4), 13).is_err());
        graph.grants.get_mut(&11).unwrap().expires_at = Some(14);
        assert_eq!(graph.sharing(&alice, crate::test_id(4), 14).unwrap(), SharingStatus::default());
        graph.grants.get_mut(&10).unwrap().active = false;
        assert_eq!(graph.sharing(&alice, crate::test_id(2), 15).unwrap(), SharingStatus::default());
        // Grants to the owner or grants with no Read access are not sharing.
        graph.grants.insert(12, grant(12, crate::test_id(2), alice.subject.clone(), Capabilities::READ));
        graph.grants.insert(13, grant(13, crate::test_id(2), bob.subject.clone(), Capabilities::NONE));
        assert_eq!(graph.sharing(&alice, crate::test_id(2), 15).unwrap(), SharingStatus::default());
    }
    #[test]
    fn collaborator_chooses_agent_scope_with_no_owner_delegation_flag() {
        let (mut graph, alice, bob) = fixture();
        let agent = graph.agent_actor(5, "verified-credential".into()).unwrap();
        assert_eq!(graph.effective(&agent, crate::test_id(2), 10, false).unwrap(), Capabilities::NONE);
        graph.set_grant(&alice, grant(10, crate::test_id(2), bob.subject.clone(), Capabilities::EDIT), None, 10).unwrap();
        graph.set_grant(&bob, grant(11, crate::test_id(2), agent.subject.clone(), Capabilities::PROPOSE), None, 10).unwrap();
        assert_eq!(graph.effective(&agent, crate::test_id(4), 11, false).unwrap(), Capabilities::PROPOSE);
        assert_eq!(graph.effective(&agent, crate::test_id(3), 11, false).unwrap(), Capabilities::NONE);
        assert!(graph.set_grant(&bob, grant(12, crate::test_id(2), agent.subject.clone(), Capabilities::ALL), None, 10).is_err());
        assert!(graph.set_grant(&alice, grant(12, crate::test_id(2), agent.subject.clone(), Capabilities::READ), None, 10).is_err());
        graph.set_grant(&alice, grant(10, crate::test_id(2), bob.subject.clone(), Capabilities::READ), Some(1), 12).unwrap();
        assert_eq!(graph.effective(&agent, crate::test_id(4), 13, false).unwrap(), Capabilities::READ);
        graph.set_grant(&alice, grant(13, crate::test_id(3), bob.subject.clone(), Capabilities::ALL), None, 14).unwrap();
        assert_eq!(graph.effective(&agent, crate::test_id(3), 15, false).unwrap(), Capabilities::NONE);
    }
    #[test]
    fn live_union_expiry_and_explicit_agent_revocation_have_distinct_meanings() {
        let (mut graph, alice, bob) = fixture();
        let agent = graph.agent_actor(5, "verified-credential".into()).unwrap();
        let share = graph.set_grant(&alice, grant(10, crate::test_id(2), bob.subject.clone(), Capabilities::EDIT), None, 10).unwrap();
        let ai = graph.set_grant(&bob, grant(11, crate::test_id(2), agent.subject.clone(), Capabilities::EDIT), None, 10).unwrap();
        graph.set_grant(&alice, grant(12, crate::test_id(4), bob.subject.clone(), Capabilities::READ), None, 10).unwrap();
        graph.set_grant(&alice, AccessGrant { active: false, ..share }, Some(1), 11).unwrap();
        assert_eq!(graph.effective(&agent, crate::test_id(4), 12, false).unwrap(), Capabilities::READ);
        assert_eq!(graph.effective(&agent, crate::test_id(2), 12, false).unwrap(), Capabilities::NONE);
        graph
            .set_grant(&alice, AccessGrant { expires_at: Some(20), ..grant(13, crate::test_id(2), bob.subject.clone(), Capabilities::ALL) }, None, 12)
            .unwrap();
        assert_eq!(graph.effective(&agent, crate::test_id(4), 19, false).unwrap(), Capabilities::EDIT);
        assert_eq!(graph.effective(&agent, crate::test_id(4), 20, false).unwrap(), Capabilities::READ);
        graph.set_grant(&bob, AccessGrant { active: false, ..ai.clone() }, Some(1), 21).unwrap();
        graph.set_grant(&alice, grant(14, crate::test_id(2), bob.subject.clone(), Capabilities::ALL), None, 22).unwrap();
        assert_eq!(graph.effective(&agent, crate::test_id(4), 23, false).unwrap(), Capabilities::NONE);
        assert!(graph.set_grant(&bob, ai, Some(2), 24).is_err());
    }
    #[test]
    fn owner_agent_never_inherits_administration_and_disconnect_closes_every_scope() {
        let (mut graph, alice, _) = fixture();
        graph.agents.insert(6, AgentConnection { id: 6, sponsor: alice.sponsor, name: "Alice's AI".into(), active: true, created_at: 1 });
        let agent = graph.agent_actor(6, "verified-credential".into()).unwrap();
        for (id, root) in [(10, 2), (11, 3)] {
            graph.set_grant(&alice, grant(id, crate::test_id(root), agent.subject.clone(), Capabilities::ALL), None, 10).unwrap();
        }
        assert_eq!(graph.effective(&agent, crate::test_id(2), 11, false).unwrap(), Capabilities::ALL);
        assert!(graph.owner(&agent, crate::test_id(2)).is_err());
        graph.disconnect(&alice, 6).unwrap();
        assert!(graph.effective(&agent, crate::test_id(2), 12, false).is_err());
        assert!(graph.effective(&agent, crate::test_id(3), 12, false).is_err());
        assert!(graph.agent_actor(6, "renewed-credential".into()).is_err());
    }
    #[test]
    fn ancestry_rejects_cycles_foreign_spaces_and_trashed_parents() {
        let (mut graph, alice, _) = fixture();
        graph.objects.get_mut(&crate::test_id(2)).unwrap().trashed_at = Some(5);
        assert!(graph.effective(&alice, crate::test_id(4), 10, false).is_err());
        assert!(graph.effective(&alice, crate::test_id(4), 10, true).is_ok());
        graph.objects.get_mut(&crate::test_id(2)).unwrap().trashed_at = None;
        graph.objects.get_mut(&crate::test_id(2)).unwrap().parent = Some(crate::test_id(4));
        assert!(graph.effective(&alice, crate::test_id(4), 10, false).is_err());
        graph.objects.get_mut(&crate::test_id(2)).unwrap().parent = Some(crate::test_id(1));
        graph.objects.get_mut(&crate::test_id(2)).unwrap().space = crate::test_id(99);
        assert!(graph.effective(&alice, crate::test_id(4), 10, false).is_err());
        assert!(Actor::user(Principal::anonymous()).is_err());
    }
}

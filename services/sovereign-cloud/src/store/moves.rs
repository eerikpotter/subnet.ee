//! One move planner for preview and mutation; all output is derived from the catalog.
use super::*;
use crate::permissions::{AccessGraph, Actor};
use sovereign_cloud::drive::*;
use std::collections::{BTreeMap, BTreeSet};

// A capability's latest expiry completely describes additive, expiring grants.
// 0 = absent, MAX = unbounded. Comparing only today's booleans misses latent changes.
type Envelope = [u128; 5];
fn envelope(graph: &AccessGraph, id: ObjectId, now: u64) -> DriveResult<BTreeMap<Participant, Envelope>> {
    let object = graph.objects.get(&id).ok_or(DriveError::Inaccessible)?;
    let path = graph.ancestry(id, true)?;
    let mut result = BTreeMap::new();
    for g in graph.grants.values().filter(|g| g.active && g.capabilities.read && path.contains(&g.root) && g.expires_at.is_none_or(|e| e > now)) {
        let rights = g.capabilities;
        let bits = [rights.read, rights.propose, rights.create && object.kind == ObjectKind::Folder, rights.edit, rights.delete];
        let entry = result.entry(g.subject.clone()).or_insert([0; 5]);
        for (value, bit) in entry.iter_mut().zip(bits) {
            if bit {
                *value = (*value).max(g.expires_at.map(u128::from).unwrap_or(u128::MAX));
            }
        }
    }
    // The owner's rights never depend on inherited grants.
    let mut owner = [u128::MAX; 5];
    if object.kind != ObjectKind::Folder {
        owner[2] = 0;
    }
    if graph.teams.contains_key(&object.space) {
        for ((space, principal), role) in &graph.members {
            if *space != object.space {
                continue;
            }
            let rights = role.capabilities();
            let bits = [rights.read, rights.propose, rights.create && object.kind == ObjectKind::Folder, rights.edit, rights.delete];
            let entry = result.entry(Participant::User(*principal)).or_insert([0; 5]);
            for (value, bit) in entry.iter_mut().zip(bits) {
                if bit {
                    *value = u128::MAX;
                }
            }
        }
    } else {
        result.insert(Participant::User(object.owner), owner);
    }
    let public = graph.public_deadline(id, now, true)?;
    if public > 0 {
        result.insert(Participant::User(Principal::anonymous()), [public, 0, 0, 0, 0]);
    }
    let humans = result.clone();
    for (subject, rights) in &mut result {
        if let Participant::Agent(id) = subject {
            let human = graph.agents.get(id).filter(|a| a.active).and_then(|a| humans.get(&Participant::User(a.sponsor))).copied().unwrap_or([0; 5]);
            for (value, bound) in rights.iter_mut().zip(human) {
                *value = (*value).min(bound);
            }
        }
    }
    result.retain(|_, rights| rights[0] != 0);
    Ok(result)
}
impl<M: Memory> Store<M> {
    pub(crate) fn drive_move_preview(&self, actor: &Actor, object: ObjectId, parent: ObjectId, now: u64) -> DriveResult<MovePreview> {
        {
            let mut graph = self.graph_for_object(object)?;
            graph.require(actor, object, Capabilities::READ, now, false)?;
            self.read_subtree(&mut graph, object)?;
            self.read_path(&mut graph, parent)?;
            self.plan_move(&graph, actor, object, parent, now)
        }
    }
    pub(super) fn plan_move(&self, graph: &AccessGraph, actor: &Actor, id: ObjectId, parent: ObjectId, now: u64) -> DriveResult<MovePreview> {
        self.plan_relocation(graph, actor, id, parent, None, now)
    }
    pub(crate) fn drive_restore_preview(&self, actor: &Actor, id: ObjectId, parent: ObjectId, name: &str, now: u64) -> DriveResult<MovePreview> {
        {
            let mut graph = self.graph_for_object(id)?;
            graph.require(actor, id, Capabilities::READ, now, true)?;
            self.read_subtree(&mut graph, id)?;
            self.read_path(&mut graph, parent)?;
            self.plan_relocation(&graph, actor, id, parent, Some(name), now)
        }
    }
    pub(super) fn plan_relocation(
        &self,
        graph: &AccessGraph,
        actor: &Actor,
        id: ObjectId,
        parent: ObjectId,
        restore_name: Option<&str>,
        now: u64,
    ) -> DriveResult<MovePreview> {
        let restoring = restore_name.is_some();
        let object = graph.objects.get(&id).ok_or(DriveError::Inaccessible)?;
        let name = restore_name.map(super::collaboration::name).transpose()?.unwrap_or_else(|| object.name.clone());
        let required = Capabilities { edit: true, delete: true, ..Capabilities::READ };
        graph.require(actor, id, required, now, restoring)?;
        if restoring && object.trashed_at.is_none() {
            return Err(DriveError::Conflict);
        }
        if !restoring || object.parent != Some(parent) {
            graph.require(actor, parent, Capabilities { create: true, ..Capabilities::READ }, now, false)?;
        }
        let destination = graph.objects.get(&parent).ok_or(DriveError::Inaccessible)?;
        if object.parent.is_none()
            || destination.space != object.space
            || destination.kind != ObjectKind::Folder
            || graph.ancestry(parent, false)?.contains(&id)
        {
            return Err(DriveError::InvalidInput("Choose a folder in the same space outside this subtree".into()));
        }
        if restoring || object.parent != Some(parent) {
            self.check_new_name(parent, &name, Some(id))?;
        }
        let mut after = graph.clone();
        after.objects.get_mut(&id).ok_or(DriveError::Inaccessible)?.parent = Some(parent);
        let mut changes: BTreeMap<Participant, MoveAccessChange> = BTreeMap::new();
        let mut affected_objects = 0;
        for child in graph.objects.values() {
            if !graph.ancestry(child.id, true)?.contains(&id) {
                continue;
            }
            graph.require(actor, child.id, required, now, true)?;
            after.ancestry(child.id, true)?; // Includes the depth limit after reparenting.
            let before = envelope(graph, child.id, now)?;
            let next = envelope(&after, child.id, now)?;
            let subjects: BTreeSet<_> = before.keys().chain(next.keys()).cloned().collect();
            let mut changed = false;
            for subject in subjects {
                let a = before.get(&subject).copied().unwrap_or([0; 5]);
                let b = next.get(&subject).copied().unwrap_or([0; 5]);
                if a == b {
                    continue;
                }
                // Editors cannot learn others' access or change its duration/audience.
                graph.owner(actor, id)?;
                changed = true;
                let change = changes.entry(subject.clone()).or_insert(MoveAccessChange { subject, gains: false, loses: false, objects: 0 });
                change.objects += 1;
                change.gains |= a.iter().zip(b).any(|(old, new)| new > *old);
                change.loses |= a.iter().zip(b).any(|(old, new)| new < *old);
            }
            affected_objects += u64::from(changed);
        }
        Ok(MovePreview {
            object_version: object.version,
            tree_version: graph.tree_version,
            affected_objects,
            affected_subjects: changes.len() as u64,
            changes: changes.into_values().take(100).collect(),
        })
    }
}

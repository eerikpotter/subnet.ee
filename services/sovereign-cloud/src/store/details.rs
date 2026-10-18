//! Caller-scoped metadata projections; no second catalog or recipient directory.
use super::collaboration::decoded;
use super::*;
use crate::permissions::Actor;
use sovereign_cloud::drive::*;

impl<M: Memory> Store<M> {
    pub(crate) fn drive_details(&self, actor: &Actor, id: ObjectId, now: u64) -> DriveResult<DriveDetails> {
        let graph = self.indexed_graph(id)?;
        let entry = self.entry(&graph, actor, id, now)?;
        let path = graph.ancestry(id, false)?;
        let location = |id| {
            graph.require(actor, id, Capabilities::READ, now, false).ok()?;
            let object = graph.objects.get(&id)?;
            Some(ObjectLocation { id, name: object.name.clone() })
        };
        let locations = path.iter().rev().filter(|p| **p != id).filter_map(|p| location(*p)).collect();
        let access = graph
            .grants
            .values()
            .filter(|g| {
                g.subject == actor.subject && g.active && g.capabilities.read && path.contains(&g.root) && g.expires_at.is_none_or(|expiry| expiry > now)
            })
            .map(|g| AccessSource {
                inherited: g.root != id,
                location: location(g.root),
                capabilities: g.capabilities.intersect(entry.capabilities),
                expires_at: g.expires_at,
            })
            .collect();
        let revision_count = self.projects.get(&id).map(|bytes| decoded::<Project>(&bytes).map(|p| p.revisions.len() as u64)).transpose()?.unwrap_or(0);
        Ok(DriveDetails { entry, locations, access, revision_count })
    }
}

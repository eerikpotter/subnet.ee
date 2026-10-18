//! Owner-only projection over canonical content/accounting. No usage database.
use super::collaboration::decoded;
use super::*;
use crate::permissions::Actor;
use sovereign_cloud::drive::*;
impl<M: Memory> Store<M> {
    pub(crate) fn drive_storage(&self, actor: &Actor) -> DriveResult<StorageOverview> {
        Actor::user(actor.sponsor)?;
        if actor.subject != Participant::User(actor.sponsor) || actor.credential.is_some() {
            return Err(DriveError::Inaccessible);
        }
        let graph = self.graph_for_owner(actor.sponsor)?;
        let mut view = StorageOverview {
            current_bytes: 0,
            history_bytes: 0,
            trash_bytes: 0,
            pending_bytes: 0,
            used_bytes: self.usage.get(&actor.sponsor).unwrap_or(0),
            quota_bytes: USER_QUOTA_BYTES,
            files: vec![],
        };
        for entry in self.project_owners.range((actor.sponsor, ObjectId::MIN)..=(actor.sponsor, ObjectId::MAX)) {
            let id = entry.key().1;
            let project: Project = decoded(&self.projects.get(&id).ok_or(DriveError::Internal)?)?;
            let total = project.revisions.iter().try_fold(0u64, |sum, r| sum.checked_add(r.bytes).ok_or(DriveError::Internal))?;
            let current_bytes = project.revisions.last().map_or(0, |r| r.bytes);
            let history_bytes = total.checked_sub(current_bytes).ok_or(DriveError::Internal)?;
            let trashed = graph.objects.contains_key(&id) && graph.ancestry(id, false).is_err();
            if trashed {
                view.trash_bytes = view.trash_bytes.checked_add(total).ok_or(DriveError::Internal)?;
            } else {
                view.current_bytes = view.current_bytes.checked_add(current_bytes).ok_or(DriveError::Internal)?;
                view.history_bytes = view.history_bytes.checked_add(history_bytes).ok_or(DriveError::Internal)?;
            }
            view.files.push(StorageItem {
                object: id,
                name: project.name,
                current_bytes,
                history_bytes,
                revision_count: project.revisions.len() as u64,
                trashed,
            });
        }
        view.pending_bytes = view
            .used_bytes
            .checked_sub(view.current_bytes)
            .and_then(|v| v.checked_sub(view.history_bytes))
            .and_then(|v| v.checked_sub(view.trash_bytes))
            .ok_or(DriveError::Internal)?;
        view.files.sort_by(|a, b| (b.current_bytes + b.history_bytes).cmp(&(a.current_bytes + a.history_bytes)).then(a.object.cmp(&b.object)));
        Ok(view)
    }
}

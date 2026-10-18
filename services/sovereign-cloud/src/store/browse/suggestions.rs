//! Suggestions are a derived, permission-filtered view over existing catalog facts.
//! Private summaries stay keyed by viewer; no ranking state or event log is stored.
use super::*;

const MINUTE: u64 = 60_000_000_000;
const DAY: u64 = 24 * 60 * MINUTE;

impl<M: Memory> Store<M> {
    pub(crate) fn drive_acknowledge_open(&mut self, actor: &Actor, id: ObjectId, revision: Option<u64>, now: u64) -> DriveResult<()> {
        self.viewer(actor)?;
        let entry = self.drive_get(actor, id, now)?;
        match (&entry.object.kind, revision) {
            (ObjectKind::Folder | ObjectKind::Shortcut(_), None) => {}
            (ObjectKind::File(_), Some(revision)) => {
                if entry.latest.as_ref().is_none_or(|r| r.id != revision) && !self.drive_history(actor, id, now)?.iter().any(|r| r.id == revision) {
                    return Err(DriveError::Inaccessible);
                }
            }
            _ => return Err(DriveError::InvalidInput("An opened file requires its revision".into())),
        }
        // The account root is navigation, not a suggested folder.
        if entry.object.parent.is_none() {
            return Ok(());
        }
        self.record_open(actor, id, revision, now)
    }
    pub(super) fn record_open(&mut self, actor: &Actor, id: ObjectId, revision: Option<u64>, now: u64) -> DriveResult<()> {
        let mut item = self.viewer_item(actor.sponsor, id)?;
        if item.opened_revision == revision && item.opened_at.is_some_and(|at| now.saturating_sub(at) < MINUTE) {
            return Ok(());
        }
        item.opened_at = Some(now);
        item.opened_revision = revision;
        self.save_viewer_item(actor, id, item, now)
    }
    pub(crate) fn drive_dismiss_suggestion(&mut self, actor: &Actor, id: ObjectId, now: u64) -> DriveResult<()> {
        self.viewer(actor)?;
        self.drive_get(actor, id, now)?;
        let mut item = self.viewer_item(actor.sponsor, id)?;
        // Duplicate requests do not extend the dismissal indefinitely.
        if item.hidden_until.is_some_and(|until| until > now) {
            return Ok(());
        }
        item.hidden_until = Some(now.saturating_add(7 * DAY));
        self.save_viewer_item(actor, id, item, now)
    }
    pub(crate) fn drive_suggested_home(&self, actor: &Actor, now: u64) -> DriveResult<SuggestedHome> {
        self.viewer(actor)?;
        let mut candidates = BTreeSet::new();
        // Rank a bounded candidate window, not the whole account. No persistent
        // scores or replicated permission snapshots are needed for suggestions.
        for scope in [BrowseScope::All, BrowseScope::Starred, BrowseScope::Recent] {
            let query = BrowseQuery {
                scope,
                text: String::new(),
                file_type: String::new(),
                owner: None,
                modified_after: None,
                modified_before: None,
                sort: SortField::Modified,
                descending: true,
            };
            candidates.extend(self.drive_query(actor, query, None, 50, now)?.items.into_iter().map(|i| i.entry.object.id));
        }
        let mut ranked = Vec::new();
        for id in candidates {
            let entry = self.drive_get(actor, id, now)?;
            let state = self.viewer_item(actor.sponsor, id)?;
            if state.hidden_until.is_some_and(|until| until > now) {
                continue;
            }
            let recent = state.opened_at.filter(|at| now.saturating_sub(*at) <= 30 * DAY);
            let changed =
                entry.latest.as_ref().filter(|r| recent.is_some_and(|at| r.created_at > at) && state.opened_revision.is_some_and(|seen| seen != r.id));
            let (priority, reason, reason_at) = if let Some(revision) = changed {
                (3, SuggestionReason::UpdatedSinceOpened, revision.created_at)
            } else if let Some(opened) = recent {
                (2, SuggestionReason::RecentlyOpened, opened)
            } else if state.starred {
                (1, SuggestionReason::Starred, 0)
            } else {
                (0, SuggestionReason::RecentlyCreated, entry.object.created_at)
            };
            ranked.push((
                priority,
                SuggestedItem { item: BrowseItem { entry, color: state.color, starred: state.starred, opened_at: state.opened_at }, reason, reason_at },
            ));
        }
        // Stable tie-breaking; renames never pretend to be document edits.
        ranked.sort_by_key(|(priority, s)| std::cmp::Reverse((*priority, s.reason_at, s.item.entry.object.id)));
        let mut home = SuggestedHome { files: Vec::new(), folders: Vec::new() };
        for (_, item) in ranked {
            let (items, limit) = if item.item.entry.object.kind == ObjectKind::Folder { (&mut home.folders, 6) } else { (&mut home.files, 20) };
            if items.len() < limit {
                items.push(item);
            }
        }
        Ok(home)
    }
}

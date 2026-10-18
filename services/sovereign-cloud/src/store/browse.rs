//! Permission-filtered metadata retrieval and account-private presentation state.
//! Existing object/parent/grant indexes remain the only catalog and ACL source.
use super::collaboration::{decoded, encoded};
use super::*;
use crate::permissions::{AccessGraph, Actor};
use serde::Deserialize;
use sovereign_cloud::drive::*;
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

// Matches the existing catalog bound. Never silently return incomplete search.
const MAX_CANDIDATES: usize = 10_000;
const RECENT_LIMIT: usize = 200;

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct ViewerItem {
    starred: bool,
    color: Option<String>,
    opened_at: Option<u64>,
    opened_revision: Option<u64>,
    hidden_until: Option<u64>,
}

mod pagination;
mod suggestions;

#[cfg(test)]
#[path = "browse_tests.rs"]
mod tests;
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct ViewerSettings {
    version: u64,
    item_count: u64,
    starred_count: u64,
    preferences: BrowsePreferences,
}
pub(super) fn normalized(value: &str) -> String {
    value.nfc().collect::<String>().to_lowercase()
}
pub(super) fn type_matches(object: &DriveObject, filter: &str) -> bool {
    match (&object.kind, filter) {
        (_, "") => true,
        (ObjectKind::Folder, "folder") => true,
        (ObjectKind::Shortcut(_), "shortcut") => true,
        (ObjectKind::File(_), "file") => true,
        (ObjectKind::File(format), "pdf") => format.media_type == "application/pdf" || object.name.to_lowercase().ends_with(".pdf"),
        (ObjectKind::File(format), "image" | "audio" | "video" | "text") => format.media_type.starts_with(&format!("{filter}/")),
        (ObjectKind::File(format), app) => format.craft.as_ref().is_some_and(|kind| kind.created_by_app == app),
        _ => false,
    }
}
impl<M: Memory> Store<M> {
    fn viewer(&self, actor: &Actor) -> DriveResult<ViewerSettings> {
        if actor.sponsor == Principal::anonymous()
            || actor.subject != Participant::User(actor.sponsor)
            || actor.credential.is_some()
            || !self.accounts.contains_key(&actor.sponsor)
        {
            return Err(DriveError::Inaccessible);
        }
        self.viewer_settings.get(&actor.sponsor).map(|bytes| decoded(&bytes)).unwrap_or_else(|| Ok(ViewerSettings::default()))
    }
    fn viewer_item(&self, caller: Principal, id: ObjectId) -> DriveResult<ViewerItem> {
        self.viewer_items.get(&(caller, id)).map(|bytes| decoded(&bytes)).unwrap_or_else(|| Ok(ViewerItem::default()))
    }
    pub(crate) fn drive_folder_color(&mut self, actor: &Actor, object: ObjectId, color: Option<String>, now: u64) -> DriveResult<()> {
        self.viewer(actor)?;
        let entry = self.drive_get(actor, object, now)?;
        if entry.object.kind != ObjectKind::Folder {
            return Err(DriveError::InvalidInput("Only folders have a color".into()));
        }
        if color.as_deref().is_some_and(|c| !["blue", "gold", "green", "purple", "rose", "gray"].contains(&c)) {
            return Err(DriveError::InvalidInput("Unknown folder color".into()));
        }
        let mut item = self.viewer_item(actor.sponsor, object)?;
        item.color = color;
        // Same bounded private record as stars and opening summaries.
        let count = self.viewer(actor)?.item_count;
        if count >= 1000 && !self.viewer_items.contains_key(&(actor.sponsor, object)) {
            return Err(DriveError::Limit("Personal item limit reached".into()));
        }
        self.save_viewer_item(actor, object, item, now)
    }
    pub(crate) fn drive_preferences(&self, actor: &Actor) -> DriveResult<BrowsePreferences> {
        Ok(self.viewer(actor)?.preferences)
    }
    pub(crate) fn drive_set_preferences(&mut self, actor: &Actor, preferences: BrowsePreferences) -> DriveResult<()> {
        let mut viewer = self.viewer(actor)?;
        if viewer.preferences == preferences {
            return Ok(());
        }
        viewer.version = viewer.version.checked_add(1).ok_or(DriveError::Internal)?;
        viewer.preferences = preferences;
        let bytes = encoded(&viewer)?;
        self.viewer_settings.insert(actor.sponsor, bytes);
        Ok(())
    }
    pub(crate) fn drive_star(&mut self, actor: &Actor, id: ObjectId, starred: bool, now: u64) -> DriveResult<()> {
        let viewer = self.viewer(actor)?;
        self.drive_get(actor, id, now)?;
        let mut item = self.viewer_item(actor.sponsor, id)?;
        if item.starred == starred {
            return Ok(());
        }
        if starred && viewer.starred_count >= MAX_CANDIDATES as u64 {
            return Err(DriveError::Limit("Starred item limit reached".into()));
        }
        item.starred = starred;
        self.save_viewer_item(actor, id, item, now)
    }

    /// Explicit user open, never called by background catalog refreshes.
    pub(crate) fn drive_open(&mut self, actor: &Actor, id: ObjectId, now: u64) -> DriveResult<DriveEntry> {
        self.viewer(actor)?;
        let entry = self.drive_get(actor, id, now)?;
        if entry.object.kind == ObjectKind::Folder {
            return Ok(entry);
        }
        // An intent to open carries no assertion
        // that a particular revision was successfully decoded by the editor.
        self.record_open(actor, id, None, now)?;
        Ok(entry)
    }
    fn save_viewer_item(&mut self, actor: &Actor, id: ObjectId, item: ViewerItem, now: u64) -> DriveResult<()> {
        let mut viewer = self.viewer(actor)?;
        // Only the target and capped recent/dismissal summaries participate.
        // Thousands of intentional stars or colors never turn one open into a
        // whole-account read or write.
        let bounds = (actor.sponsor, (0, ObjectId::MIN))..=(actor.sponsor, (u64::MAX, ObjectId::MAX));
        let mut ids = BTreeSet::from([id]);
        ids.extend(self.viewer_opened.range(bounds.clone()).take(RECENT_LIMIT).map(|e| e.key().1.1));
        ids.extend(self.viewer_hidden.range(bounds).take(RECENT_LIMIT).map(|e| e.key().1.1));
        let mut recent = ids.into_iter().map(|other| Ok((other, self.viewer_item(actor.sponsor, other)?))).collect::<DriveResult<Vec<_>>>()?;
        for (other, value) in &mut recent {
            if *other == id {
                *value = item.clone();
            }
        }
        recent.sort_by_key(|(id, item)| (item.opened_at, *id));
        let mut count = recent.iter().filter(|(_, item)| item.opened_at.is_some()).count().saturating_sub(RECENT_LIMIT);
        for (_, item) in &mut recent {
            if item.opened_at.is_some() && count > 0 {
                item.opened_at = None;
                item.opened_revision = None;
                count -= 1;
            }
            if item.hidden_until.is_some_and(|until| until <= now) {
                item.hidden_until = None;
            }
        }
        recent.sort_by_key(|(id, item)| (item.hidden_until, *id));
        let mut count = recent.iter().filter(|(_, item)| item.hidden_until.is_some()).count().saturating_sub(RECENT_LIMIT);
        let mut updates = Vec::new();
        for (id, mut item) in recent {
            if item.hidden_until.is_some() && count > 0 {
                item.hidden_until = None;
                count -= 1;
            }
            let old_bytes = self.viewer_items.get(&(actor.sponsor, id));
            let old: ViewerItem = old_bytes.as_ref().map(|b| decoded(b)).transpose()?.unwrap_or_default();
            if old == item {
                continue;
            }
            let present = item.starred || item.color.is_some() || item.opened_at.is_some() || item.hidden_until.is_some();
            viewer.item_count =
                viewer.item_count.checked_sub(u64::from(old_bytes.is_some())).and_then(|n| n.checked_add(u64::from(present))).ok_or(DriveError::Internal)?;
            viewer.starred_count =
                viewer.starred_count.checked_sub(u64::from(old.starred)).and_then(|n| n.checked_add(u64::from(item.starred))).ok_or(DriveError::Internal)?;
            let bytes = if present { Some(encoded(&item)?) } else { None };
            updates.push((id, old, item, bytes));
        }
        viewer.version = viewer.version.checked_add(1).ok_or(DriveError::Internal)?;
        let viewer_bytes = encoded(&viewer)?;
        // All decoding, counters and serialization finish before stable writes.
        for (id, old, item, bytes) in updates {
            if let Some(at) = old.opened_at {
                self.viewer_opened.remove(&(actor.sponsor, (at, id)));
            }
            if let Some(at) = old.hidden_until {
                self.viewer_hidden.remove(&(actor.sponsor, (at, id)));
            }
            if let Some(at) = item.opened_at {
                self.viewer_opened.insert((actor.sponsor, (at, id)), 0);
            }
            if let Some(at) = item.hidden_until {
                self.viewer_hidden.insert((actor.sponsor, (at, id)), 0);
            }
            if let Some(bytes) = bytes {
                self.viewer_items.insert((actor.sponsor, id), bytes);
            } else {
                self.viewer_items.remove(&(actor.sponsor, id));
            }
        }
        self.viewer_settings.insert(actor.sponsor, viewer_bytes);
        Ok(())
    }
}

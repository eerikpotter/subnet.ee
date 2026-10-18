//! Bounded, ordered retrieval. Filter misses advance the bookmark too; a page
//! can be empty and still have a continuation. Clients must follow `next`.
use super::super::catalog_index::CatalogKey;
use super::*;

pub(super) const SCAN_LIMIT: usize = 512;
const SOURCE_LIMIT: usize = 64;
const SHARED_ROOT_LIMIT: usize = 256;
#[derive(Serialize, Deserialize)]
struct BrowseBookmark {
    actor: Principal,
    query_hash: Vec<u8>,
    tree_version: u64,
    viewer_version: u64,
    expires_at: Option<u64>,
    after: ObjectId,
}

impl<M: Memory> Store<M> {
    fn shared_candidates(&self, actor: &Actor) -> DriveResult<Vec<ObjectId>> {
        let roots: Vec<_> =
            self.shared_roots.range((actor.sponsor, ObjectId::MIN)..=(actor.sponsor, ObjectId::MAX)).take(SHARED_ROOT_LIMIT + 1).map(|e| e.key().1).collect();
        if roots.len() > SHARED_ROOT_LIMIT {
            return Err(DriveError::Limit("Choose a folder to narrow this view".into()));
        }
        Ok(roots)
    }
    fn query_entry(&self, graph: &AccessGraph, actor: &Actor, id: ObjectId, trash: bool, now: u64) -> DriveResult<DriveEntry> {
        if !trash {
            return self.entry(graph, actor, id, now);
        }
        graph.require(actor, id, Capabilities { delete: true, ..Capabilities::READ }, now, true)?;
        let object = graph.objects.get(&id).ok_or(DriveError::Inaccessible)?.clone();
        if object.trashed_at.is_none() {
            return Err(DriveError::Inaccessible);
        }
        let latest = self.projects.get(&id).map(|b| decoded::<Project>(&b)).transpose()?.and_then(|p| p.revisions.last().cloned());
        Ok(DriveEntry { object, capabilities: graph.effective(actor, id, now, true)?, latest, sharing: None, attribution: None })
    }
    pub(crate) fn drive_query(&self, actor: &Actor, query: BrowseQuery, cursor: Option<BrowseCursor>, limit: u32, now: u64) -> DriveResult<BrowsePage> {
        let viewer = self.viewer(actor)?;
        let cursor: Option<BrowseBookmark> = cursor.map(|c| self.open_bookmark(&c, b"browse-v1", actor.sponsor)).transpose()?;
        if query.text.len() > 160
            || query.text.chars().any(char::is_control)
            || query.file_type.len() > 40
            || query.modified_after.zip(query.modified_before).is_some_and(|(a, b)| a > b)
        {
            return Err(DriveError::InvalidInput("Invalid search filters".into()));
        }
        if !["", "folder", "file", "shortcut", "image", "audio", "video", "text", "pdf"].contains(&query.file_type.as_str())
            && FileKind::for_app(&query.file_type).is_none()
        {
            return Err(DriveError::InvalidInput("Unknown file type".into()));
        }
        if query.sort == SortField::Opened && query.scope != BrowseScope::Recent {
            return Err(DriveError::InvalidInput("Opened order belongs to Recent".into()));
        }
        let viewer_version = if matches!(query.scope, BrowseScope::Recent | BrowseScope::Starred) { viewer.version } else { 0 };
        let query_hash = Sha256::digest(encoded(&query)?).to_vec();
        // Validate caller and query before using a client-supplied bookmark.
        if cursor.as_ref().is_some_and(|c| {
            c.actor != actor.sponsor || c.query_hash != query_hash || c.viewer_version != viewer_version || c.expires_at.is_some_and(|t| now >= t)
        }) {
            return Err(DriveError::Conflict);
        }

        let mut graph = self.read_graph();
        let mut sources = BTreeSet::new();
        let mut small = None;
        let mut folder = None;
        let mut breadcrumbs = Vec::new();
        let mut version = Sha256::new();
        let local_folder = match query.scope {
            BrowseScope::Folder(id) | BrowseScope::Item(id) => Some(id),
            _ => None,
        };
        version.update(self.scope_version(actor, local_folder)?.to_be_bytes());
        match query.scope {
            BrowseScope::Item(id) => {
                self.read_path(&mut graph, id)?;
                self.entry(&graph, actor, id, now)?;
                small = Some(vec![id]);
            }
            BrowseScope::Folder(id) | BrowseScope::Subtree(id) => {
                self.read_path(&mut graph, id)?;
                let entry = self.entry(&graph, actor, id, now)?;
                if entry.object.kind != ObjectKind::Folder {
                    return Err(DriveError::Inaccessible);
                }
                if matches!(query.scope, BrowseScope::Folder(_)) {
                    sources.insert((1, id));
                } else {
                    sources.insert((0, entry.object.space));
                }
                folder = Some(entry);
                for id in graph.ancestry(id, false)? {
                    match self.entry(&graph, actor, id, now) {
                        Ok(entry) => breadcrumbs.push(entry),
                        Err(DriveError::Inaccessible) => break,
                        Err(e) => return Err(e),
                    }
                }
                breadcrumbs.reverse();
            }
            BrowseScope::Recent => {
                // Summaries are independently capped at 200 opens. Reading this
                // index never visits all stars, files or unrelated accounts.
                small = Some(
                    self.viewer_opened
                        .range((actor.sponsor, (0, ObjectId::MIN))..=(actor.sponsor, (u64::MAX, ObjectId::MAX)))
                        .take(RECENT_LIMIT)
                        .map(|e| e.key().1.1)
                        .collect(),
                );
            }
            BrowseScope::Shared => {
                small = Some(self.shared_candidates(actor)?);
            }
            _ => {
                let account: Account = decoded(&self.accounts.get(&actor.sponsor).ok_or(DriveError::Inaccessible)?)?;
                sources.insert((0, account.personal_space));
                for root in self.shared_candidates(actor)? {
                    match self.read_path(&mut graph, root).and_then(|()| graph.require(actor, root, Capabilities::READ, now, true)) {
                        Ok(()) => {
                            sources.insert((0, graph.objects.get(&root).ok_or(DriveError::Internal)?.space));
                        }
                        Err(DriveError::Inaccessible) => {}
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        if sources.len() > SOURCE_LIMIT {
            return Err(DriveError::Limit("Choose a folder to narrow this view".into()));
        }
        // A discovery cursor depends only on spaces it actually reads. It never
        // uses a canister-wide epoch. Folder cursors use their contents + ancestry.
        for &(kind, id) in &sources {
            version.update(id.as_bytes());
            if kind == 0 {
                version.update(self.space_versions.get(&id).unwrap_or(0).to_be_bytes());
            }
        }
        if let Some(ids) = &small {
            for &id in ids {
                if let Some(bytes) = self.read_objects.get(&id) {
                    let object: DriveObject = decoded(&bytes)?;
                    version.update(id.as_bytes());
                    version.update(self.space_versions.get(&object.space).unwrap_or(0).to_be_bytes());
                }
            }
        }
        let tree_version = u64::from_be_bytes(version.finalize()[..8].try_into().map_err(|_| DriveError::Internal)?);
        if cursor.as_ref().is_some_and(|c| c.tree_version != tree_version) {
            return Err(DriveError::Conflict);
        }
        let field = match query.sort {
            SortField::Name | SortField::Opened => 0,
            SortField::Modified => 1,
            SortField::Size => 2,
        };
        let rank_folders = matches!(query.scope, BrowseScope::Folder(_) | BrowseScope::Shared);
        let key_for = |id| -> DriveResult<CatalogKey> {
            let object: DriveObject = decoded(&self.read_objects.get(&id).ok_or(DriveError::Conflict)?)?;
            let mut key = CatalogKey::for_object(&object, self.catalog_size(id)?, u8::from(rank_folders), object.space, field);
            if query.sort == SortField::Opened {
                key.number = self.viewer_item(actor.sponsor, id)?.opened_at.unwrap_or(0);
            }
            Ok(key)
        };
        let after = cursor.as_ref().map(|c| key_for(c.after)).transpose()?;
        let compare = |a: &CatalogKey, b: &CatalogKey| {
            a.rank.cmp(&b.rank).then_with(|| if query.descending { b.position().cmp(&a.position()) } else { a.position().cmp(&b.position()) })
        };
        let mut iterators: Vec<Box<dyn Iterator<Item = CatalogKey> + '_>> = Vec::new();
        if let Some(ids) = small {
            let mut keys = Vec::new();
            for id in ids {
                if !self.read_objects.contains_key(&id) {
                    continue;
                }
                let key = key_for(id)?;
                if after.as_ref().is_none_or(|a| compare(&key, a).is_gt()) {
                    keys.push(key);
                }
            }
            keys.sort_by(compare);
            iterators.push(Box::new(keys.into_iter()));
        } else {
            for (kind, id) in sources {
                iterators.push(self.catalog_iter(kind, id, field, query.descending, after.as_ref()));
            }
        }
        let mut iterators: Vec<_> = iterators.into_iter().map(|i| i.peekable()).collect();
        let text = normalized(query.text.trim());
        let mut items = Vec::new();
        let mut scanned = None;
        let mut visited = 0;
        let limit = limit.clamp(1, 50) as usize;
        while visited < SCAN_LIMIT && items.len() < limit {
            let mut best: Option<(usize, CatalogKey)> = None;
            for (i, iter) in iterators.iter_mut().enumerate() {
                if let Some(key) = iter.peek()
                    && best.as_ref().is_none_or(|(_, b)| compare(key, b).is_lt())
                {
                    best = Some((i, key.clone()));
                }
            }
            let Some((index, key)) = best else { break };
            iterators[index].next();
            visited += 1;
            scanned = Some(key.id);
            let trash = query.scope == BrowseScope::Trash;
            let entry = match self.read_path(&mut graph, key.id).and_then(|()| self.query_entry(&graph, actor, key.id, trash, now)) {
                Ok(entry) => entry,
                Err(DriveError::Inaccessible) => continue,
                Err(e) => return Err(e),
            };
            let object = &entry.object;
            let path = graph.ancestry(object.id, trash)?;
            if object.parent.is_none()
                || matches!(query.scope, BrowseScope::Subtree(root) if object.id == root || !path.contains(&root))
                || (query.scope == BrowseScope::Shared && object.owner == actor.sponsor)
                || (!matches!(query.scope, BrowseScope::Item(_) | BrowseScope::Folder(_) | BrowseScope::Trash) && self.safety.hidden(actor.sponsor, &path))
            {
                continue;
            }
            let state = self.viewer_item(actor.sponsor, object.id)?;
            if !normalized(&object.name).contains(&text)
                || !type_matches(object, &query.file_type)
                || query.owner.is_some_and(|o| o != object.owner)
                || query.modified_after.is_some_and(|t| object.updated_at < t)
                || query.modified_before.is_some_and(|t| object.updated_at >= t)
                || (query.scope == BrowseScope::Starred && !state.starred)
                || (query.scope == BrowseScope::Recent && (state.opened_at.is_none() || object.kind == ObjectKind::Folder))
            {
                continue;
            }
            items.push(BrowseItem { entry, color: state.color, starred: state.starred, opened_at: state.opened_at });
        }
        let more = iterators.iter_mut().any(|i| i.peek().is_some());
        let expires_at = graph.grants.values().filter(|g| g.active && g.subject == actor.subject).filter_map(|g| g.expires_at).filter(|t| *t > now).min();
        let next = if more {
            scanned
                .map(|after| {
                    self.seal_bookmark(
                        &BrowseBookmark { actor: actor.sponsor, query_hash, tree_version, viewer_version, expires_at, after },
                        b"browse-v1",
                        actor.sponsor,
                    )
                })
                .transpose()?
        } else {
            None
        };
        Ok(BrowsePage { items, next, folder, breadcrumbs, preferences: viewer.preferences })
    }
}

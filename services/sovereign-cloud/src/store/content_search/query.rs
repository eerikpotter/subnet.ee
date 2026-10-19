use super::*;
#[derive(Serialize, serde::Deserialize)]
struct Bookmark {
    binding: Vec<u8>,
    space: usize,
    anchor: String,
    after: ObjectId,
}
impl<M: Memory> Store<M> {
    pub(crate) fn content_query(&self, actor: &Actor, request: ContentQuery, now: u64) -> DriveResult<ContentPage> {
        if request.text.len() > 256 {
            return Err(DriveError::InvalidInput("Search text is too long".into()));
        }
        let terms: BTreeSet<_> = sovereign_cloud_text::tokens(&request.text).into_iter().collect();
        if terms.is_empty() || terms.len() > 8 {
            return Err(DriveError::InvalidInput("Use one to eight search words".into()));
        }
        let roots = if let Some(folder) = request.folder {
            let entry = self.drive_get(actor, folder, now)?;
            if entry.object.kind != ObjectKind::Folder {
                return Err(DriveError::Inaccessible);
            }
            BTreeSet::from([folder])
        } else {
            self.root_ids(actor)?
        };
        let mut spaces = BTreeSet::new();
        for id in &roots {
            if let Ok(graph) = self.indexed_graph(*id)
                && graph.require(actor, *id, Capabilities::READ, now, false).is_ok()
            {
                spaces.insert(graph.objects[id].space);
            }
        }
        let spaces: Vec<_> = spaces.into_iter().collect();
        let binding = Sha256::digest(encoded(&(&terms, &roots, &spaces))?).to_vec();
        let mut page: Bookmark = if let Some(cursor) = request.cursor {
            let page: Bookmark = self.open_bookmark(&cursor, b"content-search", actor.sponsor)?;
            if page.binding != binding || !terms.contains(&page.anchor) || page.space >= spaces.len() {
                return Err(DriveError::Conflict);
            }
            page
        } else {
            Bookmark { binding: binding.clone(), space: 0, anchor: String::new(), after: ObjectId::MIN }
        };
        let limit = request.limit.clamp(1, 50) as usize;
        let mut hits = Vec::new();
        let mut work = 0;
        while page.space < spaces.len() && hits.len() < limit && work < 128 {
            let space = spaces[page.space];
            if page.anchor.is_empty() {
                page.anchor = terms
                    .iter()
                    .min_by_key(|term| self.search.counts.get(&TermKey(space, (*term).clone())).unwrap_or(0))
                    .cloned()
                    .ok_or(DriveError::Internal)?;
            }
            let start = Posting(space, page.anchor.clone(), page.after);
            let end = Posting(space, page.anchor.clone(), ObjectId::MAX);
            let mut source = self.search.postings.range((Excluded(start), Included(end))).peekable();
            while work < 128 && hits.len() < limit {
                let Some(posting) = source.next() else {
                    break;
                };
                work += 1;
                let id = posting.key().2;
                let (revision, section) = posting.value();
                page.after = id;
                if terms.iter().any(|term| self.search.postings.get(&Posting(space, term.clone(), id)).is_none_or(|p| p.0 != revision)) {
                    continue;
                }
                let graph = match self.indexed_graph(id) {
                    Ok(graph) => graph,
                    Err(DriveError::Inaccessible) => continue,
                    Err(e) => return Err(e),
                };
                let path = graph.ancestry(id, false).unwrap_or_default();
                if !path.iter().any(|p| roots.contains(p)) {
                    continue;
                }
                if self.safety.hidden(actor.sponsor, &path) {
                    continue;
                }
                let entry = match self.entry(&graph, actor, id, now) {
                    Ok(entry) => entry,
                    Err(DriveError::Inaccessible) => continue,
                    Err(e) => return Err(e),
                };
                if entry.latest.as_ref().is_none_or(|r| r.id != revision) {
                    continue;
                }
                let record: Record = decoded(&self.search.records.get(&id).ok_or(DriveError::Internal)?)?;
                if record.revision != revision || record.extractor != EXTRACTOR_VERSION || !matches!(record.state, IndexState::Ready | IndexState::Partial) {
                    continue;
                }
                let section: Section = decoded(&self.search.sections.get(&(id, section)).ok_or(DriveError::Internal)?)?;
                let lower = section.text.to_lowercase();
                let at = lower.find(&page.anchor).unwrap_or(0);
                let mut start = at.saturating_sub(80).min(section.text.len());
                while !section.text.is_char_boundary(start) {
                    start -= 1;
                }
                let snippet = format!("{}{}", if start > 0 { "…" } else { "" }, section.text[start..].chars().take(320).collect::<String>());
                hits.push(ContentHit { entry, revision, location: section.location, snippet, partial: record.state == IndexState::Partial });
            }
            if source.peek().is_none() {
                page.space += 1;
                page.anchor.clear();
                page.after = ObjectId::MIN;
            }
        }
        // Empty bounded pages may still have a continuation; never return a leaked count.
        let next = if page.space < spaces.len() {
            if page.anchor.is_empty() {
                let space = spaces[page.space];
                page.anchor =
                    terms.iter().min_by_key(|t| self.search.counts.get(&TermKey(space, (*t).clone())).unwrap_or(0)).cloned().ok_or(DriveError::Internal)?;
            }
            Some(self.seal_bookmark(&page, b"content-search", actor.sponsor)?)
        } else {
            None
        };
        Ok(ContentPage { hits, next })
    }
}

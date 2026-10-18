//! Ordered metadata indexes. Each write touches one object, its parent and space;
//! listing seeks a B-tree instead of sorting an account-wide object graph.
use super::collaboration::decoded;
use super::*;
use ic_stable_structures::Storable;
use sovereign_cloud::drive::*;
use std::{
    borrow::Cow,
    ops::Bound::{Excluded, Included},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub(super) struct CatalogKey {
    // 0 = space, 1 = immediate parent. Prefixes are logical, not canister IDs.
    pub scope_kind: u8,
    pub scope: ObjectId,
    pub field: u8,
    pub rank: u8,
    pub number: u64,
    pub name: String,
    pub id: ObjectId,
}
impl Storable for CatalogKey {
    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Unbounded;
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        serde_json::to_vec(self).unwrap_or_else(|_| ic_cdk::trap("Invalid catalog key encoding")).into()
    }
    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| ic_cdk::trap("Invalid stored catalog key"))
    }
}
impl CatalogKey {
    pub fn position(&self) -> (u8, u64, &str, ObjectId) {
        (self.rank, self.number, &self.name, self.id)
    }
    pub fn for_object(object: &DriveObject, size: u64, scope_kind: u8, scope: ObjectId, field: u8) -> Self {
        Self {
            scope_kind,
            scope,
            field,
            rank: if scope_kind == 1 { u8::from(object.kind != ObjectKind::Folder) } else { 0 },
            number: match field {
                1 => object.updated_at,
                2 => size,
                _ => 0,
            },
            name: super::browse::normalized(&object.name),
            id: object.id,
        }
    }
    fn edge(scope_kind: u8, scope: ObjectId, field: u8, rank: u8, upper: bool) -> Self {
        Self {
            scope_kind,
            scope,
            field,
            rank,
            number: if upper { u64::MAX } else { 0 },
            name: if upper { char::MAX.to_string().repeat(161) } else { String::new() },
            id: if upper { ObjectId::MAX } else { ObjectId::MIN },
        }
    }
}
pub(super) fn keys(object: &DriveObject, size: u64) -> Vec<CatalogKey> {
    // Roots are navigation, not searchable files. Trash stays indexed and is
    // filtered using live ancestry; trashing a folder never rewrites descendants.
    let Some(parent) = object.parent else { return Vec::new() };
    (0..3).flat_map(|field| [CatalogKey::for_object(object, size, 0, object.space, field), CatalogKey::for_object(object, size, 1, parent, field)]).collect()
}
impl<M: Memory> Store<M> {
    pub(super) fn catalog_size(&self, id: ObjectId) -> DriveResult<u64> {
        self.projects.get(&id).map(|b| decoded::<Project>(&b).map(|p| p.revisions.last().map_or(0, |r| r.bytes))).unwrap_or(Ok(0))
    }
    /// Return an ordered iterator from a verified bookmark. Descending folder
    /// views reverse each rank independently so folders remain first.
    pub(super) fn catalog_iter(
        &self,
        kind: u8,
        scope: ObjectId,
        field: u8,
        descending: bool,
        after: Option<&CatalogKey>,
    ) -> Box<dyn Iterator<Item = CatalogKey> + '_> {
        let ranks = if kind == 1 { 2 } else { 1 };
        let mut iterators: Vec<Box<dyn Iterator<Item = CatalogKey> + '_>> = Vec::new();
        for rank in 0..ranks {
            if after.is_some_and(|a| rank < a.rank) {
                continue;
            }
            let low = CatalogKey::edge(kind, scope, field, rank, false);
            let high = CatalogKey::edge(kind, scope, field, rank, true);
            let bookmark = after.filter(|a| a.rank == rank).map(|a| CatalogKey { scope_kind: kind, scope, field, ..a.clone() });
            if descending {
                let end = bookmark.map(Excluded).unwrap_or(Included(high));
                iterators.push(Box::new(self.catalog_order.range((Included(low), end)).rev().map(|e| e.key().clone())));
            } else {
                let start = bookmark.map(Excluded).unwrap_or(Included(low));
                iterators.push(Box::new(self.catalog_order.range((start, Included(high))).map(|e| e.key().clone())));
            }
        }
        Box::new(iterators.into_iter().flatten())
    }
}

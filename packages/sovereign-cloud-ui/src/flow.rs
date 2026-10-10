//! Save routing shared with native regression tests; no identity or networking dependencies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SaveRoute {
    Name,
    Upload,
    AlreadySaved,
}

pub fn route(saved_document_revision: Option<u64>, current_revision: u64, copy: bool) -> SaveRoute {
    if copy || saved_document_revision.is_none() {
        SaveRoute::Name
    } else if saved_document_revision == Some(current_revision) {
        SaveRoute::AlreadySaved
    } else {
        SaveRoute::Upload
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_uses_cloud_snapshot_and_never_requests_a_second_confirmation() {
        assert_eq!(route(None, 3, false), SaveRoute::Name);
        let uploaded_snapshot = 3;
        assert_eq!(route(Some(uploaded_snapshot), 3, false), SaveRoute::AlreadySaved);
        // Edits while a snapshot uploads must still require a subsequent upload.
        assert_eq!(route(Some(uploaded_snapshot), 4, false), SaveRoute::Upload);
        // Saving locally does not alter the separate cloud snapshot revision.
        let locally_saved_revision = 4;
        assert_eq!(route(Some(uploaded_snapshot), locally_saved_revision, false), SaveRoute::Upload);
        assert_eq!(route(Some(4), 4, false), SaveRoute::AlreadySaved);
        // An explicit copy always requests a name, even for unchanged contents.
        assert_eq!(route(Some(4), 4, true), SaveRoute::Name);
    }
}

//! Pure workspace policy. The host never owns document bytes or authorization.
use crate::DocumentState;

#[derive(Default)]
pub struct Creation {
    baseline: Option<(u64, u64)>,
    abandoned: bool,
}
impl Creation {
    pub fn observe(&mut self, doc: &DocumentState) -> bool {
        if self.abandoned {
            return false;
        }
        match self.baseline {
            None => {
                self.baseline = Some((doc.id, doc.revision));
                false
            }
            Some((id, revision)) if id == doc.id => doc.revision != revision,
            Some(_) => {
                self.abandoned = true;
                false
            }
        }
    }
}

pub fn save_help(error: &str) -> (&'static str, bool) {
    let message = error.to_ascii_lowercase();
    if message.contains("limit") || message.contains("quota") || message.contains("64 mib") {
        (
            "Cloud capacity or item limit reached. Free space or contact the workspace owner. Keep a local copy using Export; retry after the limit is resolved.",
            false,
        )
    } else if message.contains("conflict") || message.contains("changed") {
        ("The cloud file changed. Keep this document open and save a separate copy to compare the versions.", false)
    } else {
        ("Your work remains in the editor. Retry this captured save; newer edits will be saved afterwards.", true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc(id: u64, revision: u64) -> DocumentState {
        DocumentState { id, revision, name: "Untitled".into(), dirty: true }
    }
    #[test]
    fn empty_launch_never_creates_a_file_and_new_tabs_do_not_inherit_destination() {
        let mut intent = Creation::default();
        assert!(!intent.observe(&doc(1, 3)));
        assert!(!intent.observe(&doc(1, 3)));
        assert!(intent.observe(&doc(1, 4)));
        assert!(!intent.observe(&doc(2, 4)));
        assert!(!intent.observe(&doc(1, 5)));
    }
    #[test]
    fn permanent_failures_do_not_suggest_blind_retry() {
        assert!(!save_help("Cloud file or folder limit reached").1);
        assert!(!save_help("Revision conflict").1);
        assert!(save_help("Network unavailable").1);
    }
}

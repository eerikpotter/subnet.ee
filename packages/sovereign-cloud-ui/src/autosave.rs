//! Pure scheduling policy. Upload, native encoding and browser persistence stay separate.
#[derive(Default)]
pub struct Schedule {
    observed: Option<(u64, u64)>,
    changed_at: f64,
    attempted: Option<(u64, u64)>,
    last_attempt: f64,
}
impl Schedule {
    pub fn due(&mut self, id: u64, revision: u64, now: f64, eligible: bool) -> bool {
        let value = (id, revision);
        if self.observed != Some(value) {
            self.observed = Some(value);
            self.changed_at = now;
        }
        if !eligible || self.attempted == Some(value) || now - self.changed_at < 5000.0 || now - self.last_attempt < 30000.0 {
            return false;
        }
        self.attempted = Some(value);
        self.last_attempt = now;
        true
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debounce_does_not_replay_a_failed_snapshot_or_cross_documents() {
        let mut s = Schedule::default();
        assert!(!s.due(1, 2, 100000., true));
        assert!(!s.due(1, 2, 104999., true));
        assert!(s.due(1, 2, 105000., true));
        assert!(!s.due(1, 2, 200000., true));
        assert!(!s.due(1, 3, 110000., true));
        assert!(!s.due(1, 3, 116000., true));
        assert!(s.due(1, 3, 135000., true));
        assert!(!s.due(2, 3, 180000., true));
        assert!(!s.due(2, 3, 185000., false));
        assert!(s.due(2, 3, 186000., true));
    }
}

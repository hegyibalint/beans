//! Shared model-version identity, independent of how models are stored.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(i64);

impl Revision {
    pub const fn new(value: i64) -> Self {
        assert!(value >= 0, "revision cannot be negative");
        Self(value)
    }

    pub const fn as_i64(self) -> i64 {
        self.0
    }

    pub fn advance(&mut self) -> Self {
        self.0 = self.0.checked_add(1).expect("revision overflow");
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::Revision;

    #[test]
    fn zero_and_the_maximum_revision_are_valid() {
        assert_eq!(Revision::default().as_i64(), 0);
        assert_eq!(Revision::new(i64::MAX).as_i64(), i64::MAX);
    }

    #[test]
    #[should_panic(expected = "revision cannot be negative")]
    fn negative_revisions_are_rejected() {
        Revision::new(-1);
    }

    #[test]
    fn advancing_can_reach_the_maximum_revision() {
        let mut revision = Revision::new(i64::MAX - 1);
        assert_eq!(revision.advance(), Revision::new(i64::MAX));
    }

    #[test]
    #[should_panic(expected = "revision overflow")]
    fn advancing_past_the_maximum_revision_is_rejected() {
        Revision::new(i64::MAX).advance();
    }

    #[test]
    fn advancing_returns_and_retains_the_next_revision() {
        let mut revision = Revision::new(4);

        assert_eq!(revision.advance(), Revision::new(5));
        assert_eq!(revision, Revision::new(5));
    }
}

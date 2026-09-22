pub mod storage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(u64);

impl Revision {
    pub const fn new(value: u64) -> Self {
        Self(value)
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
    fn advancing_returns_and_retains_the_next_revision() {
        let mut revision = Revision::new(4);

        assert_eq!(revision.advance(), Revision::new(5));
        assert_eq!(revision, Revision::new(5));
    }
}

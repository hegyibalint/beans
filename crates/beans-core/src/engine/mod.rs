pub mod storage;

use crate::model::classpath::Classpath;

/// Provides the revision and classpath for queries over stored models.
/// `None` leaves source discovery unrestricted; an empty classpath does not.
#[derive(Clone, Copy)]
pub struct QueryEnvironment<'a> {
    revision: Revision,
    classpath: Option<&'a Classpath>,
}

impl<'a> QueryEnvironment<'a> {
    pub fn new(revision: Revision, classpath: Option<&'a Classpath>) -> Self {
        Self {
            revision,
            classpath,
        }
    }

    pub fn revision(self) -> Revision {
        self.revision
    }

    pub fn classpath(self) -> Option<&'a Classpath> {
        self.classpath
    }
}

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
    use super::{QueryEnvironment, Revision};
    use crate::model::classpath::Classpath;

    #[test]
    fn query_environment_keeps_its_revision_and_borrowed_classpath() {
        let classpath = Classpath::default();
        let environment = QueryEnvironment::new(Revision::new(4), Some(&classpath));

        assert_eq!(environment.revision(), Revision::new(4));
        assert!(std::ptr::eq(environment.classpath().unwrap(), &classpath));
        assert!(
            QueryEnvironment::new(Revision::new(4), None)
                .classpath()
                .is_none()
        );
    }

    #[test]
    fn advancing_returns_and_retains_the_next_revision() {
        let mut revision = Revision::new(4);

        assert_eq!(revision.advance(), Revision::new(5));
        assert_eq!(revision, Revision::new(5));
    }
}

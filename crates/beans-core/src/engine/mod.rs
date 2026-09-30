pub mod storage;

use crate::model::classpath::Classpath;

/// Provides the revision and visibility policy for queries over stored models.
#[derive(Clone, Copy)]
pub struct QueryEnvironment<'a> {
    revision: Revision,
    classpath: &'a dyn Classpath,
}

impl<'a> QueryEnvironment<'a> {
    pub fn new(revision: Revision, classpath: &'a dyn Classpath) -> Self {
        Self {
            revision,
            classpath,
        }
    }

    pub fn revision(self) -> Revision {
        self.revision
    }

    pub fn classpath(self) -> &'a dyn Classpath {
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
    use crate::model::classpath::{Classpath, Unrestricted};

    #[test]
    fn query_environment_keeps_its_revision_and_borrowed_classpath() {
        let classpath = Unrestricted;
        let environment = QueryEnvironment::new(Revision::new(4), &classpath);

        assert_eq!(environment.revision(), Revision::new(4));
        assert!(std::ptr::eq(
            environment.classpath(),
            &classpath as &dyn Classpath
        ));
    }

    #[test]
    fn advancing_returns_and_retains_the_next_revision() {
        let mut revision = Revision::new(4);

        assert_eq!(revision.advance(), Revision::new(5));
        assert_eq!(revision, Revision::new(5));
    }
}

pub mod storage;

use crate::model::classpath::Classpath;

/// Selects which model versions and source origins a query can see.
/// This is a query boundary, not a language's lexical scope or an owned snapshot.
#[derive(Clone, Copy)]
pub struct QueryScope<'a> {
    revision: Revision,
    classpath: &'a dyn Classpath,
}

impl<'a> QueryScope<'a> {
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
    use super::{QueryScope, Revision};
    use crate::model::classpath::{Classpath, Unrestricted};

    #[test]
    fn query_scope_keeps_its_revision_and_borrowed_classpath() {
        let classpath = Unrestricted;
        let scope = QueryScope::new(Revision::new(4), &classpath);

        assert_eq!(scope.revision(), Revision::new(4));
        assert!(std::ptr::eq(
            scope.classpath(),
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

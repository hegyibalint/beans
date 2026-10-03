use crate::{classpath::Classpath, revision::Revision};

/// Selects which model versions and origins a query can see.
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

#[cfg(test)]
mod tests {
    use super::{QueryScope, Revision};
    use crate::classpath::{Classpath, Unrestricted};

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
}

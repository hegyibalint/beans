use crate::classpath::Classpath;

/// Selects which locations a query can see; the database transaction owns its snapshot.
#[derive(Clone, Copy)]
pub struct QueryScope<'a> {
    classpath: &'a dyn Classpath,
}

impl<'a> QueryScope<'a> {
    pub fn new(classpath: &'a dyn Classpath) -> Self {
        Self { classpath }
    }

    pub fn classpath(self) -> &'a dyn Classpath {
        self.classpath
    }
}

#[cfg(test)]
mod tests {
    use super::QueryScope;
    use crate::classpath::{Classpath, Unrestricted};

    #[test]
    fn query_scope_keeps_its_borrowed_classpath() {
        let classpath = Unrestricted;
        let scope = QueryScope::new(&classpath);
        assert!(std::ptr::eq(
            scope.classpath(),
            &classpath as &dyn Classpath
        ));
    }
}

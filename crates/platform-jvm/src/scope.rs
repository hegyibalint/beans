use beans_core::{Revision, storage::RevisionedStorage};

use crate::model::{Fqn, JvmClass, JvmSource};

/// What one source sees of the lake. Moot for now: every source sees
/// every class, found by full scan. Classpath filtering and ordering
/// belong here, below the language specs.
pub struct FileScope<'a> {
    class_lake: &'a RevisionedStorage<JvmSource, Vec<JvmClass>>,
    revision: Revision,
}

impl<'a> FileScope<'a> {
    pub(crate) fn new(
        class_lake: &'a RevisionedStorage<JvmSource, Vec<JvmClass>>,
        revision: Revision,
    ) -> FileScope<'a> {
        FileScope {
            class_lake,
            revision,
        }
    }

    pub fn class(&self, fqn: &Fqn) -> Option<&JvmClass> {
        self.classes().find(|class| class.fqn == *fqn)
    }

    /// Package in the binary-name sense: `p.Outer$Inner` lives in `p`,
    /// so nested classes come back too; languages filter by `enclosing`.
    pub fn package(&self, package: &str) -> impl Iterator<Item = &JvmClass> {
        self.classes()
            .filter(move |class| class.fqn.package() == package)
    }

    fn classes(&self) -> impl Iterator<Item = &JvmClass> {
        self.class_lake
            .iter_at(self.revision)
            .flat_map(|(_source, classes)| classes)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::PlatformJvm;
    use crate::model::JvmKind;

    use super::*;

    fn source(path: &str) -> JvmSource {
        JvmSource::SourceFile {
            path: PathBuf::from(path),
        }
    }

    fn class(fqn: &str) -> JvmClass {
        JvmClass {
            fqn: Fqn::new(fqn),
            kind: JvmKind::Class,
            enclosing: None,
            superclass: None,
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
        }
    }

    #[test]
    fn finds_a_class_by_fqn() {
        let mut revision = Revision::default();
        let mut platform = PlatformJvm::new();
        let first = revision.bump();
        platform.register(first, source("A.java"), vec![class("p.A")]);

        let scope = platform.scope(&source("A.java"), first);
        assert_eq!(scope.class(&Fqn::new("p.A")).unwrap().fqn, Fqn::new("p.A"));
        assert!(scope.class(&Fqn::new("p.B")).is_none());
    }

    #[test]
    fn package_gathers_classes_across_sources() {
        let mut revision = Revision::default();
        let mut platform = PlatformJvm::new();
        platform.register(revision.bump(), source("A.java"), vec![class("p.A")]);
        let second = revision.bump();
        platform.register(second, source("B.java"), vec![class("p.B"), class("q.C")]);

        let scope = platform.scope(&source("A.java"), second);
        let mut in_p: Vec<_> = scope.package("p").map(|class| class.fqn.clone()).collect();
        in_p.sort();
        assert_eq!(in_p, [Fqn::new("p.A"), Fqn::new("p.B")]);
    }

    #[test]
    fn scope_reads_the_lake_as_of_its_revision() {
        let mut revision = Revision::default();
        let mut platform = PlatformJvm::new();
        let first = revision.bump();
        platform.register(first, source("A.java"), vec![class("p.A")]);
        platform.register(revision.bump(), source("B.java"), vec![class("p.B")]);

        let scope = platform.scope(&source("A.java"), first);
        assert!(scope.class(&Fqn::new("p.B")).is_none());
    }
}

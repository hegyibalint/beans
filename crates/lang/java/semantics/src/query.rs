use beans_core_engine::{Revision, storage::RevisionedStorage};
use beans_core_model::{classpath::Classpath, source::Source};
use beans_lang_java_model::{
    File,
    declarations::{Declaration, DeclarationIndex, types::TypeDeclaration},
    references::NameRef,
};

#[derive(Debug, Clone, Copy)]
pub struct JavaTypeEntry<'a> {
    pub source: &'a Source,
    pub file: &'a File,
    pub declaration_index: DeclarationIndex,
    pub declaration: &'a TypeDeclaration,
}

pub struct JavaQuery<'a> {
    files: &'a RevisionedStorage<Source, File>,
    revision: Revision,
    classpath: &'a Classpath,
}

impl<'a> JavaQuery<'a> {
    pub fn new(
        files: &'a RevisionedStorage<Source, File>,
        revision: Revision,
        classpath: &'a Classpath,
    ) -> Self {
        Self {
            files,
            revision,
            classpath,
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn classpath(&self) -> &'a Classpath {
        self.classpath
    }

    /// Reads a stored model at this revision, without applying classpath visibility.
    pub fn file(&self, source: &Source) -> Option<&'a File> {
        self.files.get(self.revision, source)
    }

    /// Finds top-level types by qualified name (JLS §6.7, §7.6), without access checks.
    pub fn find_type(&self, name: &[String]) -> Vec<JavaTypeEntry<'a>> {
        let Some((simple_name, package_name)) = name.split_last() else {
            return Vec::new();
        };
        let mut candidates = Vec::new();

        for (source, file) in self.files.iter(self.revision) {
            let Source::SourceFile { path } = source else {
                continue;
            };
            if !self.classpath.contains_source_file(path) {
                continue;
            }
            let package = match &file.package_name {
                Some(NameRef::Simple(name)) => std::slice::from_ref(name),
                Some(NameRef::Qualified(components)) => components.as_slice(),
                None => &[],
            };
            if package != package_name {
                continue;
            }

            for entry in file.iter_declarations_in_scope(File::ROOT_SCOPE_ID) {
                let Declaration::Type(declaration) = entry.declaration else {
                    continue;
                };
                if declaration.name.as_ref() == Some(simple_name) {
                    candidates.push(JavaTypeEntry {
                        source,
                        file,
                        declaration_index: entry.declaration_index,
                        declaration,
                    });
                }
            }
        }
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core_model::classpath::ClasspathElement;

    fn name(name: &str) -> Vec<String> {
        name.split('.').map(str::to_owned).collect()
    }

    fn classpath(roots: &[&str]) -> Classpath {
        Classpath::new(roots.iter().map(|root| {
            ClasspathElement::new((*root).into(), [0; 32].into())
        }).collect())
    }

    #[test]
    fn file_reads_use_the_bound_revision_and_source_without_cloning() {
        let mut files = RevisionedStorage::default();
        let source = Source::SourceFile {
            path: "Example.java".into(),
        };
        let other = Source::SourceFile {
            path: "Other.java".into(),
        };
        let revision = Revision::new(2);
        files.put(revision, source.clone(), File::new());
        files.put(revision, other.clone(), File::new());
        files.put(Revision::new(5), source.clone(), File::new());
        let classpath = Classpath::default();
        let query = JavaQuery::new(&files, revision, &classpath);

        let expected = files.get(revision, &source).unwrap();
        let actual = query.file(&source).unwrap();
        assert!(std::ptr::eq(actual, expected));
        assert!(!std::ptr::eq(actual, query.file(&other).unwrap()));
        assert!(!std::ptr::eq(
            actual,
            files.get(Revision::new(5), &source).unwrap()
        ));
    }

    #[test]
    fn discovery_matches_declared_package_and_type_not_directory_layout() {
        let mut files = RevisionedStorage::default();
        let source = Source::SourceFile { path: "src/misplaced/Example.java".into() };
        let revision = Revision::new(1);
        files.put(revision, source.clone(), crate::lower_into("package p.q; public class Example {}"));
        let classpath = classpath(&["src"]);
        let query = JavaQuery::new(&files, revision, &classpath);
        let found = query.find_type(&name("p.q.Example"));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, &source);
        let Declaration::Type(stored) = found[0].file.declaration(found[0].declaration_index).unwrap() else {
            panic!("expected a type declaration");
        };
        assert!(std::ptr::eq(found[0].declaration, stored));
        for missing in ["p.Example", "p.q.example", "p.q.Other", "misplaced.Example"] {
            assert!(query.find_type(&name(missing)).is_empty());
        }
        assert!(query.find_type(&[]).is_empty());
    }

    #[test]
    fn discovery_retains_distinct_origins_without_duplicating_overlapping_roots() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        for path in ["src/a/Example.java", "src/b/Example.java", "other/Example.java"] {
            files.put(revision, Source::SourceFile { path: path.into() },
                crate::lower_into("package p; class Example {}"));
        }
        let classpath = classpath(&["src", "src/a"]);
        let query = JavaQuery::new(&files, revision, &classpath);
        let found = query.find_type(&name("p.Example"));

        assert_eq!(found.len(), 2);
        assert_ne!(found[0].source, found[1].source);
        assert!(found.iter().all(|entry| matches!(entry.source,
            Source::SourceFile { path } if path.starts_with("src"))));
    }

    #[test]
    fn discovery_does_not_search_members_or_non_source_origins() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        files.put(revision, Source::SourceFile { path: "src/Outer.java".into() },
            crate::lower_into("package p; class Outer { class Inner {} }"));
        files.put(revision, Source::ClassFile { path: "src/Example.class".into() },
            crate::lower_into("package p; class Example {}"));
        let classpath = classpath(&["src"]);
        let query = JavaQuery::new(&files, revision, &classpath);

        assert_eq!(query.find_type(&name("p.Outer")).len(), 1);
        for missing in ["p.Inner", "p.Outer.Inner", "p.Example"] {
            assert!(query.find_type(&name(missing)).is_empty());
        }
    }

    #[test]
    fn discovery_preserves_duplicate_declarations_within_one_origin() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        // Error recovery: duplicate top-level declarations remain distinct.
        files.put(revision, Source::SourceFile { path: "src/Example.java".into() },
            crate::lower_into("package p; class Example {} class Example {}"));
        let classpath = classpath(&["src"]);
        let query = JavaQuery::new(&files, revision, &classpath);
        let found = query.find_type(&name("p.Example"));

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].source, found[1].source);
        assert_ne!(found[0].declaration_index, found[1].declaration_index);
    }

    #[test]
    fn discovery_uses_the_bound_revision_after_replacement_and_deletion() {
        let mut files = RevisionedStorage::default();
        let source = Source::SourceFile { path: "src/Example.java".into() };
        files.put(Revision::new(1), source.clone(), crate::lower_into("package p; class Example {}"));
        files.put(Revision::new(2), source.clone(), crate::lower_into("package p; class Other {}"));
        files.remove(Revision::new(3), source);
        let classpath = classpath(&["src"]);
        let old = JavaQuery::new(&files, Revision::new(1), &classpath);
        let deleted = JavaQuery::new(&files, Revision::new(3), &classpath);

        assert_eq!(old.find_type(&name("p.Example")).len(), 1);
        assert!(deleted.find_type(&name("p.Example")).is_empty());
        assert!(deleted.find_type(&name("p.Other")).is_empty());
    }

    #[test]
    fn context_retains_the_supplied_revision_and_classpath() {
        let files = RevisionedStorage::default();
        let revision = Revision::new(7);
        let classpath = Classpath::default();
        let query = JavaQuery::new(&files, revision, &classpath);

        assert_eq!(query.revision(), revision);
        assert!(std::ptr::eq(query.classpath(), &classpath));
    }
}

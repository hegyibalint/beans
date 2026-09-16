mod arguments;
mod boundaries;
mod imports;
mod inheritance;
mod lexical;

use super::*;
use beans_core_engine::storage::RevisionedStorage;
use beans_core_model::classpath::{Classpath, ClasspathElement};

struct Fixture {
    files: RevisionedStorage<Source, File>,
    classpath: Classpath,
}

impl Fixture {
    fn new(sources: &[(&str, &str)]) -> Self {
        let mut files = RevisionedStorage::default();
        for (path, text) in sources {
            files.put(Revision::new(1), source(path), crate::lower_into(text));
        }
        Self {
            files,
            classpath: Classpath::new(vec![ClasspathElement::new("src".into(), [0; 32].into())]),
        }
    }

    fn query(&self) -> JavaQuery<'_> {
        JavaQuery::new(&self.files, Revision::new(1), &self.classpath)
    }

    fn field(&self, path: &str) -> Vec<ResolutionResult> {
        let source = source(path);
        let query = self.query();
        let file = query.file(&source).unwrap();
        let (node, reference) = file
            .iter_nodes()
            .find_map(|entry| match entry.node.kind() {
                NodeKind::Field(field) if field.name == "target" => {
                    Some((entry.index, &field.declared_type))
                }
                _ => None,
            })
            .expect("expected target field");
        Resolver {}.resolve(&ResolverContext::new(
            &source,
            node,
            reference,
            ReferenceLocation::Body,
            &query,
        ))
    }

    fn superclass(&self, path: &str, owner: &str) -> Vec<ResolutionResult> {
        let source = source(path);
        let query = self.query();
        let file = query.file(&source).unwrap();
        let (node, reference) = file
            .iter_nodes()
            .find_map(|entry| match entry.node.kind() {
                NodeKind::Type(declaration) if declaration.name.as_deref() == Some(owner) => {
                    Some((
                        entry.index,
                        declaration.declared_superclass.as_ref().unwrap(),
                    ))
                }
                _ => None,
            })
            .expect("expected superclass reference");
        Resolver {}.resolve(&ResolverContext::new(
            &source,
            node,
            reference,
            ReferenceLocation::Supertype,
            &query,
        ))
    }

    fn assert_type(&self, result: &ResolutionResult, path: &str, canonical: &str) {
        self.assert_declaration(&resolved(result).declaration, path, canonical);
    }

    fn assert_declaration(&self, declaration: &ResolvedDeclaration, path: &str, canonical: &str) {
        let ResolvedDeclaration::Java { declaration } = declaration else {
            panic!("expected a Java declaration, got {declaration:?}");
        };
        let query = self.query();
        let entry = query.declaration(declaration).unwrap();
        assert_eq!(entry.source, &source(path));
        let found = entry.file.find_type(&name(canonical));
        assert!(
            found.iter().any(|(index, _)| *index == entry.node_index),
            "expected {canonical}"
        );
    }
}

fn source(path: &str) -> Source {
    Source::SourceFile { path: path.into() }
}

fn name(name: &str) -> Name {
    name.split('.').map(str::to_owned).collect()
}

fn resolved(result: &ResolutionResult) -> &ResolvedType {
    let Ok(Resolution::Resolved(resolved)) = result else {
        panic!("expected resolution, got {result:?}");
    };
    resolved
}

fn assert_parameter<'a>(result: &'a ResolutionResult, parameter: &str) -> &'a DeclarationHandle {
    let ResolvedDeclaration::Parameter { declaration, name } = &resolved(result).declaration else {
        panic!("expected parameter, got {result:?}");
    };
    assert_eq!(name, parameter);
    declaration
}

fn example() -> Fixture {
    Fixture::new(&[
        (
            "src/app/Foo.java",
            include_str!("../../examples/resolution/app/Foo.java"),
        ),
        (
            "src/app/Base.java",
            include_str!("../../examples/resolution/app/Base.java"),
        ),
        (
            "src/library/Base.java",
            include_str!("../../examples/resolution/library/Base.java"),
        ),
    ])
}

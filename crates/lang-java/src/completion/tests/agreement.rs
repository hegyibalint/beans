//! Completion promises one thing: that it offers what resolution would find.
//! Everything else it does is a way of computing that set cheaply.
//!
//! Every other file here checks one rule. This one checks that the two code
//! paths cannot answer differently, which is the property the whole framing
//! exists to buy — and the property a refactor is most likely to break.

use super::*;

use crate::resolution::{TypeResolution, resolve_type_name};

/// A fixture with all four stages in play, plus a name contested across two of
/// them, so agreement is asserted over something with structure in it.
fn contested() -> Workspace {
    Workspace::of(&[
        (
            "p/Test.java",
            "package p;
import q.Imported;
import q.Contested;
class Test {
    class Member {}
    <cur> field;
}
",
        ),
        ("p/Sibling.java", "package p;\nclass Sibling {}\n"),
        ("p/Contested.java", "package p;\nclass Contested {}\n"),
        ("q/Imported.java", "package q;\npublic class Imported {}\n"),
        (
            "q/Contested.java",
            "package q;\npublic class Contested {}\n",
        ),
    ])
    .compiled("jdk/java/lang/String.class", "java.lang.String")
}

impl Workspace {
    /// The same question resolution is asked, at the same point.
    fn resolve(&self, name: &str) -> TypeResolution {
        let file = self.java.model_at(&self.caret, self.revision).unwrap();
        let query = Query::new(
            jvm::query::Query::new(&self.jvm, jvm::query::ScopeQuery::unscoped(), self.revision),
            &self.java,
        );
        let point = Point::at(&self.caret, file, self.offset, &self.text, &query).unwrap();

        resolve_type_name(
            &model::Name::Simple(model::Identifier {
                text: name.to_string(),
                span: point.replace,
            }),
            &self.caret,
            file,
            point.at.scope,
            &query,
        )
    }
}

#[test]
fn every_offered_name_resolves() {
    let workspace = contested();
    let items = workspace.complete();
    assert!(
        items.len() >= 5,
        "the fixture stopped exercising the stages"
    );

    for item in &items {
        // §7.5.1 is the one stage allowed to disagree. An import is a name the
        // user wrote rather than one we propose, so it is offered whether or not
        // anything answers it; `imports.rs` pins that on its own.
        if is_imported(&workspace, &item.label) {
            continue;
        }

        assert!(
            matches!(
                workspace.resolve(&item.label),
                TypeResolution::Resolved(_) | TypeResolution::Ambiguous(_)
            ),
            "completion offered {:?} and resolution reaches nothing",
            item.label
        );
    }
}

#[test]
fn every_offered_name_resolves_to_what_the_item_says() {
    let workspace = contested();

    for item in &workspace.complete() {
        let TypeResolution::Resolved(target) = workspace.resolve(&item.label) else {
            continue;
        };
        let Some(detail) = &item.detail else { continue };

        assert_eq!(
            &label_of(&workspace, &target),
            detail,
            "completion and resolution disagree about which {:?} wins",
            item.label
        );
    }
}

/// The contested name is the point of the fixture: two declarations, one row,
/// and both halves have to name the same winner.
#[test]
fn a_contested_name_is_won_by_the_same_declaration_on_both_sides() {
    let workspace = contested();
    let items = workspace.complete();

    assert_eq!(
        items.iter().filter(|i| i.label == "Contested").count(),
        1,
        "one name is one row"
    );
    let TypeResolution::Resolved(target) = workspace.resolve("Contested") else {
        panic!("the import should have settled it");
    };
    assert_eq!(
        item(&items, "Contested").detail.as_deref(),
        Some(label_of(&workspace, &target).as_str())
    );
}

fn is_imported(workspace: &Workspace, name: &str) -> bool {
    let file = workspace
        .java
        .model_at(&workspace.caret, workspace.revision)
        .unwrap();
    file.imports.iter().any(|import| {
        import.kind == model::ImportKind::Type
            && import
                .name
                .segments()
                .last()
                .is_some_and(|s| s.text == name)
    })
}

fn label_of(workspace: &Workspace, target: &crate::resolution::TypeTarget) -> String {
    let query = Query::new(
        jvm::query::Query::new(
            &workspace.jvm,
            jvm::query::ScopeQuery::unscoped(),
            workspace.revision,
        ),
        &workspace.java,
    );
    match target {
        crate::resolution::TypeTarget::Parsed {
            source,
            declaration,
        } => query
            .model_of(source)
            .unwrap()
            .declaration_label(*declaration)
            .unwrap(),
        crate::resolution::TypeTarget::Compiled { fqn, .. } => fqn.to_string(),
    }
}

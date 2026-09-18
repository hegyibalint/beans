use beans_lang_java_model::{File, NodeEntry, nodes::NodeIndex};

use super::{ResolutionFailure, model::TypeCandidate};

pub(super) fn iter_enclosing_types(
    file: &File,
    node: NodeIndex,
) -> impl Iterator<Item = TypeCandidate<'_>> + '_ {
    file.iter_ancestors(node).filter_map(type_candidate)
}

pub(super) fn iter_declared_member_types(
    file: &File,
    owner: NodeIndex,
) -> impl Iterator<Item = TypeCandidate<'_>> + '_ {
    let owner_node = file.node(owner).expect("member type owner must exist");
    assert!(
        owner_node.kind().as_type().is_some(),
        "member type owner must be a type declaration"
    );

    file.iter_children(owner).filter_map(type_candidate)
}

/// This is the semantic entry point for member type traversal.
///
/// JLS §§8.5 and 9.5 also include inherited member types. Until supertypes can
/// be resolved to `TypeCandidate`s, this iterator exposes declared members only.
pub(super) fn iter_member_types(
    file: &File,
    owner: NodeIndex,
) -> impl Iterator<Item = Result<TypeCandidate<'_>, ResolutionFailure>> + '_ {
    iter_declared_member_types(file, owner).map(Ok)
}

fn type_candidate(entry: NodeEntry<'_>) -> Option<TypeCandidate<'_>> {
    let declaration = entry.node.kind().as_type()?;
    Some(TypeCandidate::new(entry.index, declaration))
}

#[cfg(test)]
mod tests {
    use beans_lang_java_model::{names::Name, nodes::NodeKind};

    use super::*;

    fn type_index(file: &File, components: &[&str]) -> NodeIndex {
        let name = Name::new(
            components
                .iter()
                .map(|component| (*component).to_owned())
                .collect(),
        );
        let matches = file.find_type(&name);
        assert_eq!(matches.len(), 1);
        matches[0].0
    }

    #[test]
    fn enclosing_types_are_nearest_first() {
        let file = crate::lower_into(
            "class Outer { class Inner { class Deep { int value; } } class Sibling {} }",
        );
        let deep = type_index(&file, &["Outer", "Inner", "Deep"]);
        let field = file
            .iter_children(deep)
            .find(|entry| matches!(entry.node.kind(), NodeKind::Field(_)))
            .expect("expected a field");

        let names: Vec<_> = iter_enclosing_types(&file, field.index)
            .map(|candidate| candidate.declaration().name.as_deref())
            .collect();

        assert_eq!(names, [Some("Deep"), Some("Inner"), Some("Outer")]);
    }

    #[test]
    fn declared_member_types_are_direct_and_ordered() {
        let file = crate::lower_into(
            "class Outer { class First {} interface Second {} int ignored; class Branch { class Deep {} } }",
        );
        let outer = type_index(&file, &["Outer"]);

        let members: Vec<_> = iter_declared_member_types(&file, outer)
            .map(|candidate| {
                (
                    candidate.node_index(),
                    candidate.declaration().name.as_deref(),
                )
            })
            .collect();

        assert_eq!(members.len(), 3);
        assert_eq!(
            members.iter().map(|(_, name)| *name).collect::<Vec<_>>(),
            [Some("First"), Some("Second"), Some("Branch")]
        );
        assert!(members.iter().all(|(index, _)| *index != outer));
    }

    #[test]
    #[should_panic(expected = "member type owner must be a type declaration")]
    fn declared_member_types_reject_a_non_type_owner() {
        let file = crate::lower_into("class Example {}");

        let _ = iter_declared_member_types(&file, File::ROOT_NODE_ID).next();
    }
}

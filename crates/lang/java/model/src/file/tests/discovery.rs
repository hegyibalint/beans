use crate::{
    File,
    names::Name,
    nodes::{
        NodeIndex, NodeKind,
        fields::FieldDeclaration,
        types::{AccessLevel, Kind, TypeDeclaration},
    },
    references::{PrimitiveType, TypeNameComponent, TypeRef},
};

fn name(value: &str) -> Name {
    value.split('.').map(str::to_owned).collect()
}

fn add_type(file: &mut File, parent: NodeIndex, name: &str) -> NodeIndex {
    let mut declaration = TypeDeclaration::new(Kind::Class);
    declaration.name = Some(name.into());
    file.add_node(parent, NodeKind::Type(declaration))
}

#[test]
fn package_and_member_components_lead_to_the_stored_declaration() {
    let mut file = File::new();
    file.package_name = name("a.b");
    let outer = add_type(&mut file, File::ROOT_NODE_ID, "C");
    let member = add_type(&mut file, outer, "D");
    let nested_member = add_type(&mut file, member, "E");
    let sibling = add_type(&mut file, File::ROOT_NODE_ID, "Sibling");
    add_type(&mut file, sibling, "D");
    add_type(&mut file, sibling, "OnlyInSibling");

    for (path, expected) in [
        ("a.b.C", outer),
        ("a.b.C.D", member),
        ("a.b.C.D.E", nested_member),
    ] {
        let found = file.find_type(&name(path));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, expected);
        let NodeKind::Type(stored) = file.node(expected).unwrap().kind() else {
            panic!("expected a type declaration");
        };
        assert!(std::ptr::eq(found[0].1, stored));
    }
    for missing in [
        "a",
        "a.b",
        "a.b.c",
        "a.bc.C",
        "b.C",
        "C",
        "a.b.D",
        "a.b.C.E",
        "a.b.C.D.Missing",
        "a.b.C.OnlyInSibling",
        "a.b.C..D",
        "a.b.C.D.",
    ] {
        assert!(file.find_type(&name(missing)).is_empty(), "{missing}");
    }
    assert!(file.find_type(&Name::default()).is_empty());
}

#[test]
fn malformed_package_names_cannot_match_even_an_identical_prefix() {
    let mut file = File::new();
    add_type(&mut file, File::ROOT_NODE_ID, "Example");
    file.package_name = name("p..q");
    assert!(file.find_type(&name("p..q.Example")).is_empty());
}

#[test]
fn single_component_packages_are_matched_exactly() {
    let mut file = File::new();
    file.package_name = name("p");
    let expected = add_type(&mut file, File::ROOT_NODE_ID, "Example");
    assert_eq!(file.find_type(&name("p.Example"))[0].0, expected);
    for missing in ["Example", "pp.Example", "q.Example", "p"] {
        assert!(file.find_type(&name(missing)).is_empty());
    }
}

#[test]
fn unnamed_packages_have_no_package_prefix() {
    let mut file = File::new();
    let outer = add_type(&mut file, File::ROOT_NODE_ID, "Outer");
    let member = add_type(&mut file, outer, "Member");
    assert_eq!(file.find_type(&name("Outer"))[0].0, outer);
    assert_eq!(file.find_type(&name("Outer.Member"))[0].0, member);
    assert!(file.find_type(&name("p.Outer")).is_empty());
    assert!(file.find_type(&Name::default()).is_empty());
}

#[test]
fn duplicate_declarations_remain_distinct_at_every_depth() {
    let mut file = File::new();
    let first = add_type(&mut file, File::ROOT_NODE_ID, "Outer");
    let second = add_type(&mut file, File::ROOT_NODE_ID, "Outer");
    let first_member = add_type(&mut file, first, "Member");
    let duplicate_member = add_type(&mut file, first, "Member");
    let second_member = add_type(&mut file, second, "Member");
    let outer_indices: Vec<_> = file
        .find_type(&name("Outer"))
        .iter()
        .map(|(index, _)| *index)
        .collect();
    assert_eq!(outer_indices, [first, second]);
    let member_indices: Vec<_> = file
        .find_type(&name("Outer.Member"))
        .iter()
        .map(|(index, _)| *index)
        .collect();
    assert_eq!(
        member_indices,
        [first_member, duplicate_member, second_member]
    );
}

#[test]
fn inherited_members_are_not_part_of_the_subclass_canonical_path() {
    let mut file = File::new();
    let base = add_type(&mut file, File::ROOT_NODE_ID, "Base");
    let member = add_type(&mut file, base, "Member");
    let mut subclass = TypeDeclaration::new(Kind::Class);
    subclass.name = Some("Subclass".into());
    subclass.declared_superclass = Some(TypeRef::Named {
        segments: vec![TypeNameComponent {
            name: "Base".into(),
            bounds: vec![],
        }],
    });
    file.add_node(File::ROOT_NODE_ID, NodeKind::Type(subclass));
    assert_eq!(file.find_type(&name("Base.Member"))[0].0, member);
    assert!(file.find_type(&name("Subclass.Member")).is_empty());
}

#[test]
fn capitalization_does_not_determine_the_package_boundary() {
    let mut file = File::new();
    file.package_name = name("a.B");
    let outer = add_type(&mut file, File::ROOT_NODE_ID, "c");
    let expected = add_type(&mut file, outer, "d");
    assert_eq!(file.find_type(&name("a.B.c.d"))[0].0, expected);
}

#[test]
fn non_type_and_unnamed_declarations_do_not_match() {
    let mut file = File::new();
    file.add_node(
        File::ROOT_NODE_ID,
        NodeKind::Field(FieldDeclaration {
            name: "Field".into(),
            declared_type: TypeRef::Primitive(PrimitiveType::Int),
        }),
    );
    let anonymous = file.add_node(
        File::ROOT_NODE_ID,
        NodeKind::Type(TypeDeclaration::new(Kind::Class)),
    );
    add_type(&mut file, anonymous, "Hidden");
    add_type(&mut file, File::ROOT_NODE_ID, "");
    for missing in ["Field", "Hidden", "", ".Hidden"] {
        assert!(file.find_type(&name(missing)).is_empty());
    }
}

#[test]
fn discovery_includes_inaccessible_types_of_every_kind() {
    let mut file = File::new();
    let outer = add_type(&mut file, File::ROOT_NODE_ID, "Outer");
    for (kind, name) in [
        (Kind::Class, "MemberClass"),
        (Kind::Interface, "MemberInterface"),
        (Kind::AnnotationInterface, "MemberAnnotation"),
        (Kind::Enum, "MemberEnum"),
        (Kind::Record, "MemberRecord"),
    ] {
        let mut declaration = TypeDeclaration::new(kind);
        declaration.name = Some(name.into());
        declaration.access.push(AccessLevel::Private);
        let expected = file.add_node(outer, NodeKind::Type(declaration));
        let found = file.find_type(&Name::new(vec!["Outer".into(), name.into()]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, expected);
    }
}

#[test]
fn block_contained_types_and_their_members_have_no_canonical_path() {
    let mut file = File::new();
    let outer = add_type(&mut file, File::ROOT_NODE_ID, "Outer");
    let block = file.add_node(outer, NodeKind::Block);
    let local = add_type(&mut file, block, "Local");
    add_type(&mut file, local, "Member");
    for missing in ["Local", "Outer.Local", "Outer.Local.Member"] {
        assert!(file.find_type(&name(missing)).is_empty());
    }
}

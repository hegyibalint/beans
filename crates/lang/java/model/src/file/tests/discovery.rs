use crate::{
    File,
    declarations::{
        Declaration, DeclarationIndex,
        fields::FieldDeclaration,
        types::{AccessLevel, Kind, TypeDeclaration},
    },
    names::Name,
    references::{PrimitiveType, TypeNameComponent, TypeRef},
    scopes::{ScopeIndex, ScopeKind},
};

fn name(value: &str) -> Vec<String> {
    value.split('.').map(str::to_owned).collect()
}

fn add_type(file: &mut File, scope: ScopeIndex, name: &str) -> (DeclarationIndex, ScopeIndex) {
    let mut declaration = TypeDeclaration::new(Kind::Class);
    declaration.name = Some(name.into());
    let index = file.add_declaration(scope, Declaration::Type(declaration));
    let body = file.new_child_scope(scope, ScopeKind::TypeBody { owner: index });
    (index, body)
}

#[test]
fn package_and_member_components_lead_to_the_stored_declaration() {
    let mut file = File::new();
    file.package_name = Name::new(name("a.b"));
    let (outer, outer_body) = add_type(&mut file, File::ROOT_SCOPE_ID, "C");
    let (member, member_body) = add_type(&mut file, outer_body, "D");
    let (nested_member, _) = add_type(&mut file, member_body, "E");
    let (_, sibling_body) = add_type(&mut file, File::ROOT_SCOPE_ID, "Sibling");
    add_type(&mut file, sibling_body, "D");
    add_type(&mut file, sibling_body, "OnlyInSibling");

    for (path, expected) in [
        ("a.b.C", outer),
        ("a.b.C.D", member),
        ("a.b.C.D.E", nested_member),
    ] {
        let found = file.find_type(&name(path));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, expected);
        let Declaration::Type(stored) = file.declaration(expected).unwrap() else {
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
    assert!(file.find_type(&[]).is_empty());
}

#[test]
fn malformed_package_names_cannot_match_even_an_identical_prefix() {
    let mut file = File::new();
    add_type(&mut file, File::ROOT_SCOPE_ID, "Example");

    file.package_name = Name::new(name("p..q"));

    assert!(file.find_type(&name("p..q.Example")).is_empty());
}

#[test]
fn single_component_packages_are_matched_exactly() {
    let mut file = File::new();
    file.package_name = Name::new(vec!["p".into()]);
    let (expected, _) = add_type(&mut file, File::ROOT_SCOPE_ID, "Example");

    assert_eq!(file.find_type(&name("p.Example"))[0].0, expected);
    for missing in ["Example", "pp.Example", "q.Example", "p"] {
        assert!(file.find_type(&name(missing)).is_empty());
    }
}

#[test]
fn unnamed_packages_have_no_package_prefix() {
    let mut file = File::new();
    let (outer, body) = add_type(&mut file, File::ROOT_SCOPE_ID, "Outer");
    let (member, _) = add_type(&mut file, body, "Member");

    assert_eq!(file.find_type(&name("Outer"))[0].0, outer);
    assert_eq!(file.find_type(&name("Outer.Member"))[0].0, member);
    assert!(file.find_type(&name("p.Outer")).is_empty());
    assert!(file.find_type(&[]).is_empty());
}

#[test]
fn duplicate_declarations_remain_distinct_at_every_depth() {
    let mut file = File::new();
    let (first, first_body) = add_type(&mut file, File::ROOT_SCOPE_ID, "Outer");
    let (second, second_body) = add_type(&mut file, File::ROOT_SCOPE_ID, "Outer");
    let (first_member, _) = add_type(&mut file, first_body, "Member");
    let (duplicate_member, _) = add_type(&mut file, first_body, "Member");
    let (second_member, _) = add_type(&mut file, second_body, "Member");

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
    let (_, base_body) = add_type(&mut file, File::ROOT_SCOPE_ID, "Base");
    let (member, _) = add_type(&mut file, base_body, "Member");
    let mut subclass = TypeDeclaration::new(Kind::Class);
    subclass.name = Some("Subclass".into());
    subclass.declared_superclass = Some(TypeRef::Named {
        segments: vec![TypeNameComponent {
            name: "Base".into(),
            bounds: vec![],
        }],
    });
    file.add_declaration(File::ROOT_SCOPE_ID, Declaration::Type(subclass));

    assert_eq!(file.find_type(&name("Base.Member"))[0].0, member);
    assert!(file.find_type(&name("Subclass.Member")).is_empty());
}

#[test]
fn capitalization_does_not_determine_the_package_boundary() {
    let mut file = File::new();
    file.package_name = Name::new(name("a.B"));
    let (_, body) = add_type(&mut file, File::ROOT_SCOPE_ID, "c");
    let (expected, _) = add_type(&mut file, body, "d");

    assert_eq!(file.find_type(&name("a.B.c.d"))[0].0, expected);
}

#[test]
fn non_type_and_unnamed_declarations_do_not_match() {
    let mut file = File::new();
    file.add_declaration(
        File::ROOT_SCOPE_ID,
        Declaration::Field(FieldDeclaration {
            name: "Field".into(),
            declared_type: TypeRef::Primitive(PrimitiveType::Int),
        }),
    );
    let anonymous = file.add_declaration(
        File::ROOT_SCOPE_ID,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );
    let body = file.new_child_scope(
        File::ROOT_SCOPE_ID,
        ScopeKind::TypeBody { owner: anonymous },
    );
    add_type(&mut file, body, "Hidden");
    add_type(&mut file, File::ROOT_SCOPE_ID, "");

    for missing in ["Field", "Hidden", "", ".Hidden"] {
        assert!(file.find_type(&name(missing)).is_empty());
    }
}

#[test]
fn discovery_includes_inaccessible_types_of_every_kind() {
    let mut file = File::new();
    let (_, body) = add_type(&mut file, File::ROOT_SCOPE_ID, "Outer");
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
        let expected = file.add_declaration(body, Declaration::Type(declaration));
        let found = file.find_type(&["Outer".into(), name.into()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, expected);
    }
}

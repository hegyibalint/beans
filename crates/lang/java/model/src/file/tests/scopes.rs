use crate::{
    File,
    declarations::{
        Declaration,
        types::{Kind, TypeDeclaration},
    },
    scopes::{ScopeIndex, ScopeKind},
};

#[test]
fn scopes_from_walks_ancestors_including_start_and_root_but_not_siblings() {
    let mut file = File::new();
    let outer = file.add_declaration(
        File::ROOT_SCOPE_ID,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );
    let outer_scope =
        file.new_child_scope(File::ROOT_SCOPE_ID, ScopeKind::TypeBody { owner: outer });
    let inner = file.add_declaration(
        outer_scope,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );
    let inner_scope = file.new_child_scope(outer_scope, ScopeKind::TypeBody { owner: inner });
    let sibling = file.add_declaration(
        outer_scope,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );
    file.new_child_scope(outer_scope, ScopeKind::TypeBody { owner: sibling });

    let entries: Vec<_> = file.iter_scopes_from(inner_scope).collect();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.scope_index)
            .collect::<Vec<_>>(),
        [inner_scope, outer_scope, File::ROOT_SCOPE_ID]
    );
    for entry in entries {
        assert!(std::ptr::eq(
            entry.scope,
            file.scope(entry.scope_index).unwrap()
        ));
    }
}

#[test]
fn new_child_scope_links_parent_child_and_owner() {
    let mut file = File::new();
    let declaration = file.add_declaration(
        File::ROOT_SCOPE_ID,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );

    let child = file.new_child_scope(
        File::ROOT_SCOPE_ID,
        ScopeKind::TypeBody { owner: declaration },
    );

    assert_eq!(
        file.scope(child).unwrap().parent_scope(),
        Some(File::ROOT_SCOPE_ID)
    );
    assert_eq!(
        file.scope(child).unwrap().kind(),
        ScopeKind::TypeBody { owner: declaration }
    );
    assert_eq!(
        file.scope(File::ROOT_SCOPE_ID)
            .unwrap()
            .iter_child_scopes()
            .collect::<Vec<_>>(),
        [child]
    );
}

#[test]
#[should_panic(expected = "invalid parent scope index")]
fn new_child_scope_rejects_an_unknown_parent() {
    let mut file = File::new();
    let declaration = file.add_declaration(
        File::ROOT_SCOPE_ID,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );

    file.new_child_scope(
        ScopeIndex::new(10),
        ScopeKind::TypeBody { owner: declaration },
    );
}

#[test]
fn scoped_iteration_exposes_only_the_selected_scopes_declarations() {
    let mut file = File::new();
    let owner = file.add_declaration(
        File::ROOT_SCOPE_ID,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );
    let body = file.new_child_scope(File::ROOT_SCOPE_ID, ScopeKind::TypeBody { owner });
    let member = file.add_declaration(body, Declaration::Type(TypeDeclaration::new(Kind::Class)));

    let mut entries = file.iter_declarations_in_scope(body);
    let entry = entries.next().expect("expected the member declaration");
    assert!(entries.next().is_none());
    assert_eq!(entry.scope_index, body);
    assert_eq!(entry.declaration_index, member);
    assert!(std::ptr::eq(entry.scope, file.scope(body).unwrap()));
    assert!(std::ptr::eq(
        entry.declaration,
        file.declaration(member).unwrap()
    ));

    assert_eq!(
        file.iter_declarations()
            .map(|entry| (entry.scope_index, entry.declaration_index))
            .collect::<Vec<_>>(),
        [(File::ROOT_SCOPE_ID, owner), (body, member)]
    );
    for entry in file.iter_scopes() {
        assert!(std::ptr::eq(
            entry.scope,
            file.scope(entry.scope_index).unwrap()
        ));
    }
}

#[test]
#[should_panic(expected = "invalid scope index")]
fn scoped_iteration_rejects_an_unknown_scope() {
    let file = File::new();
    let _ = file.iter_declarations_in_scope(ScopeIndex::new(10));
}

#[test]
fn add_declaration_registers_it_with_its_scope() {
    let mut file = File::new();
    let declaration = Declaration::Type(TypeDeclaration::new(Kind::Class));

    let declaration = file.add_declaration(File::ROOT_SCOPE_ID, declaration);

    let mut entries = file.iter_declarations();
    let entry = entries.next().expect("expected one declaration");
    assert!(entries.next().is_none());
    assert_eq!(entry.scope_index, File::ROOT_SCOPE_ID);
    assert_eq!(entry.declaration_index, declaration);
    assert!(std::ptr::eq(
        entry.scope,
        file.scope(entry.scope_index).unwrap()
    ));
    assert!(std::ptr::eq(
        entry.declaration,
        file.declaration(entry.declaration_index).unwrap()
    ));
    assert_eq!(
        file.iter_declarations_in_scope(File::ROOT_SCOPE_ID)
            .map(|entry| entry.declaration_index)
            .collect::<Vec<_>>(),
        [declaration]
    );
}

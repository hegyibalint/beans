//! The facade as a consumer sees it.
//!
//! These tests stand in for "a future non-LSP consumer depends on
//! `beans` and performs basic indexing plus go-to-definition without
//! importing `beans-lsp`" (issue #6, acceptance criterion). They use
//! only the [`beans::Workspace`] surface — no graph or registry poking,
//! no LSP types — to prove the facade owns enough of the workspace story
//! to stand on its own.

use std::path::Path;

use beans::Workspace;

#[test]
fn index_and_resolve_within_a_file() {
    let mut ws = Workspace::new();
    let path = Path::new("/tmp/beans-facade/Dog.java");
    let source = "package com.example;\n\
        public class Dog {\n\
        \x20   public String getName() { return null; }\n\
        }\n";
    ws.update_file(path, source);

    // Cursor on the `Dog` class name resolves to its declaration.
    let def = ws
        .definition_at(path, 1, 13)
        .expect("Dog should resolve to a definition");
    assert_eq!(def.file.as_ref(), path);
    assert_eq!(def.start_line, 1);
}

#[test]
fn go_to_definition_across_files() {
    // The headline criterion: a consumer indexes two files and jumps
    // from a use in one to the declaration in the other — through the
    // facade alone.
    let mut ws = Workspace::new();
    let model = Path::new("/tmp/beans-facade/model/User.java");
    let service = Path::new("/tmp/beans-facade/service/UserService.java");

    ws.update_file(
        model,
        "package com.example.model;\npublic class User {\n    public String getName() { return null; }\n}\n",
    );
    ws.update_file(
        service,
        "package com.example.service;\n\
         import com.example.model.User;\n\
         public class UserService {\n\
         \x20   public User findUser() { return null; }\n\
         }\n",
    );

    // `User` on the return-type line of UserService (line 3) jumps to
    // the declaration in User.java.
    let def = ws
        .definition_at(service, 3, 11)
        .expect("User should resolve across files via the import");
    assert_eq!(
        def.file.as_ref(),
        model,
        "definition should land in the declaring file"
    );
    assert_eq!(def.start_line, 1, "User is declared on line 1 of User.java");
}

#[test]
fn removing_a_file_unindexes_its_symbols() {
    let mut ws = Workspace::new();
    let a = Path::new("/tmp/beans-facade/A.java");
    let b = Path::new("/tmp/beans-facade/B.java");
    ws.update_file(
        a,
        "package com.example;\npublic class A {\n    public B make() { return null; }\n}\n",
    );
    ws.update_file(b, "package com.example;\npublic class B {}\n");

    // `B` resolves while it is indexed.
    assert!(
        ws.definition_at(a, 2, 11).is_some(),
        "B should resolve while indexed"
    );

    ws.remove_file(b);

    // After removal the use no longer resolves.
    assert!(
        ws.definition_at(a, 2, 11).is_none(),
        "B should not resolve after its file is removed"
    );
}

#[test]
fn outline_lists_a_files_members() {
    let mut ws = Workspace::new();
    let path = Path::new("/tmp/beans-facade/Account.java");
    ws.update_file(
        path,
        "package com.example;\n\
         public class Account {\n\
         \x20   private long balance;\n\
         \x20   public long balance() { return balance; }\n\
         }\n",
    );

    let symbols = ws.document_symbols(path);
    assert_eq!(symbols.len(), 1, "one top-level type");
    assert_eq!(symbols[0].name, "Account");
    let members: Vec<&str> = symbols[0]
        .children
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(members.contains(&"balance"), "field present in outline");
}

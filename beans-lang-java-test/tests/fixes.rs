//! Quick-fix behavior tests — tier 1 (library fixtures).
//!
//! These pin *tool behavior* of analysis-layer [`beans::Fix`]
//! values — which fixes are offered and what applying them yields —
//! not JLS claims, so they live outside the `spec/` tree. The spec
//! facts they lean on (single-type-import semantics, the
//! `missing-import` diagnostic) are tested in
//! `spec/jls07_packages.rs`.
//!
//! Tier 2 — LSP lifecycle role-play (didOpen → publishDiagnostics →
//! codeAction → apply → didChange → republish) — does NOT belong
//! here: those tests live in `beans-lsp`'s own suite, driving the
//! in-process tower-lsp service (backlog #038).

mod prelude;

fn fixture() -> beans_test_harness::fixture::Fixture {
    prelude::fixture()
}

// ----- missing-import quick fix -----
//
// The fix inserts a single-type-import declaration for an unresolved
// type use whose simple name matches a workspace type. One fix per
// candidate FQN, labeled "Import '<fqn>'". v1 placement policy:
// directly after the package statement, blank-line separated —
// sorting/grouping is organize-imports' job, later.
mod missing_import {
    use super::*;

    #[test]
    fn inserts_import_after_package() {
        // The anchored run pins text, position, and blank-line
        // separation in one assertion.
        fixture()
            .file(
                "com/example/model/Service.java",
                r#"
                package com.example.model;
                public class Service {}
            "#,
            )
            .file(
                "com/example/app/App.java",
                r#"
                package com.example.app;

                public class App {
                    private Ser<cur>vice service;
                }
            "#,
            )
            .quick_fix_default()
            .apply("Import 'com.example.model.Service'")
            .expect_lines(&[
                "package com.example.app;",
                "",
                "import com.example.model.Service;",
            ])
            .run();
    }

    #[test]
    fn offers_one_action_per_candidate() {
        // Ambiguous simple name: one fix per candidate FQN, selected
        // by label. Applying the alpha fix imports alpha.
        fixture()
            .file(
                "com/alpha/Service.java",
                r#"
                package com.alpha;
                public class Service {}
            "#,
            )
            .file(
                "com/beta/Service.java",
                r#"
                package com.beta;
                public class Service {}
            "#,
            )
            .file(
                "com/example/app/App.java",
                r#"
                package com.example.app;

                public class App {
                    private Ser<cur>vice service;
                }
            "#,
            )
            .quick_fix_default()
            .apply("Import 'com.alpha.Service'")
            .expect_lines(&["package com.example.app;", "", "import com.alpha.Service;"])
            .run();
    }

    #[test]
    fn inserts_before_existing_imports() {
        // KISS placement holds even with an existing import block:
        // the new import still lands right after the package line,
        // and the existing import survives untouched.
        fixture()
            .file(
                "com/example/model/Service.java",
                r#"
                package com.example.model;
                public class Service {}
            "#,
            )
            .file(
                "com/example/model/Existing.java",
                r#"
                package com.example.model;
                public class Existing {}
            "#,
            )
            .file(
                "com/example/app/App.java",
                r#"
                package com.example.app;

                import com.example.model.Existing;

                public class App {
                    private Existing existing;
                    private Ser<cur>vice service;
                }
            "#,
            )
            .quick_fix_default()
            .apply("Import 'com.example.model.Service'")
            .expect_lines(&[
                "package com.example.app;",
                "",
                "import com.example.model.Service;",
            ])
            .expect_lines(&["import com.example.model.Existing;"])
            .run();
    }
}

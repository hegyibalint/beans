use std::path::Path;

use crate::parser::{ParseError, parse};

use super::project;

#[test]
fn module_keys_are_ids_in_deterministic_order() {
    let project = project("[module.lib]\n[module.app-main]\n[module.\"app.test\"]\n");

    assert_eq!(
        project
            .modules
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["app-main", "app.test", "lib"]
    );
}

#[test]
fn omitted_lists_are_empty_and_the_project_jdk_is_optional() {
    let project = project("[module.app]\n");
    let module = &project.modules["app"];

    assert!(module.sources.is_empty());
    assert!(module.depends_on.is_empty());
    assert_eq!(project.jdk_home, None);
}

#[test]
fn an_empty_descriptor_has_no_modules_or_jdk() {
    let project = project("");

    assert!(project.modules.is_empty());
    assert_eq!(project.jdk_home, None);
}

#[test]
fn dependency_ids_are_preserved() {
    let project = project("[module.app]\ndepends_on = [\"lib\"]\n[module.lib]\n");

    assert_eq!(project.modules["app"].depends_on, ["lib"]);
}

#[test]
fn unknown_dependencies_name_both_modules_in_the_error() {
    let error = parse(
        "[module.app]\ndepends_on = [\"missing\"]\n",
        Path::new("/project"),
    )
    .unwrap_err();

    assert!(
        matches!(&error, ParseError::UnknownDependency { module, dependency }
        if module == "app" && dependency == "missing")
    );
    assert!(error.to_string().contains("app"));
    assert!(error.to_string().contains("missing"));
}

#[test]
fn unknown_fields_and_removed_configuration_are_rejected() {
    for contents in [
        "modules = {}\n",
        "[module.app]\nsauces = [\"src\"]\n",
        "[unit.app]\n",
        "[module.app]\njdk_home = \"jdk\"\n",
        "[module.app]\nclasspath = [\"lib.jar\"]\n",
    ] {
        let error = parse(contents, Path::new("/project")).unwrap_err();

        assert!(matches!(error, ParseError::Toml(_)), "{contents}");
    }
}

#[test]
fn duplicate_module_tables_are_rejected() {
    // TOML 1.1.0, Table: a table cannot be defined more than once.
    // https://toml.io/en/v1.1.0#table
    assert!(matches!(
        parse("[module.app]\n[module.app]\n", Path::new("/project")),
        Err(ParseError::Toml(_))
    ));
}

#[test]
fn wrong_field_types_are_rejected() {
    for contents in [
        "jdk_home = 26\n",
        "[module.app]\nsources = \"src\"\n",
        "[module.app]\ndepends_on = [1]\n",
    ] {
        assert!(
            matches!(
                parse(contents, Path::new("/project")),
                Err(ParseError::Toml(_))
            ),
            "{contents}"
        );
    }
}

#[test]
fn syntax_errors_keep_the_toml_location_and_cause() {
    use std::error::Error;

    let error = parse("[module.app\n", Path::new("/project")).unwrap_err();
    let ParseError::Toml(toml) = &error else {
        panic!("expected a TOML error")
    };

    assert!(toml.span().is_some());
    assert!(error.source().is_some());
}

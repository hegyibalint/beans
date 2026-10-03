use std::path::Path;

use crate::parser::{ParseError, parse};

use super::project;

#[test]
fn unit_keys_are_ids_in_deterministic_order() {
    let project = project("[unit.lib]\n[unit.app-main]\n[unit.\"app.test\"]\n");

    assert_eq!(
        project.units.keys().map(String::as_str).collect::<Vec<_>>(),
        ["app-main", "app.test", "lib"]
    );
}

#[test]
fn omitted_lists_are_empty_and_no_jdk_is_invented() {
    let project = project("[unit.app]\n");
    let unit = &project.units["app"];

    assert!(unit.sources.is_empty());
    assert!(unit.depends_on.is_empty());
    assert!(unit.classpath.is_empty());
    assert_eq!(unit.jdk_home, None);
}

#[test]
fn an_empty_descriptor_has_no_units() {
    assert!(project("").units.is_empty());
}

#[test]
fn dependency_ids_are_preserved() {
    let project = project("[unit.app]\ndepends_on = [\"lib\"]\n[unit.lib]\n");

    assert_eq!(project.units["app"].depends_on, ["lib"]);
}

#[test]
fn unknown_dependencies_name_both_units_in_the_error() {
    let error = parse(
        "[unit.app]\ndepends_on = [\"missing\"]\n",
        Path::new("/project"),
    )
    .unwrap_err();

    assert!(
        matches!(&error, ParseError::UnknownDependency { unit, dependency }
        if unit == "app" && dependency == "missing")
    );
    assert!(error.to_string().contains("app"));
    assert!(error.to_string().contains("missing"));
}

#[test]
fn unknown_fields_are_rejected_at_both_levels() {
    for contents in ["units = {}\n", "[unit.app]\nsauces = [\"src\"]\n"] {
        let error = parse(contents, Path::new("/project")).unwrap_err();

        assert!(matches!(error, ParseError::Toml(_)), "{contents}");
    }
}

#[test]
fn duplicate_unit_tables_are_rejected() {
    // TOML 1.1.0, Table: a table cannot be defined more than once.
    // https://toml.io/en/v1.1.0#table
    assert!(matches!(
        parse("[unit.app]\n[unit.app]\n", Path::new("/project")),
        Err(ParseError::Toml(_))
    ));
}

#[test]
fn wrong_field_types_are_rejected() {
    for contents in [
        "jdk_home = 26\n",
        "[unit.app]\nsources = \"src\"\n",
        "[unit.app]\ndepends_on = [1]\n",
        "[unit.app]\nclasspath = [false]\n",
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

    let error = parse("[unit.app\n", Path::new("/project")).unwrap_err();
    let ParseError::Toml(toml) = &error else {
        panic!("expected a TOML error")
    };

    assert!(toml.span().is_some());
    assert!(error.source().is_some());
}

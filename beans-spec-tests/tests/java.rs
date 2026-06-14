//! Java facade spec tests, organized by JLS chapter, plus Java
//! fix-behavior tests. Exercised through the composed `beans` facade
//! and the fixture harness (ADR-0032). Vertical-local parser/model
//! unit tests stay in `beans-lang-java/tests/`.

#[path = "support/prelude.rs"]
mod prelude;

#[path = "java/jls04_types.rs"]
mod jls04_types;
#[path = "java/jls06_names.rs"]
mod jls06_names;
#[path = "java/jls07_packages.rs"]
mod jls07_packages;
#[path = "java/jls08_classes.rs"]
mod jls08_classes;
#[path = "java/jls09_interfaces.rs"]
mod jls09_interfaces;
#[path = "java/jls10_arrays.rs"]
mod jls10_arrays;
#[path = "java/jls14_statements.rs"]
mod jls14_statements;
#[path = "java/jls15_expressions.rs"]
mod jls15_expressions;

#[path = "java/fixes.rs"]
mod fixes;

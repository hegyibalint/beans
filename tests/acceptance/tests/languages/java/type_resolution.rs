// Type resolution is one observable feature assembled from rules across the JLS,
// so each file below is one premise: the thing that has to hold for its cases to
// be about anything.
//
// A pending expectation carries the reason it is pending, and that reason is part
// of the assertion; when it stops being true the marker is wrong, even while the
// test still passes. Where a whole rule is out of reach we keep the citation and
// the claims we would make, rather than tests that all fail for one reason.

mod inaccessible_types;
mod module_imports;
mod multi_unit;
mod package_type_boundary;
mod qualified_type_names;
mod scope_of_declarations;
mod shadowing;
mod simple_type_names;
mod single_type_imports;
mod static_type_imports;
mod type_imports_on_demand;

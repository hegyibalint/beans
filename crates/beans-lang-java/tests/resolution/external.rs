use std::path::Path;

use beans_core::{
    artifact_sources::ArtifactSources,
    classpath::Classpath,
    location::{Location, LocationEntry},
    location_match::LocationMatcher,
    module::Module,
    ranges::ByteRange,
    resource_filter::ResourceFilter,
};
use beans_engine::{Engine, ImportKind};
use beans_lang_java::{
    model::nodes::NodeKind,
    query as java_query,
    semantics::resolution::{
        Context, ResolutionFailure, TypeCandidate, TypeLocator, query as resolution_query, resolve,
        resolve_supertype,
    },
};
use beans_testing::{TempDir, support::class_model};

use super::support::Models;

#[derive(Clone, Copy)]
enum Occurrence {
    Field,
    Supertype,
}

struct Fixture {
    java: Models,
    source: Location,
    engine: Engine,
    files: TempDir,
}

impl Fixture {
    fn new(current: &str, other_files: &[(&str, &str)]) -> Self {
        let mut fixture = Self::from_sources("/src/Use.java", &[("/src/Use.java", current)]);
        for (name, text) in other_files {
            fixture.java.store(
                fixture.java.process(text),
                Location::file(format!("/src/{name}.java")),
            );
        }
        fixture
    }

    fn from_sources(source: &str, sources: &[(&str, &str)]) -> Self {
        let mut java = Models::new();
        for (path, contents) in sources {
            java.store(java.process(contents), Location::file(*path));
        }
        let files = TempDir::new();
        let engine = Engine::new(files.root()).unwrap();
        Self {
            java,
            source: Location::file(source),
            engine,
            files,
        }
    }

    fn competing_targets() -> Self {
        let fixture = Self::from_sources(
            "/consumer/Use.java",
            &[
                ("/consumer/Use.java", "class Use { Target field; }"),
                ("/sourceset-a/Target.java", "class Target {}"),
                ("/sourceset-b/Target.java", "class Target {}"),
            ],
        );
        let locations: Vec<_> = fixture
            .prospective_candidates("Target")
            .into_iter()
            .map(|candidate| candidate.location().clone())
            .collect();
        assert_eq!(
            locations,
            [
                Location::file("/sourceset-a/Target.java"),
                Location::file("/sourceset-b/Target.java"),
            ]
        );
        fixture
    }

    fn resolve(&self) -> Result<TypeCandidate, ResolutionFailure> {
        self.resolve_at(Occurrence::Field)
    }

    fn resolve_at(&self, occurrence: Occurrence) -> Result<TypeCandidate, ResolutionFailure> {
        let modules = self.engine.modules().unwrap();
        let matcher = LocationMatcher::new(&modules);
        let file = java_query::file(&self.source, &self.java.connection)
            .unwrap()
            .unwrap();
        let (index, reference) = file
            .iter_nodes()
            .find_map(|entry| match (occurrence, entry.node.kind()) {
                (Occurrence::Field, NodeKind::Field(field)) => {
                    Some((entry.index, field.declared_type.value()))
                }
                (Occurrence::Supertype, NodeKind::Type(declaration))
                    if declaration.name() == Some("Use") =>
                {
                    declaration
                        .declared_superclass
                        .as_ref()
                        .map(|reference| (entry.index, reference.value()))
                }
                _ => None,
            })
            .unwrap();
        let ctx = Context::new(&file, &self.source, index, reference)
            .with_connection(&self.java.connection)
            .with_location_matcher(Some(&matcher));
        match occurrence {
            Occurrence::Field => resolve(&ctx),
            Occurrence::Supertype => resolve_supertype(&ctx),
        }
    }

    fn set_modules(&self, modules: Vec<Module>) {
        self.engine
            .register_modules(modules, ImportKind::Toml, self.files.root())
            .unwrap();
    }

    fn prospective_candidates(&self, name: &str) -> Vec<TypeCandidate> {
        resolution_query::find_types(
            &name.split('.').map(str::to_owned).collect(),
            &self.java.connection,
        )
        .unwrap()
    }

    fn expected(&self, name: &str) -> TypeCandidate {
        let entry = java_query::find_type(
            &name.split('.').map(str::to_owned).collect(),
            &self.java.connection,
        )
        .unwrap()
        .into_iter()
        .next()
        .expect("expected a stored Java type");
        let declaration = java_query::declaration(&entry, &self.java.connection)
            .unwrap()
            .unwrap();
        TypeCandidate::new(
            TypeLocator::Declaration(entry),
            declaration.name.unwrap().range(),
        )
    }
}

#[test]
fn unmatched_sources_resolve_without_classpath_restrictions() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![module(&["/other"], Some(&[]))]);

    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(
            fixture.prospective_candidates("Target")
        ))
    );
}

#[test]
fn matching_empty_classpaths_hide_external_targets() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![module(&["/consumer"], Some(&[]))]);

    assert_eq!(fixture.resolve(), Err(ResolutionFailure::NotFound));
}

#[test]
fn matching_unrestricted_classpaths_admit_external_targets() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![module(&["/consumer"], None)]);

    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(
            fixture.prospective_candidates("Target")
        ))
    );
}

#[test]
fn dependency_on_sourceset_a_resolves_its_target() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![module(&["/consumer"], Some(&["/sourceset-a"]))]);

    assert_eq!(
        fixture.resolve().unwrap().location(),
        &Location::file("/sourceset-a/Target.java")
    );
}

#[test]
fn dependency_on_sourceset_b_resolves_its_target() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![module(&["/consumer"], Some(&["/sourceset-b"]))]);

    assert_eq!(
        fixture.resolve().unwrap().location(),
        &Location::file("/sourceset-b/Target.java")
    );
}

#[test]
fn dependencies_on_both_sourcesets_make_target_ambiguous() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![module(
        &["/consumer"],
        Some(&["/sourceset-a", "/sourceset-b"]),
    )]);

    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(
            fixture.prospective_candidates("Target")
        ))
    );
}

#[test]
fn empty_entries_still_admit_other_files_in_the_owning_inputs() {
    let fixture = Fixture::from_sources(
        "/consumer/Use.java",
        &[
            ("/consumer/Use.java", "class Use { Target field; }"),
            ("/consumer/Target.java", "class Target {}"),
            ("/other/Target.java", "class Target {}"),
        ],
    );
    fixture.set_modules(vec![module(&["/consumer"], Some(&[]))]);

    assert_eq!(
        fixture.resolve().unwrap().location(),
        &Location::file("/consumer/Target.java")
    );
}

#[test]
fn classpath_entries_do_not_select_owning_classpaths() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![
        module(&["/consumer"], Some(&[])),
        module(&["/sourceset-a"], Some(&["/consumer", "/sourceset-b"])),
    ]);

    assert_eq!(fixture.resolve(), Err(ResolutionFailure::NotFound));
}

#[test]
fn unrelated_unrestricted_classpaths_do_not_bypass_matching_restrictions() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![
        module(&["/consumer"], Some(&[])),
        module(&["/other"], None),
    ]);

    assert_eq!(fixture.resolve(), Err(ResolutionFailure::NotFound));
}

#[test]
fn overlapping_classpaths_union_visibility_within_and_across_modules() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![
        module(&["/consumer"], Some(&["/sourceset-a"])),
        module(&["/consumer"], Some(&["/sourceset-b"])),
    ]);
    let expected = Err(ResolutionFailure::Ambiguous(
        fixture.prospective_candidates("Target"),
    ));
    assert_eq!(fixture.resolve(), expected);

    let mut modules = fixture.engine.modules().unwrap();
    let second = modules.pop().unwrap();
    modules[0].classpaths.extend(second.classpaths);
    fixture.set_modules(modules);
    assert_eq!(fixture.resolve(), expected);
}

#[test]
fn a_matching_unrestricted_classpath_overrides_matching_restrictions() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![
        module(&["/consumer"], Some(&[])),
        module(&["/consumer"], None),
    ]);

    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(
            fixture.prospective_candidates("Target")
        ))
    );
}

#[test]
fn missing_artifacts_connect_to_selected_sources_independently_of_module_order() {
    let files = TempDir::new();
    let jar = files.path("api.jar");
    let classes = files.path("classes");
    let artifacts = [jar.to_str().unwrap(), classes.to_str().unwrap()];
    for artifact in artifacts {
        let mut fixture = Fixture::competing_targets();
        fixture.java.store(
            fixture.java.process("class Target {}"),
            Location::file("/sourceset-a/Excluded.java"),
        );
        fixture.set_modules(vec![
            module(&["/consumer"], Some(&[artifact])),
            Module {
                classpaths: vec![],
                artifact_sources: vec![
                    artifact_sources(&artifacts, &["/sourceset-a/Target.java"]),
                    artifact_sources(&["/other.jar"], &["/sourceset-b/Target.java"]),
                ],
            },
        ]);

        assert!(!Path::new(artifact).exists());
        for _ in 0..2 {
            assert_eq!(
                fixture.resolve().unwrap().location(),
                &Location::file("/sourceset-a/Target.java")
            );
            let mut modules = fixture.engine.modules().unwrap();
            modules.reverse();
            fixture.set_modules(modules);
        }
    }
}

#[test]
fn producing_sources_do_not_export_their_own_compilation_dependencies() {
    let fixture = Fixture::competing_targets();
    let mut producer = module(&["/sourceset-a"], Some(&["/sourceset-b"]));
    producer.artifact_sources = vec![artifact_sources(
        &["/api.jar"],
        &["/sourceset-a/Target.java"],
    )];
    fixture.set_modules(vec![module(&["/consumer"], Some(&["/api.jar"])), producer]);

    assert_eq!(
        fixture.resolve().unwrap().location(),
        &Location::file("/sourceset-a/Target.java")
    );
}

#[test]
fn consuming_two_artifacts_preserves_genuine_ambiguity() {
    let fixture = Fixture::competing_targets();
    fixture.set_modules(vec![
        module(&["/consumer"], Some(&["/a.jar", "/b.jar"])),
        Module {
            classpaths: vec![],
            artifact_sources: vec![
                artifact_sources(&["/a.jar"], &["/sourceset-a/Target.java"]),
                artifact_sources(&["/b.jar"], &["/sourceset-b/Target.java"]),
            ],
        },
    ]);

    let mut expected = fixture.prospective_candidates("Target");
    expected[0].provenance.via = Some(Location::file("/a.jar"));
    expected[1].provenance.via = Some(Location::file("/b.jar"));
    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(expected))
    );
}

#[test]
fn source_archive_associations_preserve_the_outer_archive_identity() {
    let mut fixture = Fixture::new("class Use { Target field; }", &[]);
    let target =
        Location::file("/api-sources.jar").entry(LocationEntry::Container("Target.java".into()));
    for archive in ["/api-sources.jar", "/other-sources.jar"] {
        fixture.java.store(
            fixture.java.process("class Target {}"),
            Location::file(archive).entry(LocationEntry::Container("Target.java".into())),
        );
    }
    let mut module = module(&["/src/Use.java"], Some(&["/api.jar"]));
    module.artifact_sources = vec![artifact_sources(&["/api.jar"], &["/api-sources.jar"])];
    fixture.set_modules(vec![module]);

    assert_eq!(fixture.resolve().unwrap().location(), &target);
}

#[test]
fn artifact_sources_apply_at_every_external_lookup_tier() {
    for (current, package) in [
        (
            "package p; import q.Target; class Use { Target field; }",
            "q",
        ),
        ("package q; class Use { Target field; }", "q"),
        ("package p; import q.*; class Use { Target field; }", "q"),
        ("package p; class Use { Target field; }", "java.lang"),
        ("package p; class Use { q.Target field; }", "q"),
    ] {
        let declaration = format!("package {package}; public class Target {{}}");
        let fixture = Fixture::new(current, &[("Target", &declaration)]);
        let mut module = module(&["/src/Use.java"], Some(&["/api.jar"]));
        module.artifact_sources = vec![artifact_sources(&["/api.jar"], &["/src/Target.java"])];
        fixture.set_modules(vec![module]);

        let mut expected = fixture.expected(&format!("{package}.Target"));
        expected.provenance.via = Some(Location::file("/api.jar"));
        assert_eq!(fixture.resolve(), Ok(expected), "{current}");
    }
}

#[test]
fn artifact_sources_apply_to_imported_members_and_qualified_prefixes() {
    for (current, expected_name) in [
        (
            "package p; import q.Outer; class Use { Outer.Inner field; }",
            Some("q.Outer.Inner"),
        ),
        (
            "package p; class Use { q.Outer.Inner field; }",
            Some("q.Outer.Inner"),
        ),
        (
            "package p; import q.Outer; class Use { Outer.Missing field; }",
            None,
        ),
        ("package p; class Use { q.Outer.Missing field; }", None),
    ] {
        let fixture = Fixture::new(
            current,
            &[(
                "Outer",
                "package q; public class Outer { public class Inner {} }",
            )],
        );
        let mut module = module(&["/src/Use.java"], Some(&["/api.jar"]));
        module.artifact_sources = vec![artifact_sources(&["/api.jar"], &["/src/Outer.java"])];
        fixture.set_modules(vec![module]);

        let expected = expected_name
            .map(|name| {
                let mut candidate = fixture.expected(name);
                candidate.provenance.via = Some(Location::file("/api.jar"));
                candidate
            })
            .ok_or(ResolutionFailure::Partial);
        assert_eq!(fixture.resolve(), expected, "{current}");
    }
}

#[test]
fn single_import_shadows_current_package_and_on_demand_imports() {
    let fixture = Fixture::new(
        "package p; import q.Foo; import r.*; class Use { Foo field; }",
        &[
            ("Same", "package p; class Foo {}"),
            ("Direct", "package q; public class Foo {}"),
            ("Demand", "package r; public class Foo {}"),
        ],
    );
    assert_eq!(fixture.resolve(), Ok(fixture.expected("q.Foo")));
}

#[test]
fn current_package_shadows_on_demand_imports() {
    let fixture = Fixture::new(
        "package p; import q.*; class Use { Foo field; }",
        &[
            ("Same", "package p; class Foo {}"),
            ("Demand", "package q; public class Foo {}"),
        ],
    );
    assert_eq!(fixture.resolve(), Ok(fixture.expected("p.Foo")));
}

#[test]
fn distinct_on_demand_imports_are_ambiguous_but_duplicate_imports_are_not() {
    let files = &[
        ("First", "package q; public class Foo {}"),
        ("Second", "package r; public class Foo {}"),
    ];
    let ambiguous = Fixture::new(
        "package p; import q.*; import r.*; class Use { Foo field; }",
        files,
    );
    let Err(ResolutionFailure::Ambiguous(candidates)) = ambiguous.resolve() else {
        panic!("expected ambiguous on-demand imports");
    };
    assert_eq!(
        candidates,
        [ambiguous.expected("q.Foo"), ambiguous.expected("r.Foo")]
    );

    let duplicate = Fixture::new(
        "package p; import q.*; import q.*; class Use { Foo field; }",
        files,
    );
    assert_eq!(duplicate.resolve(), Ok(duplicate.expected("q.Foo")));
}

#[test]
fn a_qualified_suffix_does_not_disambiguate_on_demand_imports() {
    let fixture = Fixture::new(
        "package p; import q.*; import r.*; class Use { Outer.Inner field; }",
        &[
            (
                "First",
                "package q; public class Outer { public class Inner {} }",
            ),
            ("Second", "package r; public class Outer {}"),
        ],
    );
    let Err(ResolutionFailure::Ambiguous(candidates)) = fixture.resolve() else {
        panic!("expected two competing Outer types");
    };
    assert_eq!(
        candidates,
        [fixture.expected("q.Outer"), fixture.expected("r.Outer")]
    );
}

#[test]
fn java_lang_is_imported_on_demand() {
    let fixture = Fixture::new(
        "package p; class Use { String field; }",
        &[("String", "package java.lang; public class String {}")],
    );
    assert_eq!(fixture.resolve(), Ok(fixture.expected("java.lang.String")));
}

#[test]
fn imported_qualified_name_resolves_the_member_and_commits_to_its_owner() {
    let fixture = Fixture::new(
        "package p; import q.Outer; class Use { Outer.Inner field; }",
        &[(
            "Outer",
            "package q; public class Outer { public class Inner {} }",
        )],
    );
    assert_eq!(fixture.resolve(), Ok(fixture.expected("q.Outer.Inner")));

    let missing = Fixture::new(
        "package p; import q.Outer; class Use { Outer.Missing field; }",
        &[("Outer", "package q; public class Outer {}")],
    );
    assert_eq!(missing.resolve(), Err(ResolutionFailure::Partial));
}

#[test]
fn qualified_spelling_can_start_with_a_package() {
    let fixture = Fixture::new(
        "package p; class Use { q.Outer.Inner field; }",
        &[(
            "Outer",
            "package q; public class Outer { public class Inner {} }",
        )],
    );
    assert_eq!(fixture.resolve(), Ok(fixture.expected("q.Outer.Inner")));
}

#[test]
fn imported_superclass_uses_the_same_external_query() {
    let fixture = Fixture::new(
        "package p; import q.Base; class Use extends Base {}",
        &[("Base", "package q; public class Base {}")],
    );
    let mut module = module(&["/src/Use.java"], Some(&["/api.jar"]));
    module.artifact_sources = vec![artifact_sources(&["/api.jar"], &["/src/Base.java"])];
    fixture.set_modules(vec![module]);
    let mut expected = fixture.expected("q.Base");
    expected.provenance.via = Some(Location::file("/api.jar"));
    assert_eq!(fixture.resolve_at(Occurrence::Supertype), Ok(expected));
}

#[test]
fn stored_jvm_classes_are_resolved() {
    let location = Location::file("/Foo.class");
    let mut fixture = Fixture::new("package p; import q.Foo; class Use { Foo field; }", &[]);
    fixture.java.store_jvm(&class_model("q/Foo"), &location);
    assert_eq!(fixture.resolve(), Ok(jvm_candidate(location)));
}

#[test]
fn binary_classpath_entries_still_admit_jar_contents_without_source_associations() {
    let jar = "/api.jar";
    let location = Location::file(jar).entry(LocationEntry::Container("q/Foo.class".into()));
    let mut fixture = Fixture::new("package p; import q.Foo; class Use { Foo field; }", &[]);
    fixture.java.store_jvm(&class_model("q/Foo"), &location);
    fixture.set_modules(vec![module(&["/src/Use.java"], Some(&[]))]);
    assert_eq!(fixture.resolve(), Err(ResolutionFailure::NotFound));

    fixture.set_modules(vec![module(&["/src/Use.java"], Some(&[jar]))]);
    assert_eq!(fixture.resolve(), Ok(jvm_candidate(location)));
}

#[test]
fn a_jvm_single_import_shadows_a_java_type_in_the_current_package() {
    let location = Location::file("/Foo.class");
    let mut fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package p; class Foo {}")],
    );
    fixture.java.store_jvm(&class_model("q/Foo"), &location);
    assert_eq!(fixture.resolve(), Ok(jvm_candidate(location)));
}

#[test]
fn java_and_jvm_candidates_with_the_same_name_are_ambiguous() {
    let location = Location::file("/Foo.class");
    let mut fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package q; public class Foo {}")],
    );
    fixture.java.store_jvm(&class_model("q/Foo"), &location);
    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(vec![
            fixture.expected("q.Foo"),
            jvm_candidate(location),
        ]))
    );
}

#[test]
fn an_unmatched_name_is_not_found() {
    let fixture = Fixture::new("package p; class Use { Missing field; }", &[]);
    assert_eq!(fixture.resolve(), Err(ResolutionFailure::NotFound));
}

#[test]
fn two_artifact_paths_to_one_declaration_are_ambiguous() {
    let fixture = Fixture::new(
        "import q.Target; class Use { Target field; }",
        &[("Target", "package q; public class Target {}")],
    );
    fixture.set_modules(vec![
        module(
            &["/src/Use.java"],
            Some(&["/producer/artifact-a", "/producer/artifact-b"]),
        ),
        Module {
            classpaths: vec![],
            artifact_sources: vec![
                artifact_sources(&["/producer/artifact-a"], &["/src/Target.java"]),
                artifact_sources(&["/producer/artifact-b"], &["/src/Target.java"]),
            ],
        },
    ]);
    let mut first = fixture.expected("q.Target");
    let mut second = first.clone();
    first.provenance.via = Some(Location::file("/producer/artifact-a"));
    second.provenance.via = Some(Location::file("/producer/artifact-b"));

    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(vec![first, second]))
    );
}

#[test]
fn repeated_imports_and_associations_do_not_duplicate_candidate_pairs() {
    let fixture = Fixture::new(
        "import q.Target; import q.Target; class Use { Target field; }",
        &[("Target", "package q; public class Target {}")],
    );
    fixture.set_modules(vec![
        module(&["/src/Use.java"], Some(&["/producer/artifact"])),
        Module {
            classpaths: vec![],
            artifact_sources: vec![
                artifact_sources(&["/producer/artifact"], &["/src/Target.java"]),
                artifact_sources(&["/producer/artifact"], &["/src/Target.java"]),
            ],
        },
    ]);
    let mut expected = fixture.expected("q.Target");
    expected.provenance.via = Some(Location::file("/producer/artifact"));

    assert_eq!(fixture.resolve(), Ok(expected));
}

fn jvm_candidate(location: Location) -> TypeCandidate {
    TypeCandidate::new(TypeLocator::Jvm(location), ByteRange::new(0, 0))
}

fn module(inputs: &[&str], entries: Option<&[&str]>) -> Module {
    Module {
        classpaths: vec![Classpath {
            inputs: filters(inputs),
            entries: entries.map(filters),
        }],
        artifact_sources: vec![],
    }
}

fn artifact_sources(artifacts: &[&str], sources: &[&str]) -> ArtifactSources {
    ArtifactSources {
        artifacts: artifacts.iter().map(|path| Location::file(*path)).collect(),
        sources: filters(sources),
    }
}

fn filters(paths: &[&str]) -> Vec<ResourceFilter> {
    paths.iter().map(ResourceFilter::file).collect()
}

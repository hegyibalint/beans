use beans_core::{
    artifact_sources::ArtifactSources, classpath::Classpath, location::Location,
    location_match::LocationMatch, module::Module, ranges::ByteRange,
    resource_filter::ResourceFilter, source::SourceSpan,
};
use beans_engine::{Engine, ImportKind};
use beans_testing::TempDir;

#[test]
fn matches_sources_through_classpath_artifacts() {
    let (_files, engine) = engine(
        &["/producer/artifact"],
        &[("/producer/artifact", "/producer/source")],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    let matches = engine
        .match_locations(
            &Location::file("/consumer/input"),
            std::slice::from_ref(&candidate),
        )
        .unwrap();

    assert_eq!(
        matches,
        vec![LocationMatch {
            origin: candidate,
            via: Some(Location::file("/producer/artifact")),
        }]
    );
}

#[test]
fn preserves_multiple_paths_to_the_same_source() {
    let (_files, engine) = engine(
        &["/producer/artifact-a", "/producer/artifact-b"],
        &[
            ("/producer/artifact-a", "/producer/source"),
            ("/producer/artifact-b", "/producer/source"),
        ],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    let matches = engine
        .match_locations(
            &Location::file("/consumer/input"),
            std::slice::from_ref(&candidate),
        )
        .unwrap();

    assert_eq!(
        matches,
        vec![
            LocationMatch {
                origin: candidate.clone(),
                via: Some(Location::file("/producer/artifact-a")),
            },
            LocationMatch {
                origin: candidate,
                via: Some(Location::file("/producer/artifact-b")),
            },
        ]
    );
}

#[test]
fn preserves_direct_access_alongside_artifact_routes() {
    let (_files, engine) = engine(
        &["/producer/source", "/producer/artifact"],
        &[("/producer/artifact", "/producer/source")],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    assert_eq!(
        engine
            .match_locations(
                &Location::file("/consumer/input"),
                std::slice::from_ref(&candidate),
            )
            .unwrap(),
        vec![
            LocationMatch {
                origin: candidate.clone(),
                via: None,
            },
            LocationMatch {
                origin: candidate,
                via: Some(Location::file("/producer/artifact")),
            },
        ]
    );
}

#[test]
fn repeated_entries_and_associations_do_not_duplicate_routes() {
    let (_files, engine) = engine(
        &["/producer/artifact", "/producer/artifact"],
        &[
            ("/producer/artifact", "/producer/source"),
            ("/producer/artifact", "/producer/source"),
        ],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    assert_eq!(
        engine
            .match_locations(
                &Location::file("/consumer/input"),
                std::slice::from_ref(&candidate),
            )
            .unwrap(),
        vec![LocationMatch {
            origin: candidate,
            via: Some(Location::file("/producer/artifact")),
        }]
    );
}

#[test]
fn associations_do_not_admit_unconsumed_artifacts_or_unassociated_sources() {
    let (_files, engine) = engine(
        &["/producer/artifact"],
        &[
            ("/producer/artifact", "/producer/source"),
            ("/other/artifact", "/other/source"),
        ],
    );
    let candidates = ["/other/source", "/producer/private"]
        .map(|path| SourceSpan::new(Location::file(path), ByteRange::new(10, 20)));

    assert!(
        engine
            .match_locations(&Location::file("/consumer/input"), &candidates)
            .unwrap()
            .is_empty()
    );
}

fn engine(classpath: &[&str], associations: &[(&str, &str)]) -> (TempDir, Engine) {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let consumer = Module {
        classpaths: vec![Classpath {
            inputs: vec![ResourceFilter::file("/consumer/input")],
            entries: Some(classpath.iter().map(ResourceFilter::file).collect()),
        }],
        artifact_sources: vec![],
    };
    let producer = Module {
        classpaths: vec![],
        artifact_sources: associations
            .iter()
            .map(|(artifact, source)| ArtifactSources {
                artifacts: vec![Location::file(*artifact)],
                sources: vec![ResourceFilter::file(source)],
            })
            .collect(),
    };
    engine
        .register_modules(vec![consumer, producer], ImportKind::Toml, files.root())
        .unwrap();
    (files, engine)
}

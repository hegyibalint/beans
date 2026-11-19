use beans_core::{
    artifact_sources::ArtifactSources,
    classpath::Classpath,
    location::Location,
    location_match::{LocationMatch, LocationMatcher},
    module::Module,
    ranges::ByteRange,
    resource_filter::ResourceFilter,
    source::SourceSpan,
};

#[test]
fn matches_sources_through_classpath_artifacts() {
    let modules = modules(
        &["/producer/artifact"],
        &[("/producer/artifact", "/producer/source")],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    let matches = LocationMatcher::new(&modules).match_locations(
        &Location::file("/consumer/input"),
        std::slice::from_ref(&candidate),
    );

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
    let modules = modules(
        &["/producer/artifact-a", "/producer/artifact-b"],
        &[
            ("/producer/artifact-a", "/producer/source"),
            ("/producer/artifact-b", "/producer/source"),
        ],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    let matches = LocationMatcher::new(&modules).match_locations(
        &Location::file("/consumer/input"),
        std::slice::from_ref(&candidate),
    );

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
    let modules = modules(
        &["/producer/source", "/producer/artifact"],
        &[("/producer/artifact", "/producer/source")],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    assert_eq!(
        LocationMatcher::new(&modules).match_locations(
            &Location::file("/consumer/input"),
            std::slice::from_ref(&candidate),
        ),
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
    let modules = modules(
        &["/producer/artifact", "/producer/artifact"],
        &[
            ("/producer/artifact", "/producer/source"),
            ("/producer/artifact", "/producer/source"),
        ],
    );
    let candidate = SourceSpan::new(Location::file("/producer/source"), ByteRange::new(10, 20));

    assert_eq!(
        LocationMatcher::new(&modules).match_locations(
            &Location::file("/consumer/input"),
            std::slice::from_ref(&candidate),
        ),
        vec![LocationMatch {
            origin: candidate,
            via: Some(Location::file("/producer/artifact")),
        }]
    );
}

#[test]
fn associations_do_not_admit_unconsumed_artifacts_or_unassociated_sources() {
    let modules = modules(
        &["/producer/artifact"],
        &[
            ("/producer/artifact", "/producer/source"),
            ("/other/artifact", "/other/source"),
        ],
    );
    let candidates = ["/other/source", "/producer/private"]
        .map(|path| SourceSpan::new(Location::file(path), ByteRange::new(10, 20)));

    assert!(
        LocationMatcher::new(&modules)
            .match_locations(&Location::file("/consumer/input"), &candidates)
            .is_empty()
    );
}

fn modules(classpath: &[&str], associations: &[(&str, &str)]) -> Vec<Module> {
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
    vec![consumer, producer]
}

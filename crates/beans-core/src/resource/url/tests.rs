use std::collections::HashSet;

use super::*;

#[test]
fn file_and_virtual_roots_have_distinct_tags() {
    let file = ResourceId::file("Example.java");
    let virtual_id = virtual_resource("lsp", "untitled:Untitled-1");

    assert_eq!(file.to_url().unwrap(), "beans:file:Example.java");
    assert_eq!(
        virtual_id.to_url().unwrap(),
        "beans:virtual:lsp/untitled%3AUntitled-1"
    );
    round_trip(&file);
    round_trip(&virtual_id);
}

#[test]
fn nesting_uses_one_separator_per_entry() {
    let id = ResourceId::file("app.zip")
        .entry(ResourceEntry("lib/a.jar".into()))
        .entry(ResourceEntry("org/example/Foo.class".into()));

    assert_eq!(
        id.to_url().unwrap(),
        "beans:file:app.zip!/lib/a.jar!/org/example/Foo.class"
    );
    round_trip(&id);
}

#[test]
fn entry_boundaries_and_empty_entries_remain_distinct() {
    let entries = [
        vec![],
        vec![""],
        vec!["", ""],
        vec!["a/b"],
        vec!["a", "b"],
        vec!["/a"],
        vec!["a/"],
        vec!["a//b"],
        vec!["b", "a"],
    ];
    let mut urls = HashSet::new();
    for entries in entries {
        let mut id = ResourceId::file("app.zip");
        id.entries = entries
            .into_iter()
            .map(|entry| ResourceEntry(entry.into()))
            .collect();
        assert!(urls.insert(id.to_url().unwrap()));
        round_trip(&id);
    }
}

#[test]
fn delimiters_and_percent_signs_are_escaped_as_data() {
    let id = ResourceId::file("app!%?# .zip").entry(ResourceEntry("/a!/b?x#y%.class".into()));

    assert_eq!(
        id.to_url().unwrap(),
        "beans:file:app%21%25%3F%23%20.zip!//a%21/b%3Fx%23y%25.class"
    );
    round_trip(&id);
}

#[test]
fn encoded_separators_are_decoded_only_after_splitting_entries() {
    let id = ResourceId::file("app.zip")
        .entry(ResourceEntry("lib!/a.jar".into()))
        .entry(ResourceEntry("%21%2F.class".into()));

    assert_eq!(
        id.to_url().unwrap(),
        "beans:file:app.zip!/lib%21/a.jar!/%2521%252F.class"
    );
    round_trip(&id);
}

#[test]
fn providers_and_keys_are_separately_escaped_opaque_strings() {
    let id = virtual_resource("p/!:%", "k/..!#?");

    assert_eq!(
        id.to_url().unwrap(),
        "beans:virtual:p%2F%21%3A%25/k%2F..%21%23%3F"
    );
    round_trip(&id);
    assert_ne!(
        virtual_resource("provider", "key").to_url().unwrap(),
        virtual_resource("Provider", "key").to_url().unwrap()
    );
}

#[test]
fn empty_roots_and_virtual_fields_are_preserved() {
    for id in [
        ResourceId::file(""),
        virtual_resource("", ""),
        virtual_resource("", "key"),
        virtual_resource("provider", ""),
    ] {
        round_trip(&id);
    }
    assert_eq!(
        virtual_resource("", "").to_url().unwrap(),
        "beans:virtual:/"
    );
}

#[test]
fn entry_strings_are_not_normalized_as_paths() {
    let mut id = virtual_resource("provider", "../key//.");
    for name in [".", "..", "a/../b", "./a", "/a//b/"] {
        id.entries.push(ResourceEntry(name.into()));
    }

    round_trip(&id);
    let parsed = ResourceId::from_url(&id.to_url().unwrap()).unwrap();
    assert_eq!(parsed.entries, id.entries);
    assert_eq!(parsed.root, id.root);
}

#[test]
fn equal_file_paths_have_identical_keys() {
    for (a, b) in [
        ("a/./b", "a/b"),
        ("a//b", "a/b"),
        ("a/b/", "a/b"),
        ("./a/./b", "./a/b"),
    ] {
        let a = ResourceId::file(a);
        let b = ResourceId::file(b);
        assert_eq!(a, b);
        assert_eq!(a.to_url().unwrap(), b.to_url().unwrap());
        round_trip(&a);
        round_trip(&b);
    }
}

#[test]
fn parent_steps_and_leading_current_directory_steps_are_not_resolved() {
    for (a, b) in [("a/../b", "b"), ("./a", "a")] {
        let a = ResourceId::file(a);
        let b = ResourceId::file(b);
        assert_ne!(a, b);
        assert_ne!(a.to_url().unwrap(), b.to_url().unwrap());
        round_trip(&a);
    }
}

#[test]
fn text_components_round_trip_unicode_and_control_characters() {
    for character in (0..=255).map(char::from).chain(['λ', '中', '🫘']) {
        let text = character.to_string();
        let id = virtual_resource(&text, &text).entry(ResourceEntry(text));
        assert!(id.to_url().unwrap().is_ascii());
        round_trip(&id);
    }
}

#[test]
fn lower_case_escapes_are_accepted_and_reencoded_canonically() {
    let id = ResourceId::from_url("beans:virtual:p/k%2f%21%3f").unwrap();

    assert_eq!(id, virtual_resource("p", "k/!?"));
    assert_eq!(id.to_url().unwrap(), "beans:virtual:p/k%2F%21%3F");
}

#[test]
fn malformed_or_foreign_urls_are_rejected() {
    for url in [
        "",
        "file:/app.zip",
        "jar:file:/app.zip!/Foo.class",
        "beans:http:app.zip",
        "beans:virtual:provider",
        "beans:virtual:p/key/extra",
        "beans:file:%",
        "beans:file:%0",
        "beans:file:%GG",
        "beans:file:app!name.zip",
        "beans:file:app.zip?query",
        "beans:file:app.zip#fragment",
        "beans:file:app name.zip",
        "beans:virtual:p/λ",
        "beans:virtual:%FF/key",
        "beans:virtual:p/%FF",
        "beans:file:app.zip!/%FF",
        "beans:file:app.zip!/%C0%AF",
    ] {
        assert!(ResourceId::from_url(url).is_err(), "accepted {url:?}");
    }
}

#[test]
fn nesting_depth_is_not_limited_by_the_codec() {
    let mut id = ResourceId::file("app.zip");
    for index in 0..4096 {
        id.entries.push(ResourceEntry(format!("entry/{index}")));
    }

    round_trip(&id);
}

#[test]
fn a_literal_utf16_prefix_is_not_confused_with_native_encoding() {
    let id = ResourceId::file("utf16:0041");

    assert_eq!(id.to_url().unwrap(), "beans:file:utf16%3A0041");
    round_trip(&id);
}

#[cfg(unix)]
#[test]
fn unix_file_paths_preserve_non_utf8_bytes() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let id = ResourceId::file(OsString::from_vec(b"app-\xff.jar".to_vec()))
        .entry(ResourceEntry("p/Foo.class".into()));

    assert_eq!(id.to_url().unwrap(), "beans:file:app-%FF.jar!/p/Foo.class");
    round_trip(&id);
    for byte in 0..=255 {
        let id = ResourceId::file(OsString::from_vec(vec![b'a', byte, b'b']));
        round_trip(&id);
    }
}

#[cfg(not(windows))]
#[test]
fn windows_native_payloads_are_not_silently_reinterpreted() {
    assert!(ResourceId::from_url("beans:file:utf16:D800").is_err());
}

#[cfg(windows)]
#[test]
fn windows_file_paths_preserve_unpaired_surrogates() {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};

    let id = ResourceId::file(OsString::from_wide(&[0xD800, 0x0061]));

    assert_eq!(id.to_url().unwrap(), "beans:file:utf16:D8000061");
    round_trip(&id);
}

#[cfg(windows)]
#[test]
fn windows_equal_drive_prefixes_and_separators_have_identical_keys() {
    for (a, b) in [
        (r"c:\foo\bar", r"C:/foo/bar"),
        (r"c:foo", r"C:foo"),
        (r"\\server\share\foo", "//server/share/foo"),
    ] {
        let a = ResourceId::file(a);
        let b = ResourceId::file(b);
        assert_eq!(a, b);
        assert_eq!(a.to_url().unwrap(), b.to_url().unwrap());
        round_trip(&a);
        round_trip(&b);
    }
}

fn virtual_resource(provider: &str, key: &str) -> ResourceId {
    ResourceId::new(ResourceRoot::Virtual {
        provider: provider.into(),
        key: key.into(),
    })
}

fn round_trip(id: &ResourceId) {
    let url = id.to_url().unwrap();
    let parsed = ResourceId::from_url(&url).unwrap();
    assert_eq!(&parsed, id, "failed to round-trip {url}");
    assert_eq!(parsed.to_url().unwrap(), url);
}

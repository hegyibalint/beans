//! Stage 2, §7.5.1. The one stage whose names come out of the file rather than
//! out of the lake.

use super::*;

#[test]
fn a_single_type_import_offers_its_last_segment() {
    let items = complete_at_cursor(
        "package p;
import java.util.List;
class Test {
    <cur> field;
}
",
    );

    assert!(labels(&items).contains(&"List"));
    assert_eq!(
        item(&items, "List").detail.as_deref(),
        Some("java.util.List")
    );
}

/// The distinction this stage rests on: an import is a name the user wrote, not
/// one we propose. Nothing in the lake answers `java.util.List` here, and the
/// name is still offered, because declining to finish typing what is already in
/// the buffer reads as a bug rather than as caution.
#[test]
fn an_import_is_offered_even_though_nothing_resolves_it() {
    let items = complete_at_cursor(
        "package p;
import com.nowhere.Missing;
class Test {
    <cur> field;
}
",
    );

    assert!(labels(&items).contains(&"Missing"));
}

/// §7.5.2 and §7.5.4 reach the model as a kind and are read by nobody yet, so
/// an on-demand import spells no name at all — its last segment is a package.
#[test]
fn an_on_demand_import_offers_nothing() {
    let items = complete_at_cursor(
        "package p;
import java.util.*;
class Test {
    <cur> field;
}
",
    );

    assert_eq!(labels(&items), ["Test"]);
}

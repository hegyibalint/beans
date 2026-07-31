// A byte offset becomes the line and column an editor understands, which is the
// direction every answer we send travels.

use super::*;

#[test]
fn line_starts_follow_every_newline() {
    let file = TextFile::new("a\nbb\nccc");
    assert_eq!(file.line_starts, [0, 2, 5]);
}

#[test]
fn an_offset_maps_to_its_line_and_column() {
    let file = TextFile::new("a\nbb\nccc");

    assert_eq!(file.line_column(Offset(0)), line_column(0, 0));
    assert_eq!(file.line_column(Offset(2)), line_column(1, 0));
    assert_eq!(file.line_column(Offset(4)), line_column(1, 2));
    assert_eq!(file.line_column(Offset(5)), line_column(2, 0));
}

#[test]
fn columns_count_utf16_code_units() {
    // `😀` is four UTF-8 bytes but two UTF-16 code units.
    let file = TextFile::new("a😀b");
    // `b` sits at byte 5, three UTF-16 units into the line.
    assert_eq!(file.line_column(Offset(5)), line_column(0, 3));
}

#[test]
fn an_offset_past_the_end_clamps_to_the_last_position() {
    let file = TextFile::new("ab\ncd");
    assert_eq!(file.line_column(Offset(999)), line_column(1, 2));
}

#[test]
fn the_empty_file_answers_the_origin() {
    assert_eq!(
        TextFile::default().line_column(Offset(0)),
        line_column(0, 0)
    );
}

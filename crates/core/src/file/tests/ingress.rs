// The line and column an editor sends us becomes a byte offset, which is the
// direction every request we receive travels. Unlike the way back, this one can
// fail: an editor can name a position that does not exist.

use super::*;

#[test]
fn a_position_maps_to_its_byte_offset() {
    let file = TextFile::new("first\nsecond\n");

    assert_eq!(file.offset(line_column(1, 3)), Some(Offset(9)));
    assert_eq!(file.offset(line_column(2, 0)), Some(Offset(13)));
}

#[test]
fn columns_count_utf16_code_units() {
    let file = TextFile::new("a😀b");

    assert_eq!(file.offset(line_column(0, 1)), Some(Offset(1)));
    assert_eq!(file.offset(line_column(0, 3)), Some(Offset(5)));
    assert_eq!(file.offset(line_column(0, 4)), Some(Offset(6)));
    // Column 2 lands inside the surrogate pair: no such offset.
    assert_eq!(file.offset(line_column(0, 2)), None);
}

#[test]
fn a_multibyte_character_advances_one_column() {
    assert_eq!(
        TextFile::new("éx").offset(line_column(0, 1)),
        Some(Offset(2))
    );
}

#[test]
fn a_position_outside_the_file_has_no_offset() {
    assert_eq!(TextFile::new("abc").offset(line_column(0, 4)), None);
    assert_eq!(TextFile::new("abc").offset(line_column(1, 0)), None);
}

#[test]
fn a_carriage_return_is_not_part_of_its_line() {
    let file = TextFile::new("ab\r\nc");

    assert_eq!(file.offset(line_column(0, 2)), Some(Offset(2)));
    assert_eq!(file.offset(line_column(1, 0)), Some(Offset(4)));
}

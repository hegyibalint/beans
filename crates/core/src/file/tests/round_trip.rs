// The two directions have to agree. Each one can be right on its own terms and
// still disagree with the other, and a caret that moves as it crosses the wire
// is what that disagreement looks like to a user.

use super::*;

#[test]
fn a_position_survives_both_directions() {
    let file = TextFile::new("class A {\n    B field;\n}\n");
    let position = line_column(1, 4);
    assert_eq!(file.line_column(file.offset(position).unwrap()), position);
}

mod egress;
mod ingress;
mod round_trip;

use super::*;

fn line_column(line: u32, character: u32) -> LineColumnPosition {
    LineColumnPosition { line, character }
}

mod compatibility;
mod declarations;
mod malformed;

use crate::model;

fn parse_type(bytes: &[u8]) -> model::Class {
    match super::parse(bytes).expect("fixture should parse") {
        super::ParseOutcome::Class(class) => class,
        super::ParseOutcome::ModuleDescriptor => panic!("fixture should describe a type"),
    }
}

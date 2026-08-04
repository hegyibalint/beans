mod compatibility;
mod declarations;
mod malformed;

use crate::model::JvmClass;

fn parse_type(bytes: &[u8]) -> JvmClass {
    match super::parse(bytes).expect("fixture should parse") {
        super::ParseOutcome::Class(class) => class,
        super::ParseOutcome::ModuleDescriptor => panic!("fixture should describe a type"),
    }
}

use beans_core::{names::Name, ranges::ByteRange};
use beans_platform_jvm::query as jvm;
use rusqlite::Connection;

use crate::query as java;

use super::{TypeCandidate, TypeLocator};

pub fn find_types(name: &Name, connection: &Connection) -> Result<Vec<TypeCandidate>, String> {
    let mut candidates = Vec::new();
    for handle in java::find_type(name, connection)? {
        let Some(declaration) = java::declaration(&handle, connection)? else {
            continue;
        };
        let Some(name) = declaration.name else {
            continue;
        };
        candidates.push(TypeCandidate::new(
            TypeLocator::Declaration(handle),
            name.range(),
        ));
    }

    // JVMS §4.2.1: internal names use '/'; canonical member-name conversion is not wired.
    let binary_name = name.as_slice().join("/");
    for location in jvm::find_classes(&binary_name, connection)? {
        // Class files have no text model yet. This empty span is for reachability,
        // not a navigable source target; virtual class documents are still pending.
        candidates.push(TypeCandidate::new(
            TypeLocator::Jvm(location),
            ByteRange::new(0, 0),
        ));
    }
    Ok(candidates)
}

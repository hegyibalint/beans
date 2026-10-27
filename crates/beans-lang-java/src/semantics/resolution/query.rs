use beans_core::names::Name;
use beans_platform_jvm::query as jvm;
use rusqlite::Connection;

use crate::query as java;

use super::{JavaTypeCandidate, TypeCandidate};

pub fn find_types(name: &Name, connection: &Connection) -> Result<Vec<TypeCandidate>, String> {
    let java = java::find_type(name, connection)?
        .into_iter()
        .map(|handle| TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)));
    // JVMS §4.2.1: internal names use '/'; canonical member-name conversion is not wired.
    let binary_name = name.as_slice().join("/");
    let jvm = jvm::find_classes(&binary_name, connection)?
        .into_iter()
        .map(TypeCandidate::Jvm);
    Ok(java.chain(jvm).collect())
}

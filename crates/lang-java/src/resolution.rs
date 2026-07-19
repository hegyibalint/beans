use beans_core::storage::Revision;
use beans_platform_jvm::{PlatformJvm, model::JvmSource, scope::JvmScope};

use crate::{
    LanguageJava,
    model::{JavaDeclaration, JavaFile, JavaImportKind, JavaName, JavaScope},
};

pub fn resolve_type(
    name: &JavaName,
    file: &JavaFile,
    current_scope: &JavaScope,
    revision: Revision,
    jvm: &PlatformJvm,
    java: &LanguageJava,
) -> Option<JavaDeclaration> {
    // First step: check if there is any type in the local file that matches the name.
    current_scope.iter_symbols(file)

    todo!("Finish resolution engine")
}

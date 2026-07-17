use beans_core::Revision;
use beans_platform_jvm::{PlatformJvm, model::JvmSource, scope::JvmScope};

use crate::{
    LanguageJava,
    model::{JavaFile, JavaImportKind, JavaQualifiedName, JavaSymbol},
};

pub fn resolve_type(
    name: &JavaQualifiedName,
    file: &JvmSource,
    revision: Revision,
    scope: &dyn JvmScope,
    current_file: &JavaFile,
    jvm_platform: &PlatformJvm,
    lang_java: &LanguageJava,
) -> Option<JavaSymbol> {
    // First, check if the type is defined in the current file
    for class in &current_file.classes {
        if &class.name == name {
            return Some(JavaSymbol::Class(class.clone()));
        }
    }

    // Next, let's see if there is an explicit import for this type in the current file
    for import in &current_file.imports {
        match import.kind {
            JavaImportKind::Type | JavaImportKind::Static => {
                if &import.name == name {
                    // Here we would need to resolve the imported type from the platform
                    // For now, we will just return a placeholder
                    return Some(JavaSymbol::ImportedType(import.name.clone()));
                }
            }
            _ => {
                // We cannot make sure at this point if on-demand imports are not shadowed
            }
        }
    }

    // Next, we will check if the type is resolvable if we put `java.lang` in front of it. This is a common case for Java programs.

    None
}

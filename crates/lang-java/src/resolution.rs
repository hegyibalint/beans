use beans_core::storage::Revision;
use beans_platform_jvm::{
    PlatformJvm,
    model::{JvmQualifiedName, JvmSource},
};

use crate::{
    LanguageJava,
    model::{
        JavaDeclaration, JavaDeclarationId, JavaFile, JavaImportKind,
        JavaName::{self, Qualified, Simple},
        JavaLexicalScopeId,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaTypeTarget {
    Java {
        source: JvmSource,
        declaration: JavaDeclarationId,
    },
    Jvm(JvmQualifiedName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaTypeResolution {
    Resolved(JavaTypeTarget),
    Ambiguous(Vec<JavaTypeTarget>),
    Unresolved,
}

pub fn resolve_type_name(
    name: &JavaName,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
    revision: Revision,
    jvm: &PlatformJvm,
    java: &LanguageJava,
) -> JavaTypeResolution {
    // Qualified names follow a separate path that classifies their prefix and resolves members.
    // Every simple-name candidate tier follows the same rules:
    // 1. Gather all candidates in the tier.
    // 2. Deduplicate candidates by declaration identity.
    // 3. Continue to the next tier when no candidates remain.
    // 4. Resolve the name when exactly one candidate remains.
    // 5. Report ambiguity when multiple distinct candidates remain.

    // Tier 1. Resolve declarations through the lexical scope chain, nearest scope first.
    if let JavaName::Simple(name) = name {
        if let Some(declaration_id) = file
            .iter_declaration_chain(current_lexical_scope_id)
            .find_map(
            |(_, declaration_id, declaration)| {
                let declaration_name = match declaration {
                    JavaDeclaration::Type(declaration) => declaration.name.as_ref(),
                    JavaDeclaration::TypeParameter(declaration) => declaration.name.as_ref(),
                    _ => None,
                }?;

                (declaration_name.text == name.text).then_some(declaration_id)
            },
        ) {
            return JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source: source.clone(),
                declaration: declaration_id,
            });
        }
    }

    // Tier 2. Resolve single-type and single-static imports.
    let matching_java_imports = file
        .imports
        .iter()
        .filter(
            // We are only interested in exact imports
            |import| matches!(import.kind, JavaImportKind::Type | JavaImportKind::Static),
        )
        .filter(|import| match (&import.name, name) {
            (Simple(imported), Simple(query)) => imported.text.as_str() == query.text.as_str(),
            (Qualified(imported), Simple(query)) => imported
                .segments()
                .last()
                .is_some_and(|identifier| identifier.text.as_str() == query.text.as_str()),
            (_, Qualified(_)) => false,
        });
    let matching_jvm_imports =

    // Tier 3. Resolve top-level types in the current package.

    // Tier 4. Resolve ordinary on-demand imports, static on-demand imports, and java.lang.

    // Tier 5. Resolve module imports.

    // Tier 6. Search accessible declarations for import suggestions.

    todo!("Finish resolution engine")
}

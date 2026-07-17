use beans_platform_jvm::{model::JvmSource, scope::JvmScope};

use crate::model::{JavaQualifiedName, JavaSymbol};

pub fn resolve_type(
    name: &JavaQualifiedName,
    file: &JvmSource,
    scope: &dyn JvmScope,
) -> Option<JavaSymbol> {
    todo!("Implement type resolution for Java types");
}

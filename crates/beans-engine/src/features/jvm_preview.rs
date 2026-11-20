use beans_platform_jvm::{query, virtual_file};

use crate::Engine;

impl Engine {
    /// Renders a uniquely indexed class by internal name for the developer preview.
    pub fn preview_jvm_class(&self, name: &str) -> Result<String, String> {
        let connection = self.open_connection()?;
        let class = query::unique_class_by_name(name, &connection)?.ok_or_else(|| {
            format!("JVM class {name} is not indexed. If JDK indexing is running, try again when it finishes.")
        })?;

        // Render only the stored snapshot, even if the original artifact is missing.
        let document = virtual_file::generate(&class)
            .ok_or_else(|| format!("JVM class {name} has no declaration preview"))?;
        Ok(document.into_text())
    }
}

use std::path::Path;

use beans_core::{module::Module, paths::normalize};
use rusqlite::{Connection, TransactionBehavior, params};

use crate::Engine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum ImportKind {
    Toml = 0,
    Gradle = 1,
    Maven = 2,
}

impl Engine {
    /// Atomically replaces one import's modules, identified by kind and normalized root.
    /// An empty snapshot clears that import. Resources are neither read nor indexed here.
    pub fn register_modules(
        &self,
        modules: Vec<Module>,
        kind: ImportKind,
        root: &Path,
    ) -> Result<(), String> {
        // Prepare the snapshot without holding a write transaction.
        let root = normalize(&std::path::absolute(root).map_err(|error| error.to_string())?);
        let root = root.to_str().ok_or("import root is not UTF-8")?;
        let mut serialized = Vec::with_capacity(modules.len());
        for module in modules {
            serialized.push(serde_json::to_string(&module).map_err(|error| error.to_string())?);
        }

        // Replace only this import's logical modules.
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM modules WHERE kind = ?1 AND root = ?2",
                params![kind as i64, root],
            )
            .map_err(|error| error.to_string())?;
        for model in serialized {
            transaction
                .execute(
                    "INSERT INTO modules (kind, root, model) VALUES (?1, ?2, ?3)",
                    params![kind as i64, root, model],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    /// Returns all registered modules, including associations from other import roots.
    pub fn modules(&self) -> Result<Vec<Module>, String> {
        load_modules(&self.open_connection()?)
    }
}

pub(super) fn load_modules(connection: &Connection) -> Result<Vec<Module>, String> {
    let mut statement = connection
        .prepare("SELECT model FROM modules ORDER BY id")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    let mut modules = Vec::new();
    for row in rows {
        let model = row.map_err(|error| error.to_string())?;
        modules.push(serde_json::from_str(&model).map_err(|error| error.to_string())?);
    }
    Ok(modules)
}

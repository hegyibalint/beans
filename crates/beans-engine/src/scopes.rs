use std::path::Path;

use beans_core::{compilation_scope::CompilationScope, location::Location, paths::normalize};
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
    pub fn register_scopes(
        &self,
        scopes: Vec<CompilationScope>,
        kind: ImportKind,
        root: &Path,
    ) -> Result<(), String> {
        // Prepare the snapshot without holding a write transaction.
        let root = normalize(&std::path::absolute(root).map_err(|error| error.to_string())?);
        let root = root.to_str().ok_or("import root is not UTF-8")?;
        let mut serialized = Vec::with_capacity(scopes.len());
        for scope in scopes {
            let inputs = serde_json::to_string(&scope.inputs).map_err(|error| error.to_string())?;
            let dependencies = scope
                .dependencies
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?;
            serialized.push((inputs, dependencies));
        }

        // Replace only this import's logical scopes.
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM compilation_scopes WHERE kind = ?1 AND root = ?2",
                params![kind as i64, root],
            )
            .map_err(|error| error.to_string())?;
        for (inputs, dependencies) in serialized {
            transaction
                .execute(
                    "INSERT INTO compilation_scopes (kind, root, name, inputs, dependencies)
                     VALUES (?1, ?2, '', ?3, ?4)",
                    params![kind as i64, root, inputs, dependencies],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn scopes_for(&self, source: &Location) -> Result<Vec<CompilationScope>, String> {
        let connection = self.open_connection()?;
        let mut scopes = load_scopes(&connection)?;
        scopes.retain(|scope| scope.inputs.iter().any(|input| input.contains(source)));
        Ok(scopes)
    }
}

pub(super) fn load_scopes(connection: &Connection) -> Result<Vec<CompilationScope>, String> {
    let mut statement = connection
        .prepare("SELECT inputs, dependencies FROM compilation_scopes ORDER BY id")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut scopes = Vec::new();
    for row in rows {
        let (inputs, dependencies) = row.map_err(|error| error.to_string())?;
        scopes.push(CompilationScope {
            inputs: serde_json::from_str(&inputs).map_err(|error| error.to_string())?,
            dependencies: dependencies
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|error| error.to_string())?,
        });
    }
    Ok(scopes)
}

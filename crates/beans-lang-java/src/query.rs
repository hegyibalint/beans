use beans_core::{
    classpath::Classpath,
    location::{Location, LocationEntry},
    names::Name,
    query_scope::QueryScope,
    resolution::TypeDefinitionQuery,
    revision::Revision,
};
use rusqlite::{Connection, OptionalExtension, params};

pub use crate::semantics::DeclarationHandle;
use crate::{
    model::{
        File,
        nodes::{NodeIndex, types::TypeDeclaration},
    },
    storage,
};

pub struct JavaQuery<'a> {
    connection: &'a Connection,
    scope: QueryScope<'a>,
}

impl<'a> JavaQuery<'a> {
    pub fn new(connection: &'a Connection, scope: QueryScope<'a>) -> Self {
        Self { connection, scope }
    }

    pub fn revision(&self) -> Revision {
        self.scope.revision()
    }

    pub fn classpath(&self) -> &'a dyn Classpath {
        self.scope.classpath()
    }

    pub fn file(&self, source: &Location) -> Result<Option<File>, String> {
        self.file_at(self.revision(), source)
    }

    pub fn declaration_handle(
        &self,
        source: &Location,
        node_index: NodeIndex,
    ) -> DeclarationHandle {
        DeclarationHandle::new(self.revision(), source.clone(), node_index)
    }

    pub fn declaration(
        &self,
        handle: &DeclarationHandle,
    ) -> Result<Option<TypeDeclaration>, String> {
        Ok(self
            .file_at(handle.revision(), handle.source())?
            .and_then(|file| {
                file.node(handle.node_index())
                    .and_then(|node| node.kind().as_type())
                    .cloned()
            }))
    }

    pub fn find_type(&self, name: &Name) -> Result<Vec<DeclarationHandle>, String> {
        if !name.is_valid() || name.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "{LATEST}
            SELECT java_types.entry_id, java_types.node_index
            FROM java_types JOIN latest ON latest.id = java_types.entry_id
            WHERE latest.version = 1 AND latest.model_kind = 1 AND latest.present
              AND java_types.name = ?2
            ORDER BY java_types.entry_id, java_types.node_index"
        );
        let mut statement = self
            .connection
            .prepare(&sql)
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![self.revision().as_i64(), name.to_string()], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| error.to_string())?;
        let mut handles = Vec::new();
        for row in rows {
            let (id, index) = row.map_err(|error| error.to_string())?;
            let source = self.location(id)?;
            if !self.classpath().contains_source(&source) {
                continue;
            }
            let index = usize::try_from(index).map_err(|_| "invalid Java node index")?;
            handles.push(self.declaration_handle(&source, NodeIndex::new(index)));
        }
        Ok(handles)
    }

    fn file_at(&self, revision: Revision, source: &Location) -> Result<Option<File>, String> {
        if revision > self.revision() {
            return Err("requested revision is newer than the query snapshot".into());
        }
        let location = serde_json::to_string(&storage::components(source)?)
            .map_err(|error| error.to_string())?;
        let sql = format!(
            "{LATEST} SELECT entries.model_kind, entries.model
             FROM latest JOIN entries ON entries.id = latest.id
             WHERE latest.version = 1 AND latest.location = ?2"
        );
        let selected: Option<(i64, Option<String>)> = self
            .connection
            .query_row(&sql, params![revision.as_i64(), location], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((1, Some(model))) = selected else {
            return Ok(None);
        };
        let file: File = serde_json::from_str(&model).map_err(|error| error.to_string())?;
        file.validate_nodes()?;
        Ok(Some(file))
    }

    fn location(&self, id: i64) -> Result<Location, String> {
        let mut statement = self
            .connection
            .prepare("SELECT kind, value FROM locations WHERE entry_id = ?1 ORDER BY position")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        let mut entries = Vec::new();
        for row in rows {
            let (kind, value) = row.map_err(|error| error.to_string())?;
            entries.push(match kind.as_str() {
                "file" => LocationEntry::File(value.into()),
                "virtual" => LocationEntry::Virtual(value),
                "container" => LocationEntry::Container(value),
                _ => return Err(format!("unknown location kind: {kind}")),
            });
        }
        Ok(Location { entries })
    }
}

impl TypeDefinitionQuery<DeclarationHandle> for JavaQuery<'_> {
    fn find_types(&self, name: &Name) -> Result<Vec<DeclarationHandle>, String> {
        self.find_type(name)
    }
}

const LATEST: &str = "WITH versions AS (
    SELECT entries.id, entries.revision, entries.model_kind, entries.model IS NOT NULL AS present, (
        SELECT json_group_array(json_array(kind, value) ORDER BY position)
        FROM locations WHERE entry_id = entries.id
    ) AS location FROM entries WHERE revision <= ?1
), latest AS (
    SELECT *, ROW_NUMBER() OVER (PARTITION BY location ORDER BY revision DESC, id DESC) AS version
    FROM versions
)";

#[cfg(test)]
mod tests;

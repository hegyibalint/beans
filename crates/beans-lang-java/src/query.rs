use beans_core::{
    classpath::Classpath,
    location::{Location, LocationEntry},
    names::Name,
    query_scope::QueryScope,
    resolution::TypeDefinitionQuery,
};
use rusqlite::{Connection, OptionalExtension};

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

    pub fn classpath(&self) -> &'a dyn Classpath {
        self.scope.classpath()
    }

    pub fn file(&self, source: &Location) -> Result<Option<File>, String> {
        let Some(id) = entry_id(self.connection, source)? else {
            return Ok(None);
        };
        let model: String = self
            .connection
            .query_row("SELECT model FROM entries WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .map_err(|error| error.to_string())?;
        let file: File = serde_json::from_str(&model).map_err(|error| error.to_string())?;
        file.validate_nodes()?;
        Ok(Some(file))
    }

    pub fn declaration_handle(
        &self,
        source: &Location,
        node_index: NodeIndex,
    ) -> DeclarationHandle {
        DeclarationHandle::new(source.clone(), node_index)
    }

    pub fn declaration(
        &self,
        handle: &DeclarationHandle,
    ) -> Result<Option<TypeDeclaration>, String> {
        Ok(self.file(handle.source())?.and_then(|file| {
            file.node(handle.node_index())
                .and_then(|node| node.kind().as_type())
                .cloned()
        }))
    }

    pub fn find_type(&self, name: &Name) -> Result<Vec<DeclarationHandle>, String> {
        if !name.is_valid() || name.is_empty() {
            return Ok(Vec::new());
        }
        let sql = "SELECT entry_id, node_index FROM java_types WHERE name = ?1
                   ORDER BY entry_id, node_index";
        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([name.to_string()], |row| {
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

pub(crate) fn entry_id(connection: &Connection, source: &Location) -> Result<Option<i64>, String> {
    let location =
        serde_json::to_string(&storage::components(source)?).map_err(|error| error.to_string())?;
    connection
        .query_row(
            "SELECT id FROM entries WHERE model_kind = 1 AND (
            SELECT json_group_array(json_array(kind, value) ORDER BY position)
            FROM locations WHERE entry_id = entries.id
        ) = ?1",
            [location],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;

use beans_core::{
    classpath::Classpath, location::Location, names::Name, query_scope::QueryScope,
    resolution::TypeDefinitionQuery,
};
use rusqlite::{Connection, OptionalExtension};

use crate::model::{
    File,
    nodes::{NodeIndex, types::TypeDeclaration},
};
pub use crate::semantics::DeclarationHandle;

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

    pub fn file(&self, location: &Location) -> Result<Option<File>, String> {
        let Some(id) = model_id(self.connection, location)? else {
            return Ok(None);
        };
        let model: String = self
            .connection
            .query_row("SELECT model FROM models WHERE id = ?1", [id], |row| {
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
        let sql = "SELECT model_id, node_index FROM java_types WHERE name = ?1
                   ORDER BY model_id, node_index";
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
        let location: String = self
            .connection
            .query_row("SELECT location FROM models WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .map_err(|error| error.to_string())?;
        serde_json::from_str(&location).map_err(|error| error.to_string())
    }
}

impl TypeDefinitionQuery<DeclarationHandle> for JavaQuery<'_> {
    fn find_types(&self, name: &Name) -> Result<Vec<DeclarationHandle>, String> {
        self.find_type(name)
    }
}

pub(crate) fn model_id(
    connection: &Connection,
    location: &Location,
) -> Result<Option<i64>, String> {
    connection
        .query_row(
            "SELECT id FROM models WHERE kind = 1 AND location = ?1",
            [serde_json::to_string(location).map_err(|error| error.to_string())?],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

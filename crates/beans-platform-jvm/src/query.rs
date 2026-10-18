use beans_core::{location::Location, query_scope::QueryScope};
use rusqlite::Connection;

pub struct JvmQuery<'a> {
    connection: &'a Connection,
    scope: QueryScope<'a>,
}

impl<'a> JvmQuery<'a> {
    pub fn new(connection: &'a Connection, scope: QueryScope<'a>) -> Self {
        Self { connection, scope }
    }

    pub fn find_classes(&self, binary_name: &str) -> Result<Vec<Location>, String> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT location FROM models
                 WHERE model_kind = 0 AND json_extract(model, '$.name') = ?1
                 ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([binary_name], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        let mut locations = Vec::new();
        for row in rows {
            let location = row.map_err(|error| error.to_string())?;
            let location: Location =
                serde_json::from_str(&location).map_err(|error| error.to_string())?;
            if self.scope.classpath().contains_class(&location) {
                locations.push(location);
            }
        }
        Ok(locations)
    }
}

#[cfg(test)]
mod tests;

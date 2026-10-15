use beans_core::{resource::ResourceId, revision::Revision};
use beans_lang_java::{model::File, model_query::JavaModelQuery};
use beans_platform_jvm::{Class, model_query::JvmModelQuery};
use rusqlite::{Connection, OptionalExtension, Transaction};

pub struct ModelQuery<'connection> {
    transaction: Transaction<'connection>,
    revision: Revision,
}

impl<'connection> ModelQuery<'connection> {
    /// Starts a reader snapshot at its latest published model revision.
    pub fn new(connection: &'connection mut Connection) -> Result<Self, String> {
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let revision: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(revision), 0) FROM entries",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if revision < 0 {
            return Err("negative database revision".into());
        }
        Ok(Self {
            transaction,
            revision: Revision::new(revision),
        })
    }

    pub fn at_revision(
        connection: &'connection mut Connection,
        revision: Revision,
    ) -> Result<Self, String> {
        let mut query = Self::new(connection)?;
        if revision > query.revision {
            return Err("requested revision is newer than the database snapshot".into());
        }
        query.revision = revision;
        Ok(query)
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn content(&self, source: &ResourceId) -> Result<Option<Vec<u8>>, String> {
        self.transaction
            .query_row(
                "SELECT content FROM entries
                 WHERE resource_url = ?1 AND revision <= ?2
                 ORDER BY revision DESC LIMIT 1",
                rusqlite::params![source.to_url()?, self.revision.as_i64()],
                |row| row.get::<_, Option<Vec<u8>>>(0),
            )
            .optional()
            .map(Option::flatten)
            .map_err(|error| error.to_string())
    }

    fn read_models<T>(
        &self,
        kind: i64,
        decode: fn(&[u8]) -> Result<T, String>,
    ) -> Result<Vec<(ResourceId, T)>, String> {
        let mut statement = self
            .transaction
            .prepare(
                "SELECT entry.resource_url, entry.model_kind, entry.model FROM entries AS entry
             WHERE entry.revision = (
                 SELECT MAX(latest.revision) FROM entries AS latest
                 WHERE latest.resource_url = entry.resource_url AND latest.revision <= ?1
             )
             ORDER BY entry.resource_url",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([self.revision.as_i64()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        let mut models = Vec::new();
        for row in rows {
            let (url, selected_kind, bytes) = row.map_err(|error| error.to_string())?;
            if !matches!(selected_kind, 0 | 1) {
                return Err(format!("unknown model kind: {selected_kind}"));
            }
            if selected_kind != kind {
                continue;
            }
            models.push((ResourceId::from_url(&url)?, decode(&bytes)?));
        }
        Ok(models)
    }

    fn read_model(&self, source: &ResourceId, kind: i64) -> Result<Option<Vec<u8>>, String> {
        let selected: Option<(i64, Vec<u8>)> = self
            .transaction
            .query_row(
                "SELECT model_kind, model FROM entries
                 WHERE resource_url = ?1 AND revision <= ?2
                 ORDER BY revision DESC LIMIT 1",
                rusqlite::params![source.to_url()?, self.revision.as_i64()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((selected_kind, bytes)) = selected else {
            return Ok(None);
        };
        if !matches!(selected_kind, 0 | 1) {
            return Err(format!("unknown model kind: {selected_kind}"));
        }
        Ok((selected_kind == kind).then_some(bytes))
    }
}

impl JavaModelQuery for ModelQuery<'_> {
    fn java_file(&self, source: &ResourceId) -> Result<Option<File>, String> {
        self.read_model(source, 1)?
            .map(|bytes| beans_lang_java::codec::decode(&bytes))
            .transpose()
    }

    fn java_files(&self) -> Result<Vec<(ResourceId, File)>, String> {
        self.read_models(1, beans_lang_java::codec::decode)
    }
}

impl JvmModelQuery for ModelQuery<'_> {
    fn jvm_class(&self, source: &ResourceId) -> Result<Option<Class>, String> {
        self.read_model(source, 0)?
            .map(|bytes| beans_platform_jvm::codec::decode(&bytes))
            .transpose()
    }

    fn jvm_classes(&self) -> Result<Vec<(ResourceId, Class)>, String> {
        self.read_models(0, beans_platform_jvm::codec::decode)
    }
}

#[cfg(test)]
mod tests;

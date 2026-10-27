use std::{path::Path, time::Duration};

use rusqlite::{Connection, TransactionBehavior};

const SCHEMA_VERSION: i64 = 16;

/// Opens a file-backed cache. The caller chooses its location and creates its parent directory.
pub fn open(path: &Path) -> Result<Connection, String> {
    let mut connection =
        Connection::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    initialize(&mut connection)?;
    let mode: String = connection
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if mode != "wal" {
        return Err(format!("expected SQLite WAL mode, got {mode}"));
    }
    Ok(connection)
}

/// Initializes the complete cache schema and configures an existing connection.
/// Older caches are reset; newer schema versions are rejected.
/// The journal mode is left unchanged so in-memory connections are supported.
pub fn initialize(connection: &mut Connection) -> Result<(), String> {
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    initialize_schema(connection, &schema())?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| error.to_string())
}

fn schema() -> String {
    [
        include_str!("storage/schema.sql"),
        beans_lang_java::storage::SCHEMA,
    ]
    .join("\n")
}

fn initialize_schema(connection: &mut Connection, schema: &str) -> Result<(), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let version: i64 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version > SCHEMA_VERSION {
        return Err(format!("unsupported SQLite schema version: {version}"));
    }
    if version < SCHEMA_VERSION {
        // Reset in place so existing WAL connections never refer to an unlinked database.
        let objects = {
            let mut statement = transaction
                .prepare(
                    "SELECT type, name FROM sqlite_schema
                     WHERE type IN ('view', 'table') AND name NOT GLOB 'sqlite_*'
                     ORDER BY type DESC",
                )
                .map_err(|error| error.to_string())?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        for (kind, name) in objects {
            let name = name.replace('"', "\"\"");
            transaction
                .execute_batch(&format!("DROP {kind} \"{name}\""))
                .map_err(|error| error.to_string())?;
        }
        transaction
            .execute_batch(schema)
            .map_err(|error| error.to_string())?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use beans_testing::database;
    use rusqlite::Connection;

    use super::initialize_schema;

    #[test]
    fn model_payloads_must_be_valid_json_text() {
        let connection = database();
        for model in ["'not JSON'", "'{broken}'", "X'7B7D'", "NULL"] {
            assert!(
                connection
                    .execute(
                        &format!("INSERT INTO models VALUES (1, '[]', 0, {model}, NULL)"),
                        [],
                    )
                    .is_err()
            );
        }
        assert_eq!(count(&connection, "models"), 0);
    }

    #[test]
    fn locations_must_be_valid_json_text() {
        let connection = database();
        for location in ["'not JSON'", "'{broken}'", "X'5B5D'", "NULL"] {
            assert!(
                connection
                    .execute(
                        &format!("INSERT INTO models VALUES (1, {location}, 0, '{{}}', NULL)"),
                        [],
                    )
                    .is_err()
            );
        }
        assert_eq!(count(&connection, "models"), 0);
    }

    #[test]
    fn scope_kinds_must_be_known_numbers() {
        let connection = database();
        for kind in ["-1", "3", "'toml'", "NULL"] {
            assert!(
                connection
                    .execute(
                        &format!(
                            "INSERT INTO compilation_scopes (kind, root, name, inputs, dependencies)
                             VALUES ({kind}, '/project', 'main', '[]', '[]')"
                        ),
                        [],
                    )
                    .is_err()
            );
        }
        assert_eq!(count(&connection, "compilation_scopes"), 0);
    }

    #[test]
    fn scope_filters_must_be_valid_json_text_when_present() {
        let connection = database();
        for value in ["'not JSON'", "'{broken}'", "X'5B5D'"] {
            for (inputs, dependencies) in [(value, "'[]'"), ("'[]'", value)] {
                assert!(
                    connection
                        .execute(
                            &format!(
                                "INSERT INTO compilation_scopes (kind, root, name, inputs, dependencies)
                                 VALUES (0, '/project', 'main', {inputs}, {dependencies})"
                            ),
                            [],
                        )
                        .is_err()
                );
            }
        }
        assert!(
            connection
                .execute(
                    "INSERT INTO compilation_scopes (kind, root, name, inputs, dependencies)
                     VALUES (0, '/project', 'main', NULL, '[]')",
                    [],
                )
                .is_err()
        );
        assert_eq!(count(&connection, "compilation_scopes"), 0);
    }

    #[test]
    fn failed_initialization_rolls_back_the_reset() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sentinel (value INTEGER); INSERT INTO sentinel VALUES (42)",
            )
            .unwrap();
        assert!(initialize_schema(&mut connection, "CREATE TABLE broken (").is_err());
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
        assert_eq!(count(&connection, "sentinel"), 1);
    }

    fn count(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}

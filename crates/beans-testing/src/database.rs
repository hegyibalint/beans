use rusqlite::Connection;

pub fn database() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    beans_engine::storage::initialize(&mut connection).unwrap();
    connection
}

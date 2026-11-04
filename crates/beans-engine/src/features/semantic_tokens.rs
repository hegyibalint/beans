use beans_core::{location::Location, ranges::ByteRange};

use crate::Engine;

impl Engine {
    pub fn type_tokens(&self, source: &Location) -> Result<Vec<ByteRange>, String> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        self.java.type_tokens(source, &transaction)
    }
}

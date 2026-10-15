use beans_core::{location::Location, ranges::ByteRange};

use crate::Engine;

impl Engine {
    pub fn type_tokens(&self, source: &Location) -> Result<Vec<ByteRange>, String> {
        let transaction = self
            .storage
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        self.java.type_tokens(&transaction, source)
    }
}

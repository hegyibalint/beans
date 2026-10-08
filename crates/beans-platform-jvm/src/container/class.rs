use std::io::Read;

use beans_core::resource::ResourceId;

use crate::model::class::Class;

pub fn process(id: ResourceId, _reader: impl Read) -> Result<Option<(ResourceId, Class)>, String> {
    // Check _reader starts with 0xCAFEBABE

    // Use parser.rs to deliver the model
    // Return the tuple
    Err(format!("{id:?}: class processing is not implemented"))
}

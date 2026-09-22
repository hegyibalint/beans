//! Java's language-server-facing state.

use beans_lang_java_engine::JavaEngine;

/// The top of the Java vertical consumed by protocol frontends.
#[derive(Default)]
pub struct JavaServer {
    engine: JavaEngine,
}

impl JavaServer {
    pub fn new(engine: JavaEngine) -> Self {
        Self { engine }
    }

    pub fn engine(&self) -> &JavaEngine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut JavaEngine {
        &mut self.engine
    }
}

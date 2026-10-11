//! Application coordination, independent of editor protocols.

use beans_core::{events::Event, revision::Revision};
use beans_lang_java::engine::JavaLanguage;
use beans_platform_jvm::JvmPlatform;
use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg(test)]
mod tests;

pub struct Engine {
    revision: Revision,
    java: JavaLanguage,
    jvm: JvmPlatform,
    sender: Sender<Event>,
    events: Receiver<Event>,
}

impl Engine {
    pub fn new() -> Self {
        let (sender, events) = unbounded();
        let jvm = JvmPlatform::new();
        let java = JavaLanguage::new();

        Self {
            revision: Revision::default(),
            java,
            jvm,
            sender,
            events,
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn sender(&self) -> Sender<Event> {
        self.sender.clone()
    }
}

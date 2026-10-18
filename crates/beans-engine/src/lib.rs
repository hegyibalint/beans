//! Application coordination, independent of editor protocols.

use std::{fs, path::Path, thread::JoinHandle};

use beans_core::events::Event;
use beans_lang_java::engine::JavaLanguage;
use beans_platform_jvm::JvmPlatform;
use crossbeam_channel::{Receiver, Sender, unbounded};
use rusqlite::Connection;

pub mod diagnostics;
mod documents;
mod events;
mod features;
pub mod storage;

#[cfg(test)]
mod tests;

pub struct Engine {
    storage: Connection,

    // Language verticals
    jvm: JvmPlatform,
    java: JavaLanguage,

    // Event bus
    sender: Sender<Event>,
    events: Receiver<Event>,
    event_loop: Option<JoinHandle<()>>,
}

impl Engine {
    pub fn new(workspace_root: &Path) -> Result<Self, String> {
        let directory = workspace_root.join(".beans");
        fs::create_dir_all(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        let storage = storage::open(&directory.join("models"))?;
        let (sender, events) = unbounded();
        let jvm = JvmPlatform::new();
        let java = JavaLanguage::new();

        Ok(Self {
            storage,
            java,
            jvm,
            sender,
            events,
            event_loop: None,
        })
    }

    pub fn sender(&self) -> Sender<Event> {
        self.sender.clone()
    }
}

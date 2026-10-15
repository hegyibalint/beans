//! Application coordination, independent of editor protocols.

use std::{fs, path::Path};

use beans_core::{events::Event, revision::Revision};
use beans_lang_java::engine::JavaLanguage;
use beans_platform_jvm::JvmPlatform;
use crossbeam_channel::{Receiver, Sender, unbounded};
use rusqlite::Connection;

pub mod diagnostics;
mod documents;
mod features;
pub mod storage;

#[cfg(test)]
mod tests;

pub struct Engine {
    // Monotonic version where we are.
    revision: Revision,

    storage: Connection,

    // Language verticals
    jvm: JvmPlatform,
    java: JavaLanguage,

    // Event bus
    sender: Sender<Event>,
    events: Receiver<Event>,
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
            revision: Revision::default(),
            storage,
            java,
            jvm,
            sender,
            events,
        })
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn sender(&self) -> Sender<Event> {
        self.sender.clone()
    }
}

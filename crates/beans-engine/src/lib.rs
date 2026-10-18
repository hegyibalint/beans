//! Application coordination, independent of editor protocols.

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
};

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
    work_dir: PathBuf,
    storage: Connection,

    // Language verticals
    java: JavaLanguage,
    jvm: Arc<JvmPlatform>,

    // Event bus
    sender: Sender<Event>,
    events: Receiver<Event>,
    event_loop: Option<JoinHandle<()>>,
}

impl Engine {
    pub fn new(workspace_root: &Path) -> Result<Self, String> {
        let beans_dir = env::var_os("BEANS_WORK_DIR")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| ".beans".into());
        let work_dir = workspace_root.join(beans_dir);
        fs::create_dir_all(&work_dir)
            .map_err(|error| format!("{}: {error}", work_dir.display()))?;
        let storage = storage::open(&work_dir.join("models"))?;
        let (sender, events) = unbounded();
        let java = JavaLanguage::new();

        Ok(Self {
            work_dir,
            storage,
            java,
            jvm: Arc::new(JvmPlatform::new()),
            sender,
            events,
            event_loop: None,
        })
    }

    pub fn sender(&self) -> Sender<Event> {
        self.sender.clone()
    }
}

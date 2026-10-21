//! Application coordination, independent of editor protocols.

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
};

use beans_core::event::Command;
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
    workspace_root: PathBuf,
    work_dir: PathBuf,
    storage: Connection,

    // Language verticals
    java: JavaLanguage,
    jvm: Arc<JvmPlatform>,

    // Command queue
    command_sender: Sender<Command>,
    command_receiver: Receiver<Command>,
    command_loop: Option<JoinHandle<()>>,
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
        let (sender, commands) = unbounded();
        let java = JavaLanguage::new();

        Ok(Self {
            workspace_root: workspace_root.to_path_buf(),
            work_dir,
            storage,
            java,
            jvm: Arc::new(JvmPlatform::new()),
            command_sender: sender,
            command_receiver: commands,
            command_loop: None,
        })
    }

    pub fn sender(&self) -> Sender<Command> {
        self.command_sender.clone()
    }
}

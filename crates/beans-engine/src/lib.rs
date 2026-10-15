//! Application coordination, independent of editor protocols.

use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::storage::Storage;
use beans_core::events::Event;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

mod documents;
mod event;
mod features;
pub mod storage;

#[cfg(test)]
mod tests;

pub struct Engine {
    // Sqlite storage
    storage: Storage,

    // Event loop
    sender: Sender<Event>,
    receiver: Receiver<Event>,
    listeners: Arc<Mutex<Vec<Sender<Event>>>>,
    shutdown: Sender<()>,
    stopping: Receiver<()>,
    worker: Option<JoinHandle<()>>,
}

impl Engine {
    pub fn new(database_path: &Path) -> Result<Self, String> {
        let storage = Storage::new(database_path)?;
        let (sender, receiver) = unbounded();
        let (shutdown, stopping) = bounded(1);
        Ok(Self {
            storage,
            sender,
            receiver,
            listeners: Arc::new(Mutex::new(Vec::new())),
            shutdown,
            stopping,
            worker: None,
        })
    }

    pub fn sender(&self) -> Sender<Event> {
        self.sender.clone()
    }
}

//! Application coordination, independent of editor protocols.

use beans_core::{events::Event, revision::Revision};
use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::storage::Storage;

mod documents;
mod features;
mod model_query;
pub mod storage;

#[cfg(test)]
mod tests;

pub struct Engine {
    /// Monotonic version where we are.
    revision: Revision,
    /// Central model and content storage
    storage: Storage,

    /// Sender into the main event loop
    sender: Sender<Event>,
    /// Receiver for the main event loop
    events: Receiver<Event>,
}

impl Engine {
    pub fn new() -> Self {
        let (sender, events) = unbounded();

        Self {
            revision: Revision::default(),
            storage: Storage::new(),
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

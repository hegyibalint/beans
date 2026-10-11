//! Application coordination, independent of editor protocols.

use beans_core::events::Event;
use beans_lang::Languages;
use beans_workspace::Workspaces;
use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg(test)]
mod tests;

pub struct Engine {
    languages: Languages,
    workspaces: Workspaces,
    sender: Sender<Event>,
    events: Receiver<Event>,
}

impl Engine {
    pub fn new() -> Self {
        let (sender, events) = unbounded();
        Self {
            languages: Languages::default(),
            workspaces: Workspaces::default(),
            sender,
            events,
        }
    }

    pub fn sender(&self) -> Sender<Event> {
        self.sender.clone()
    }
}

use std::thread;

use beans_core::events::Event;
use crossbeam_channel::Receiver;

use crate::Engine;

impl Engine {
    pub fn start(&mut self) -> Result<(), String> {
        if self.event_loop.is_some() {
            return Err("engine is already started".into());
        }
        let events = self.events.clone();
        self.event_loop = Some(
            thread::Builder::new()
                .name("beans-events".into())
                .spawn(move || run(events))
                .map_err(|error| error.to_string())?,
        );
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let thread = self.event_loop.take().ok_or("engine is not started")?;
        let signal = self.sender.send(Event::Stop);
        thread.join().map_err(|_| "engine event thread panicked")?;
        signal.map_err(|error| error.to_string())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn run(events: Receiver<Event>) {
    for event in events {
        match event {
            Event::Stop => break,
            Event::JavaHomeFound(found) => {
                log::warn!("{}: JDK ingestion is not wired yet", found.home.display());
            }
        }
    }
}

#[cfg(test)]
mod tests;

use std::{fs::File, path::Path, sync::Arc, thread, time::Instant};

use beans_core::events::Event;
use beans_platform_jvm::JvmPlatform;
use crossbeam_channel::Receiver;
use rusqlite::{Connection, TransactionBehavior};

use crate::{Engine, storage};

mod logging;

impl Engine {
    pub fn start(&mut self) -> Result<(), String> {
        if self.event_loop.is_some() {
            return Err("engine is already started".into());
        }
        let events = self.events.clone();
        let storage = storage::open(&self.work_dir.join("models"))?;
        let event_log = logging::open(&self.work_dir);
        let jvm = Arc::clone(&self.jvm);
        self.event_loop = Some(
            thread::Builder::new()
                .name("beans-events".into())
                .spawn(move || Self::run(&jvm, events, event_log, storage))
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

    fn run(
        jvm: &JvmPlatform,
        events: Receiver<Event>,
        mut event_log: Option<File>,
        mut storage: Connection,
    ) {
        for event in events {
            if let Some(file) = event_log.as_mut() {
                if let Err(error) = logging::record(file, &event) {
                    log::warn!("Event logging disabled: {error}");
                    event_log = None;
                }
            }
            match event {
                Event::Stop => break,
                Event::ResourceFound(found) => {
                    if let Err(error) = process_resource(&mut storage, jvm, &found.path) {
                        log::warn!("{}: {error}", found.path.display());
                    }
                }
                Event::JavaHomeFound(found) => {
                    if let Err(error) = process_jdk_home(&mut storage, jvm, &found.home) {
                        log::warn!("{}: {error}", found.home.display());
                    }
                }
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn process_resource(
    storage: &mut Connection,
    jvm: &JvmPlatform,
    path: &Path,
) -> Result<(), String> {
    let started = Instant::now();
    log::info!("Processing resource {}", path.display());
    let transaction = storage
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    if jvm.process(&transaction, path)? {
        log::debug!("Committing resource {}", path.display());
        transaction.commit().map_err(|error| error.to_string())?;
        log::info!(
            "Published resource {} in {:?}",
            path.display(),
            started.elapsed()
        );
    } else {
        log::debug!("Skipped unrecognized resource {}", path.display());
    }
    Ok(())
}

fn process_jdk_home(
    storage: &mut Connection,
    jvm: &JvmPlatform,
    home: &Path,
) -> Result<(), String> {
    let started = Instant::now();
    log::info!("Processing JDK home {}", home.display());
    let transaction = storage
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    log::debug!("Acquired write transaction for JDK home {}", home.display());
    jvm.process_jdk_home(&transaction, home)?;
    log::debug!("Committing JDK home {}", home.display());
    transaction.commit().map_err(|error| error.to_string())?;
    log::info!(
        "Published JDK home {} in {:?}",
        home.display(),
        started.elapsed()
    );
    Ok(())
}

#[cfg(test)]
mod tests;

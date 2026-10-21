use std::{fs::File, path::Path, sync::Arc, thread, time::Instant};

use beans_core::event::Command;
use beans_platform_jvm::{JvmPlatform, jdk_artifacts};
use crossbeam_channel::Receiver;
use rusqlite::{Connection, TransactionBehavior};

use crate::{Engine, storage};

mod logging;

impl Engine {
    pub fn start(workspace_root: &Path) -> Result<Self, String> {
        let mut engine = Self::new(workspace_root)?;
        let commands = engine.command_receiver.clone();
        let storage = storage::open(&engine.work_dir.join("models"))?;
        let event_log = logging::open(&engine.work_dir);
        let jvm = Arc::clone(&engine.jvm);
        engine.command_loop = Some(
            thread::Builder::new()
                .name("beans-commands".into())
                .spawn(move || Self::run(&jvm, commands, event_log, storage))
                .map_err(|error| error.to_string())?,
        );
        Ok(engine)
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let thread = self.command_loop.take().ok_or("engine is not started")?;
        let signal = self.command_sender.send(Command::Stop);
        thread
            .join()
            .map_err(|_| "engine command thread panicked")?;
        signal.map_err(|error| error.to_string())
    }

    fn run(
        jvm: &JvmPlatform,
        commands: Receiver<Command>,
        mut event_log: Option<File>,
        mut storage: Connection,
    ) {
        for command in commands {
            if let Some(file) = event_log.as_mut() {
                if let Err(error) = logging::record(file, &command) {
                    log::warn!("Event logging disabled: {error}");
                    event_log = None;
                }
            }
            match command {
                Command::Start => log::warn!("Start command is not implemented yet"),
                Command::Stop => break,
                Command::ProcessResource(path) => {
                    if let Err(error) = handle_resource(&mut storage, jvm, &path) {
                        log::warn!("{}: {error}", path.display());
                    }
                }
                Command::ProcessJdkHome(home) => {
                    if let Err(error) = handle_jdk_home(&mut storage, jvm, &home) {
                        log::warn!("{}: {error}", home.display());
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

fn handle_resource(storage: &mut Connection, jvm: &JvmPlatform, path: &Path) -> Result<(), String> {
    let started = Instant::now();
    log::info!("Processing resource {}", path.display());
    let classes = jvm.process(path)?;
    let transaction = storage
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    jvm.store(&transaction, path, classes)?;
    log::debug!("Committing resource {}", path.display());
    transaction.commit().map_err(|error| error.to_string())?;
    log::info!(
        "Published resource {} in {:?}",
        path.display(),
        started.elapsed()
    );
    Ok(())
}

fn handle_jdk_home(storage: &mut Connection, jvm: &JvmPlatform, home: &Path) -> Result<(), String> {
    let started = Instant::now();
    log::info!("Processing JDK home {}", home.display());
    let mut resources = Vec::new();
    for artifact in jdk_artifacts(home)? {
        let classes = jvm.process(&artifact)?;
        resources.push((artifact, classes));
    }
    let transaction = storage
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    log::debug!("Acquired write transaction for JDK home {}", home.display());
    for (path, classes) in resources {
        jvm.store(&transaction, &path, classes)?;
    }
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

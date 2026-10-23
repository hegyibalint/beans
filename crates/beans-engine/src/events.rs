use std::{
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use beans_core::event::{Command, Notification, NotificationPayload};
use beans_platform_jvm::{JvmPlatform, jdk_artifacts};
use crossbeam_channel::{Receiver, Sender};
use rusqlite::{Connection, TransactionBehavior};

use crate::{
    Engine,
    logging::{self, Logger},
    storage,
};

impl Engine {
    pub(super) fn start_command_loop(&mut self) -> Result<(), String> {
        let commands = self.command_receiver.clone();
        let subscribers = Arc::clone(&self.subscribers);
        let storage = storage::open(&self.work_dir.join("models"))?;
        let logger = logging::open(&self.work_dir);
        let jvm = Arc::clone(&self.jvm);
        self.command_loop = Some(
            thread::Builder::new()
                .name("beans-commands".into())
                .spawn(move || Self::command_loop(&jvm, commands, &subscribers, logger, storage))
                .map_err(|error| error.to_string())?,
        );
        Ok(())
    }

    pub(super) fn stop_command_loop(&mut self) -> Result<(), String> {
        let thread = self.command_loop.take().ok_or("engine is not started")?;
        let signal = self.command_sender.send(Command::Stop);
        thread
            .join()
            .map_err(|_| "engine command thread panicked")?;
        signal.map_err(|error| error.to_string())
    }

    fn command_loop(
        jvm: &JvmPlatform,
        commands: Receiver<Command>,
        subscribers: &Mutex<Vec<Sender<Notification>>>,
        mut logger: Logger,
        mut storage: Connection,
    ) {
        let mut notification_counter = 0_u64;
        for command in commands {
            logger.record(&command);
            match command {
                Command::Start => log::warn!("Start command is not implemented yet"),
                Command::Stop => break,
                Command::ProcessResource(path) => {
                    notification_counter += 1;
                    let id = notification_counter;
                    publish_notification(
                        subscribers,
                        Notification::ProcessResourceStart(NotificationPayload {
                            id,
                            data: path.clone(),
                        }),
                    );
                    if let Err(error) = handle_command_process_resource(&mut storage, jvm, &path) {
                        log::warn!("{}: {error}", path.display());
                    }
                    publish_notification(
                        subscribers,
                        Notification::ProcessResourceStop(NotificationPayload { id, data: () }),
                    );
                }
                Command::ProcessJdkHome(home) => {
                    notification_counter += 1;
                    let id = notification_counter;
                    publish_notification(
                        subscribers,
                        Notification::ProcessJdkHomeStart(NotificationPayload {
                            id,
                            data: home.clone(),
                        }),
                    );
                    if let Err(error) = handle_command_process_jdk_home(&mut storage, jvm, &home) {
                        log::warn!("{}: {error}", home.display());
                    }
                    publish_notification(
                        subscribers,
                        Notification::ProcessJdkHomeStop(NotificationPayload { id, data: () }),
                    );
                }
            }
        }
    }
}

fn handle_command_process_resource(
    storage: &mut Connection,
    jvm: &JvmPlatform,
    path: &Path,
) -> Result<(), String> {
    let started = Instant::now();
    log::info!("Processing resource {}", path.display());
    let processed = jvm.process(path)?;
    for (location, error) in &processed.failures {
        log::warn!("{location:?}: {error}");
    }
    let transaction = storage
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    jvm.store(&transaction, path, processed.classes)?;
    log::debug!("Committing resource {}", path.display());
    transaction.commit().map_err(|error| error.to_string())?;
    log::info!(
        "Published resource {} in {:?}",
        path.display(),
        started.elapsed()
    );
    Ok(())
}

fn handle_command_process_jdk_home(
    storage: &mut Connection,
    jvm: &JvmPlatform,
    home: &Path,
) -> Result<(), String> {
    let started = Instant::now();
    log::info!("Processing JDK home {}", home.display());
    let mut resources = Vec::new();
    for artifact in jdk_artifacts(home)? {
        let processed = jvm.process(&artifact)?;
        for (location, error) in &processed.failures {
            log::warn!("{location:?}: {error}");
        }
        resources.push((artifact, processed.classes));
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

fn publish_notification(
    subscribers: &Mutex<Vec<Sender<Notification>>>,
    notification: Notification,
) {
    subscribers
        .lock()
        .unwrap()
        .retain(|sender| sender.send(notification.clone()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnected_subscribers_are_removed() {
        let (disconnected, receiver) = crossbeam_channel::unbounded();
        drop(receiver);
        let (connected, notifications) = crossbeam_channel::unbounded();
        let subscribers = Mutex::new(vec![disconnected, connected]);
        let notification =
            Notification::ProcessResourceStop(NotificationPayload { id: 1, data: () });

        publish_notification(&subscribers, notification);

        assert_eq!(subscribers.lock().unwrap().len(), 1);
        assert!(matches!(
            notifications.try_recv().unwrap(),
            Notification::ProcessResourceStop(payload) if payload.id == 1
        ));
        assert!(notifications.is_empty());
    }
}

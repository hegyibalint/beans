use std::path::PathBuf;

pub enum Command {
    // Engine lifecycle
    Start,
    Stop,
    // Resource processing
    ProcessJdkHome(PathBuf),
    ProcessResource(PathBuf),
}

pub enum Notification {
    ProcessJdkHomeStart(NotificationPayload<PathBuf>),
    ProcessJdkHomeStop(NotificationPayload<()>),
    ProcessResourceStart(NotificationPayload<PathBuf>),
    ProcessResourceStop(NotificationPayload<()>),
}

pub struct NotificationPayload<T> {
    pub id: u64,
    pub data: T,
}

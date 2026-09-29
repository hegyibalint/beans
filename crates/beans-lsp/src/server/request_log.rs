use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use lsp_server::Message;
use serde_json::json;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// Opt-in JSON Lines transcript of the decoded LSP messages, not stdio framing.
pub(super) struct RequestLog(Option<std::fs::File>);

impl RequestLog {
    pub(super) fn from_env() -> io::Result<Self> {
        let Some(path) = env::var_os("BEANS_LSP_LOG_PATH") else {
            return Ok(Self(None));
        };
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        options.mode(0o600);
        Ok(Self(Some(options.open(path)?)))
    }

    pub(super) fn record(&mut self, direction: &str, message: &Message) -> io::Result<()> {
        let Some(file) = &mut self.0 else {
            return Ok(());
        };
        let mut line = serde_json::to_vec(&json!({
            "time_ms": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),
            "pid": std::process::id(),
            "direction": direction,
            "message": message,
        }))?;
        line.push(b'\n');
        file.write_all(&line)
    }
}

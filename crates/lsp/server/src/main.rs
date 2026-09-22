use std::process::ExitCode;

use lsp_server::Connection;

fn main() -> ExitCode {
    let (connection, threads) = Connection::stdio();
    // On failure the reader may still be blocked on stdin; do not join it.
    // A normal exit notification stops the reader, so joining also flushes stdout.
    let result = beans_lsp_server::run(connection).and_then(|()| threads.join());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("beans-lsp: {error}");
            ExitCode::FAILURE
        }
    }
}

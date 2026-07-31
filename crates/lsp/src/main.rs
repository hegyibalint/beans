use beans::Beans;
use beans_lsp::{init_trace, run};
use lsp_server::Connection;

fn main() {
    init_trace();
    let (conn, _) = Connection::stdio();
    run(conn, Beans::new());
}

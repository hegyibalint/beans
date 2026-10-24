#[path = "../support/mod.rs"]
mod support;

use beans_testing::{TempDir, template::Template};
use lsp_server::ErrorCode;
use support::{Client, assert_error, initialize_params, position, request};

mod folder_selection;
mod ingestion;
mod lifecycle;
mod visibility;

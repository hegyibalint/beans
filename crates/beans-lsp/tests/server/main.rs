#[path = "../support/mod.rs"]
mod support;

use lsp_server::ErrorCode;
use serde_json::Value;
use support::{Client, assert_error, initialize_params, position, request, server};

mod diagnostics;
mod document_synchronization;
mod goto_definition;
mod hover;
mod lifecycle;
mod semantic_tokens;
mod source_locations;
mod storage;

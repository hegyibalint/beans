use beans_core::analysis::diagnostic::Diagnostics;

use crate::model::JavaFile;

pub fn dummy_diagnostic(_model: &JavaFile) -> Vec<Diagnostics> {
    Vec::new()
}

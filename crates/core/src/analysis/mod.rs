use crate::analysis::{action::Action, diagnostic::Diagnostics};

pub mod action;
pub mod diagnostic;

pub struct FileAnalysis {
    pub diagnostics: Vec<Diagnostics>,
    pub actions: Vec<Action>,
}

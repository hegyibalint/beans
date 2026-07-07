use crate::analysis::{action::Action, diagnostic::Diagnostics};

pub mod diagnostic;
pub mod action;

pub struct FileAnalysis {
    pub diagnostics: Vec<Diagnostics>,
    pub actions: Vec<Action>,
}

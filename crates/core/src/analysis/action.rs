use crate::{Revision, Span};

pub enum ActionKind {
    Terminal,
    Editor,
}

pub struct Action {
    pub span: Span,
    pub title: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub kind: ActionKind,
}

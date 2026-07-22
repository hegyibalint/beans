use crate::model::OffsetSpan;

pub enum ActionKind {
    Terminal,
    Editor,
}

pub struct Action {
    pub span: OffsetSpan,
    pub title: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub kind: ActionKind,
}

use crate::declarations;

pub type ScopeIndex = usize;

/// Scopes hold what at a given location (or technically span) we have access to.
#[derive(Debug)]
pub struct Scope {
    child_scopes: Vec<ScopeIndex>,
    declarations: Vec<declarations::DeclarationIndex>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            child_scopes: Vec::new(),
            declarations: Vec::new(),
        }
    }
}

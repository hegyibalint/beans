pub struct ScopeId(u128);

/// Scopes hold what at a given location (or technically span) we have access to.
pub struct Scope {
    /// The id of the parent scope.
    /// None means that we are the root scope
    parent_scope: Option<ScopeId>,
}

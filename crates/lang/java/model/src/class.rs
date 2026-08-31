pub struct Class {
    /// The scope defined by this class
    scope_id: ScopeId,

    /// Classes defined inside this class. Could be either:
    /// - inner classes (i.e. no static keyword)
    /// - static classes
    classes: Vec<Class>,

    /// Defined fields
    fields: Vec<Field>,
}

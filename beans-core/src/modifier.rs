#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    Public,
    Private,
    Protected,
    Static,
    Abstract,
    Final,
    Sealed,
    Default,
    Synchronized,
    Volatile,
    Transient,
    Native,
    Strictfp,
}

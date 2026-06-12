//! Groovy language module.
//!
//! Groovy's distinctive features (closures, AST transforms, dynamic
//! dispatch via the Meta-Object Protocol) are runtime-shaped rather than
//! kind-shaped — they don't add new symbol categories beyond what the JVM
//! projection ([`crate::jvm`]) already covers. As a result this module
//! carries no `SymbolKind` enum today; it exists to gate the Groovy feature
//! surface on `feature = "groovy"` and to host the parser and rule set in
//! later migration steps.

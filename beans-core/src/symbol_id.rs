//! Opaque identifier for a [`Symbol`](crate::Symbol) in a
//! [`SymbolTable`](crate::SymbolTable).

/// A handle into the symbol table's flat storage.
///
/// `SymbolId` is the prototype symbol model's analogue of the graph engine's
/// `NodeId`. It is a runtime arena index — the inner `usize` is the slot
/// number in `SymbolTable::symbols`. Like `NodeId`, it is **not** stable
/// across rebuilds of the table; callers that need a durable identity use
/// the symbol's `fqn`.
///
/// The inner field is `pub` for now because the prototype walker constructs
/// ids during table population. This will tighten when the prototype is
/// replaced by the graph-based model (per ADR-0003 the prototype is
/// disposable, so the looser visibility is not worth defending).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

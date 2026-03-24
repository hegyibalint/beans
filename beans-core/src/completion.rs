use crate::{Modifier, SymbolKind};

/// A completion item represents a symbol that is visible and relevant
/// at a cursor position. This is what the LSP offers when the developer
/// presses cmd+space.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub name: String,
    pub kind: SymbolKind,
    pub return_type: String,
    pub params: Vec<(String, String)>,
    pub modifiers: Vec<Modifier>,
    pub fqn: String,
    pub detail: String,
}

/// Thin wrapper around `Vec<CompletionItem>` with convenience query methods.
pub struct CompletionItems(pub Vec<CompletionItem>);

impl CompletionItems {
    /// Is an item with this name and kind offered?
    pub fn has(&self, name: &str, kind: SymbolKind) -> bool {
        self.0.iter().any(|i| i.name == name && i.kind == kind)
    }

    /// Get the item with this name and kind. Panics with a clear message if missing.
    pub fn get(&self, name: &str, kind: SymbolKind) -> &CompletionItem {
        self.0
            .iter()
            .find(|i| i.name == name && i.kind == kind)
            .unwrap_or_else(|| {
                let available: Vec<_> = self.0.iter().map(|i| format!("{} ({:?})", i.name, i.kind)).collect();
                panic!(
                    "completion item '{}' ({:?}) not found.\nAvailable items: {:?}",
                    name, kind, available
                );
            })
    }

    /// How many items of this kind?
    pub fn count(&self, kind: SymbolKind) -> usize {
        self.0.iter().filter(|i| i.kind == kind).count()
    }

    /// Sorted names of all items of a given kind.
    pub fn names(&self, kind: SymbolKind) -> Vec<&str> {
        let mut names: Vec<&str> = self.0.iter().filter(|i| i.kind == kind).map(|i| i.name.as_str()).collect();
        names.sort();
        names
    }

    /// Full iterator access for edge cases.
    pub fn iter(&self) -> std::slice::Iter<'_, CompletionItem> {
        self.0.iter()
    }
}

//! String interner — one shared buffer per distinct string.
//!
//! Backlog #037: at gradle/master scale the same qualified-name text
//! is owned by payload headers, registry keys, RAII handles, JVM
//! projections, and `candidate_fqns` — six-plus buffers of identical
//! bytes per symbol, and candidate lists repeat `java.lang.*` names
//! across every file. Interning collapses them to `Arc<str>` clones.
//!
//! Correctness note that keeps this simple: `Arc<str>` equality and
//! hashing are content-based, so interned and uninterned values mix
//! freely — interning is an allocation optimization, never identity
//! semantics. Parsing therefore allocates freely on rayon workers
//! (ADR-0005); the serial integrate boundary interns payloads as they
//! enter the graph.
//!
//! Single-threaded by design (ADR-0018): `RefCell`, no locks. One
//! interner per workspace, owned next to the graph and registries.

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Default)]
pub struct Interner {
    strings: RefCell<HashSet<Arc<str>>>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// The canonical shared buffer for `s` — allocated on first sight,
    /// cloned (pointer bump) afterwards.
    pub fn intern(&self, s: &str) -> Arc<str> {
        let mut strings = self.strings.borrow_mut();
        if let Some(existing) = strings.get(s) {
            return Arc::clone(existing);
        }
        let arc: Arc<str> = Arc::from(s);
        strings.insert(Arc::clone(&arc));
        arc
    }

    /// Drop table entries no external value still references. The table
    /// holds a strong `Arc` per string, so an interned name outlives the
    /// last payload that used it unless swept: `strong_count == 1` means
    /// *only the table* holds it — garbage. Without this, a long editing
    /// session that deletes files accumulates dead names unboundedly.
    ///
    /// O(table); call it where the population shrinks (after file
    /// destroys / reindex), not on the hot path. This is the GC the
    /// `Arc` form can do and a `Symbol(u32)` table fundamentally cannot
    /// (nothing tracks references to an integer).
    pub fn purge(&self) {
        self.strings
            .borrow_mut()
            .retain(|s| Arc::strong_count(s) > 1);
    }

    /// Distinct strings interned so far (diagnostics/measurement).
    pub fn len(&self) -> usize {
        self.strings.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.borrow().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interns_to_shared_buffers() {
        let interner = Interner::new();
        let a = interner.intern("com.example.Service");
        let b = interner.intern("com.example.Service");
        let c = interner.intern("com.example.Other");

        assert!(Arc::ptr_eq(&a, &b), "same text shares one buffer");
        assert!(!Arc::ptr_eq(&a, &c));
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn purge_drops_only_unreferenced_entries() {
        let interner = Interner::new();
        let kept = interner.intern("com.example.Live");
        interner.intern("com.example.Dead"); // returned Arc dropped immediately

        assert_eq!(interner.len(), 2);
        interner.purge();
        assert_eq!(interner.len(), 1, "the unreferenced entry is collected");

        // The live one survived and still interns to the same buffer.
        let again = interner.intern("com.example.Live");
        assert!(Arc::ptr_eq(&kept, &again));
    }
}

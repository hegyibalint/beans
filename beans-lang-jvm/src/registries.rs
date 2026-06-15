//! The shared JVM registry bag.
//!
//! Per ADR-0012 each registry has a typed key and is accessed by named
//! field — no generic dispatch. Per the vertical layout, this bag is
//! the *only* registry surface shared across language verticals: a
//! vertical's own registries live in its own crate
//! (`JavaRegistries`, ...), and the `beans` facade composes them all.

use beans_core::registry::Registry;

use crate::model::keys::{JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};

/// Registries for the JVM projection layer. Populated by every
/// vertical (each language registers its projections) and by bytecode
/// loading (jmod/JAR readers register here directly).
#[derive(Default)]
pub struct JvmRegistries {
    pub types: Registry<JvmTypeKey>,
    pub methods: Registry<JvmMethodKey>,
    pub fields: Registry<JvmFieldKey>,
    pub constructors: Registry<JvmConstructorKey>,
    pub packages: Registry<PackageKey>,
}

impl JvmRegistries {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a notification batch on every JVM registry. Mutations stay
    /// immediate; subscriber callbacks are deferred until the matching
    /// [`Self::commit_batch`]. Explicit per-field forwarding (no
    /// registry trait) keeps the bag's lifecycle readable.
    pub fn begin_batch(&self) {
        self.types.begin_batch();
        self.methods.begin_batch();
        self.fields.begin_batch();
        self.constructors.begin_batch();
        self.packages.begin_batch();
    }

    /// Close the notification batch on every JVM registry, firing each
    /// changed key's subscribers once at the outermost commit.
    ///
    /// The single observer-boundary guarantee is per concrete
    /// `Registry<K>`. This bag coordinates the fields explicitly but
    /// does not snapshot every field before any field fires.
    pub fn commit_batch(&self) {
        self.types.commit_batch();
        self.methods.commit_batch();
        self.fields.commit_batch();
        self.constructors.commit_batch();
        self.packages.commit_batch();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use beans_core::graph::Graph;
    use beans_core::registry::Callback;

    use super::*;
    use crate::model::keys::{
        JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey,
    };

    /// `begin_batch`/`commit_batch` on the bag must reach *every* field,
    /// not just one — a subscriber on each of the five registries should
    /// defer through the batch and fire exactly once at commit.
    #[test]
    fn batch_forwards_to_every_registry_field() {
        let regs = JvmRegistries::new();

        // Real `NodeId`s come from a graph; payload is irrelevant here, so
        // a `Graph<()>` is the cheapest way to mint distinct ids.
        let mut graph: Graph<()> = Graph::new();

        let counters: Vec<Rc<Cell<u32>>> = (0..5).map(|_| Rc::new(Cell::new(0))).collect();
        let cb = |c: &Rc<Cell<u32>>| -> Callback {
            let c = c.clone();
            Rc::new(move || c.set(c.get() + 1))
        };

        let ty = JvmTypeKey::new("com.example.Service");
        let me = JvmMethodKey::new("com.example.Service", "run", Vec::new());
        let fi = JvmFieldKey::new("com.example.Service", "name");
        let ct = JvmConstructorKey::new("com.example.Service", Vec::new());
        let pk = PackageKey::new("com.example");

        let _s0 = regs.types.query(ty.clone()).subscribe(cb(&counters[0]));
        let _s1 = regs.methods.query(me.clone()).subscribe(cb(&counters[1]));
        let _s2 = regs.fields.query(fi.clone()).subscribe(cb(&counters[2]));
        let _s3 = regs
            .constructors
            .query(ct.clone())
            .subscribe(cb(&counters[3]));
        let _s4 = regs.packages.query(pk.clone()).subscribe(cb(&counters[4]));

        regs.begin_batch();
        let _h0 = regs.types.register(ty, graph.insert((), None));
        let _h1 = regs.methods.register(me, graph.insert((), None));
        let _h2 = regs.fields.register(fi, graph.insert((), None));
        let _h3 = regs.constructors.register(ct, graph.insert((), None));
        let _h4 = regs.packages.register(pk, graph.insert((), None));

        assert!(
            counters.iter().all(|c| c.get() == 0),
            "begin_batch must defer notifications on every field"
        );

        regs.commit_batch();
        assert!(
            counters.iter().all(|c| c.get() == 1),
            "commit_batch must fire every field's deferred notification once"
        );
    }
}

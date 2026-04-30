//! JVM registry bag.
//!
//! Per ADR-0012 each registry has its own typed key. [`JvmRegistries`]
//! owns one [`Registry`] per JVM-shaped lookup ([`JvmTypeKey`],
//! [`JvmFieldKey`], [`JvmMethodKey`], [`JvmConstructorKey`],
//! [`PackageKey`]). Per ADR-0015 each registry is internally
//! `Rc<RefCell<_>>`, so the struct is cheap to clone — clones share
//! state, they don't fork it.

use crate::registry::Registry;
use crate::jvm::keys::{
    JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey,
};

/// All JVM-projection registries, bundled into one struct so resolution
/// code names them by field rather than by string key. Per ADR-0012 each
/// field is its own typed registry; there is no generic
/// [`query`](Registry::query) entry point at this level.
#[derive(Clone, Default)]
pub struct JvmRegistries {
    pub types: Registry<JvmTypeKey>,
    pub fields: Registry<JvmFieldKey>,
    pub methods: Registry<JvmMethodKey>,
    pub constructors: Registry<JvmConstructorKey>,
    pub packages: Registry<PackageKey>,
}

impl JvmRegistries {
    pub fn new() -> Self {
        Self::default()
    }
}

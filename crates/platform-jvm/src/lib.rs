pub mod model;

pub struct PlatformJvm {}

impl PlatformJvm {
    pub fn new() -> PlatformJvm {
        PlatformJvm {}
    }

    pub fn register(&self, jvm_class: &model::JvmClass) {}
}

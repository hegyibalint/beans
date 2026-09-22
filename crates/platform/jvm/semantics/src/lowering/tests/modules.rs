use beans_platform_jvm_model::class_files::ClassFile;

use super::lower;

const MODULE: &[u8] = include_bytes!("fixtures/classes/module-info.class");

#[test]
fn module_class_files_lower_to_module_descriptors() {
    let ClassFile::ModuleDescriptor(module) = lower(MODULE) else {
        panic!("expected a module descriptor")
    };

    assert_eq!(module.name.as_str(), "beans.fixture");
}

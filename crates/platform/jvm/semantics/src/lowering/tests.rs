mod classes;
mod modules;

use beans_platform_jvm_model::{class_files::ClassFile, classes::Class};
use cafebabe::{ParseOptions, parse_class_with_options};

use super::lower_into;

fn lower(bytes: &[u8]) -> ClassFile {
    let mut options = ParseOptions::default();
    options.parse_bytecode(false);
    let parsed = parse_class_with_options(bytes, &options).unwrap();
    lower_into(&parsed).unwrap()
}

fn lower_class(bytes: &[u8]) -> Class {
    let ClassFile::Class(class) = lower(bytes) else {
        panic!("expected a class")
    };
    class
}

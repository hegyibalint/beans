use std::{error::Error, fmt};

use crate::model::{
    class_files::ClassFile,
    classes::{AccessLevel, Class, ClassKind, MemberClass},
    modules::ModuleDescriptor,
    names::{BinaryName, ModuleName},
};
use cafebabe::attributes::{AttributeData, InnerClassEntry, ModuleData};
use cafebabe::{AccessFlags, ClassAccessFlags, ClassFile as ParsedClassFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoweringError {
    MissingModuleAttribute,
    MultipleModuleAttributes,
}

impl fmt::Display for LoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingModuleAttribute => {
                formatter.write_str("module descriptor has no Module attribute")
            }
            Self::MultipleModuleAttributes => {
                formatter.write_str("module descriptor has multiple Module attributes")
            }
        }
    }
}

impl Error for LoweringError {}

pub fn lower_into(class_file: &ParsedClassFile<'_>) -> Result<ClassFile, LoweringError> {
    if class_file.access_flags.contains(ClassAccessFlags::MODULE) {
        lower_module_descriptor(class_file).map(ClassFile::ModuleDescriptor)
    } else {
        Ok(ClassFile::Class(lower_class(class_file)))
    }
}

fn lower_class(class_file: &ParsedClassFile<'_>) -> Class {
    let mut class = Class::new(
        binary_name(&class_file.this_class),
        class_kind(class_file),
        class_access(class_file),
    );
    class.member_classes = inner_class_entries(class_file)
        .filter(|entry| entry.outer_class_info.as_deref() == Some(&class_file.this_class))
        .filter_map(|entry| {
            Some(MemberClass::new(
                entry.inner_name.as_deref()?,
                binary_name(&entry.inner_class_info),
            ))
        })
        .collect();
    class
}

fn lower_module_descriptor(
    class_file: &ParsedClassFile<'_>,
) -> Result<ModuleDescriptor, LoweringError> {
    let mut modules = class_file.attributes.iter().filter_map(|attribute| {
        let AttributeData::Module(module) = &attribute.data else {
            return None;
        };
        Some(module)
    });
    let module = modules
        .next()
        .ok_or(LoweringError::MissingModuleAttribute)?;
    if modules.next().is_some() {
        return Err(LoweringError::MultipleModuleAttributes);
    }

    Ok(ModuleDescriptor::new(module_name(module)))
}

fn class_kind(class_file: &ParsedClassFile<'_>) -> ClassKind {
    if class_file
        .access_flags
        .contains(ClassAccessFlags::ANNOTATION)
    {
        ClassKind::AnnotationInterface
    } else if class_file.access_flags.contains(ClassAccessFlags::ENUM) {
        ClassKind::Enum
    } else if class_file
        .attributes
        .iter()
        .any(|attribute| matches!(attribute.data, AttributeData::Record(_)))
    {
        ClassKind::Record
    } else if class_file
        .access_flags
        .contains(ClassAccessFlags::INTERFACE)
    {
        ClassKind::Interface
    } else {
        ClassKind::Class
    }
}

fn class_access(class_file: &ParsedClassFile<'_>) -> AccessLevel {
    // JVMS §4.7.6 preserves source-level access for nested classes because the
    // class header itself has no private or protected class flags.
    let flags = inner_class_entries(class_file)
        .find(|entry| entry.inner_class_info.as_ref() == &*class_file.this_class)
        .map(|entry| entry.access_flags.bits())
        .unwrap_or_else(|| class_file.access_flags.bits());
    access_level(flags)
}

fn access_level(flags: u16) -> AccessLevel {
    if flags & AccessFlags::PUBLIC.bits() != 0 {
        AccessLevel::Public
    } else if flags & AccessFlags::PROTECTED.bits() != 0 {
        AccessLevel::Protected
    } else if flags & AccessFlags::PRIVATE.bits() != 0 {
        AccessLevel::Private
    } else {
        AccessLevel::Package
    }
}

fn inner_class_entries<'a, 'class>(
    class_file: &'a ParsedClassFile<'class>,
) -> impl Iterator<Item = &'a InnerClassEntry<'class>> {
    class_file
        .attributes
        .iter()
        .filter_map(|attribute| {
            let AttributeData::InnerClasses(entries) = &attribute.data else {
                return None;
            };
            Some(entries.as_slice())
        })
        .flatten()
}

fn binary_name(internal_name: &str) -> BinaryName {
    BinaryName::new(internal_name.replace('/', "."))
}

fn module_name(module: &ModuleData<'_>) -> ModuleName {
    let mut decoded = String::with_capacity(module.name.len());
    let mut characters = module.name.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            decoded.push(
                characters
                    .next()
                    .expect("cafebabe validates escaped module names"),
            );
        } else {
            decoded.push(character);
        }
    }
    ModuleName::new(decoded)
}

#[cfg(test)]
mod tests;

use beans_core::ranges::{ByteRange, Spanned};
use cafebabe::AccessFlags as Flag;

use crate::{Class, ClassAccessFlags, ElementType, Field, FieldType, Method};

/// Byte ranges in the synthesized UTF-8 text, suitable for definition targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclarationRanges {
    pub declaration: ByteRange,
    pub name: ByteRange,
}

#[derive(Debug)]
pub struct RenderedFile {
    text: String,
    pub class: DeclarationRanges,
    /// In the same order as `Class::fields`.
    pub fields: Vec<DeclarationRanges>,
    /// In the same order as `Class::methods`.
    pub methods: Vec<DeclarationRanges>,
    type_references: Vec<Spanned<String>>,
}

impl RenderedFile {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn into_text(self) -> String {
        self.text
    }

    /// Finds the smallest reference span containing this UTF-8 byte offset.
    /// The value is an internal JVM class name, not a resolved destination.
    pub fn type_reference_at(&self, offset: usize) -> Option<&Spanned<String>> {
        self.type_references
            .iter()
            .filter(|reference| reference.range().contains(offset))
            .min_by_key(|reference| reference.range().len())
    }
}

/// Generates declarations from a stored snapshot, without reading its original artifact.
/// Module descriptors have no class declaration and return `None`.
pub fn generate(class: &Class) -> Option<RenderedFile> {
    if class.access_flags.contains(ClassAccessFlags::MODULE) {
        return None;
    }

    let mut renderer = Renderer::default();
    let name = match class.name.rsplit_once('/') {
        Some((package, name)) => {
            renderer.text.push_str("package ");
            write_qualified_name(&mut renderer.text, package);
            renderer.text.push_str(";\n\n");
            name
        }
        None => &class.name,
    };

    // Render the class header and record its explicit supertype references.
    let start = renderer.text.len();
    let mut flags = class.access_flags;
    let is_interface = flags.contains(ClassAccessFlags::INTERFACE);
    if is_interface {
        flags.remove(ClassAccessFlags::ABSTRACT);
    }
    write_flags(
        &mut renderer.text,
        flags.bits(),
        &[
            (Flag::PUBLIC, "public"),
            (Flag::ABSTRACT, "abstract"),
            (Flag::FINAL, "final"),
            (Flag::SYNTHETIC, "synthetic"),
        ],
    );
    renderer
        .text
        .push_str(if flags.contains(ClassAccessFlags::ANNOTATION) {
            "@interface "
        } else if is_interface {
            "interface "
        } else if flags.contains(ClassAccessFlags::ENUM) {
            "enum "
        } else {
            "class "
        });
    let name = write_name(&mut renderer.text, name);
    if let Some(superclass) = class
        .superclass
        .as_deref()
        .filter(|name| *name != "java/lang/Object")
    {
        renderer.text.push_str(" extends ");
        renderer.write_reference(superclass);
    }
    if !class.interfaces.is_empty() {
        renderer.text.push_str(if is_interface {
            " extends "
        } else {
            " implements "
        });
        for (index, interface) in class.interfaces.iter().enumerate() {
            if index != 0 {
                renderer.text.push_str(", ");
            }
            renderer.write_reference(interface);
        }
    }
    renderer.text.push_str(" {\n");

    // Render the members and collect their declaration targets.
    let fields = class
        .fields
        .iter()
        .map(|field| renderer.write_field(field))
        .collect();
    if !class.fields.is_empty() && !class.methods.is_empty() {
        renderer.text.push('\n');
    }
    let methods = class
        .methods
        .iter()
        .map(|method| renderer.write_method(method))
        .collect();

    renderer.text.push('}');
    let class = DeclarationRanges {
        declaration: ByteRange::new(start, renderer.text.len()),
        name,
    };
    renderer.text.push('\n');
    Some(RenderedFile {
        text: renderer.text,
        class,
        fields,
        methods,
        type_references: renderer.type_references,
    })
}

#[derive(Default)]
struct Renderer {
    text: String,
    type_references: Vec<Spanned<String>>,
}

impl Renderer {
    fn write_field(&mut self, field: &Field) -> DeclarationRanges {
        self.text.push_str("    ");
        let start = self.text.len();
        write_flags(
            &mut self.text,
            field.access_flags.bits(),
            &[
                (Flag::PUBLIC, "public"),
                (Flag::PROTECTED, "protected"),
                (Flag::PRIVATE, "private"),
                (Flag::STATIC, "static"),
                (Flag::FINAL, "final"),
                (Flag::VOLATILE, "volatile"),
                (Flag::TRANSIENT, "transient"),
                (Flag::SYNTHETIC, "synthetic"),
                (Flag::ENUM, "enum"),
            ],
        );
        self.write_type(&field.field_type);
        self.text.push(' ');
        let name = write_name(&mut self.text, &field.name);
        self.finish_member(start, name)
    }

    fn write_method(&mut self, method: &Method) -> DeclarationRanges {
        self.text.push_str("    ");
        let start = self.text.len();
        write_flags(
            &mut self.text,
            method.access_flags.bits(),
            &[
                (Flag::PUBLIC, "public"),
                (Flag::PROTECTED, "protected"),
                (Flag::PRIVATE, "private"),
                (Flag::STATIC, "static"),
                (Flag::ABSTRACT, "abstract"),
                (Flag::FINAL, "final"),
                (Flag::SYNCHRONIZED, "synchronized"),
                (Flag::NATIVE, "native"),
                (Flag::STRICT, "strictfp"),
                (Flag::BRIDGE, "bridge"),
                (Flag::VARARGS, "varargs"),
                (Flag::SYNTHETIC, "synthetic"),
            ],
        );
        match &method.return_type {
            Some(return_type) => self.write_type(return_type),
            None => self.text.push_str("void"),
        }
        self.text.push(' ');
        let name = match method.name.as_str() {
            name @ ("<init>" | "<clinit>") => {
                let start = self.text.len();
                self.text.push_str(name);
                ByteRange::new(start, self.text.len())
            }
            name => write_name(&mut self.text, name),
        };
        self.text.push('(');
        for (index, parameter) in method.parameters.iter().enumerate() {
            if index != 0 {
                self.text.push_str(", ");
            }
            self.write_type(parameter);
        }
        self.text.push(')');
        self.finish_member(start, name)
    }

    fn finish_member(&mut self, start: usize, name: ByteRange) -> DeclarationRanges {
        self.text.push(';');
        let declaration = ByteRange::new(start, self.text.len());
        self.text.push('\n');
        DeclarationRanges { declaration, name }
    }

    fn write_type(&mut self, field_type: &FieldType) {
        match &field_type.element {
            ElementType::Byte => self.text.push_str("byte"),
            ElementType::Char => self.text.push_str("char"),
            ElementType::Double => self.text.push_str("double"),
            ElementType::Float => self.text.push_str("float"),
            ElementType::Int => self.text.push_str("int"),
            ElementType::Long => self.text.push_str("long"),
            ElementType::Short => self.text.push_str("short"),
            ElementType::Boolean => self.text.push_str("boolean"),
            ElementType::Reference(name) => self.write_reference(name),
        }
        for _ in 0..field_type.dimensions {
            self.text.push_str("[]");
        }
    }

    fn write_reference(&mut self, name: &str) {
        let start = self.text.len();
        write_qualified_name(&mut self.text, name);
        self.type_references.push(Spanned::new(
            name.to_owned(),
            ByteRange::new(start, self.text.len()),
        ));
    }
}

fn write_flags(text: &mut String, bits: u16, labels: &[(Flag, &str)]) {
    for (flag, label) in labels {
        if bits & flag.bits() != 0 {
            text.push_str(label);
            text.push(' ');
        }
    }
}

fn write_qualified_name(text: &mut String, name: &str) {
    for (index, component) in name.split('/').enumerate() {
        if index != 0 {
            text.push('.');
        }
        write_name(text, component);
    }
}

fn write_name(text: &mut String, name: &str) -> ByteRange {
    let start = text.len();
    // JVM names can contain whitespace and punctuation (JVMS §4.2.2).
    if !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '$'))
    {
        text.push_str(name);
    } else {
        text.push_str(&format!("{name:?}"));
    }
    ByteRange::new(start, text.len())
}

//! Prints what a `.java` file looks like on each side of lowering.
//!
//! `cargo inspect-java` prints the bundled demonstration's semantic model.
//!
//! ```text
//! cargo inspect-java
//! cargo inspect-java path/to/A.java
//! cargo inspect-java --dump-cst --dump-semantic path/to/A.java
//! ```

use std::{env, error::Error, ffi::OsString, fmt::Write, fs, io, path::PathBuf, process};

use beans_lang_java_model::{
    File,
    declarations::{Declaration, types::TypeDeclaration},
    imports::Import,
    references::{NameRef, PrimitiveType, TypeBound, TypeNameComponent, TypeRef},
    scopes::ScopeIndex,
};
use beans_lang_java_semantics::lower_into;
use tree_sitter::{Node, Parser};

const USAGE: &str = "usage: cargo inspect-java [--dump-cst] [--dump-semantic] [file.java]";

fn main() {
    if let Err(error) = run() {
        eprintln!("inspect: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse(env::args_os().skip(1))?;
    let source = fs::read_to_string(&arguments.path)?;

    if arguments.cst {
        if arguments.both() {
            println!("== cst ==");
        }
        print!("{}", pretty_tree(&source)?);
    }

    if arguments.semantic {
        if arguments.both() {
            println!("\n== semantic ==");
        }
        print!("{}", pretty_model(&lower_into(&source)));
    }

    Ok(())
}

struct Arguments {
    path: PathBuf,
    cst: bool,
    semantic: bool,
}

impl Arguments {
    fn parse(arguments: impl Iterator<Item = OsString>) -> Result<Self, io::Error> {
        let mut path = None;
        let mut cst = false;
        let mut semantic = false;

        for argument in arguments {
            match argument.to_str() {
                Some("--dump-cst") => cst = true,
                Some("--dump-semantic") => semantic = true,
                Some(flag) if flag.starts_with("--") => {
                    return Err(io::Error::other(format!("unknown flag `{flag}`\n{USAGE}")));
                }
                _ if path.is_some() => return Err(io::Error::other(USAGE)),
                _ => path = Some(PathBuf::from(argument)),
            }
        }

        let path = path.unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/inspect.java")
        });
        if !cst && !semantic {
            semantic = true;
        }

        Ok(Self {
            path,
            cst,
            semantic,
        })
    }

    fn both(&self) -> bool {
        self.cst && self.semantic
    }
}

fn pretty_tree(source: &str) -> Result<String, Box<dyn Error>> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| io::Error::other("tree-sitter did not produce a tree"))?;

    let mut output = String::new();
    write_node(tree.root_node(), None, 0, &mut output);
    Ok(output)
}

fn write_node(node: Node<'_>, field: Option<&str>, depth: usize, output: &mut String) {
    let indentation = "  ".repeat(depth);
    output.push_str(&indentation);

    if let Some(field) = field {
        write!(output, "{field}: ").expect("writing to a string cannot fail");
    }

    if node.is_missing() {
        writeln!(output, "(MISSING {})", node.kind()).expect("writing to a string cannot fail");
        return;
    }

    write!(output, "({}", node.kind()).expect("writing to a string cannot fail");

    if node.named_child_count() == 0 {
        output.push_str(")\n");
        return;
    }

    output.push('\n');
    for index in 0..node.named_child_count() {
        let child = node
            .named_child(index)
            .expect("the child index is within the named child count");
        write_node(
            child,
            node.field_name_for_named_child(index as u32),
            depth + 1,
            output,
        );
    }

    writeln!(output, "{indentation})").expect("writing to a string cannot fail");
}

fn pretty_model(file: &File) -> String {
    let mut output = String::new();

    if let Some(package) = &file.package_name {
        writeln!(output, "package {}", name(package)).expect("writing to a string cannot fail");
    }

    for import in &file.imports {
        writeln!(output, "import {}", import_of(import)).expect("writing to a string cannot fail");
    }

    write_scope(file, File::ROOT_SCOPE_ID, 0, &mut output);
    output
}

fn write_scope(file: &File, index: ScopeIndex, depth: usize, output: &mut String) {
    let scope = file
        .scope(index)
        .expect("the scope index came from the file");
    let indentation = "  ".repeat(depth);

    writeln!(output, "{indentation}scope {index:?} ({:?})", scope.kind())
        .expect("writing to a string cannot fail");

    for declaration in scope.iter_declarations(file) {
        writeln!(
            output,
            "{indentation}  {:?} {}",
            declaration.index,
            declaration_of(declaration.declaration)
        )
        .expect("writing to a string cannot fail");
    }

    for child in scope.child_scopes() {
        write_scope(file, *child, depth + 1, output);
    }
}

fn declaration_of(declaration: &Declaration) -> String {
    match declaration {
        Declaration::Type(declaration) => type_declaration_of(declaration),
        Declaration::Field(_) => "field".to_owned(),
        Declaration::Method(_) => "method".to_owned(),
    }
}

fn type_declaration_of(declaration: &TypeDeclaration) -> String {
    let mut rendered = String::new();

    for access in &declaration.access {
        write!(rendered, "{access:?} ").expect("writing to a string cannot fail");
    }
    for modifier in &declaration.modifiers {
        write!(rendered, "{modifier:?} ").expect("writing to a string cannot fail");
    }

    write!(
        rendered,
        "{:?} {}",
        declaration.kind,
        declaration.name.as_deref().unwrap_or("<unnamed>")
    )
    .expect("writing to a string cannot fail");

    if !declaration.type_parameters.is_empty() {
        let parameters = declaration
            .type_parameters
            .iter()
            .map(|parameter| {
                let mut rendered = parameter.name.clone();
                for bound in &parameter.bounds {
                    write!(rendered, " {}", bound_of(bound))
                        .expect("writing to a string cannot fail");
                }
                rendered
            })
            .collect::<Vec<_>>()
            .join(", ");
        write!(rendered, "<{parameters}>").expect("writing to a string cannot fail");
    }

    if let Some(superclass) = &declaration.declared_superclass {
        write!(rendered, " extends {}", type_ref(superclass)).expect("writing cannot fail");
    }

    if !declaration.declared_superinterfaces.is_empty() {
        let interfaces = declaration
            .declared_superinterfaces
            .iter()
            .map(type_ref)
            .collect::<Vec<_>>()
            .join(", ");
        write!(rendered, " implements {interfaces}").expect("writing to a string cannot fail");
    }

    rendered
}

fn import_of(import: &Import) -> String {
    format!("{} ({:?})", name(import.name()), import.typ())
}

fn name(name: &NameRef) -> String {
    match name {
        NameRef::Simple(segment) => segment.clone(),
        NameRef::Qualified(segments) => segments.join("."),
    }
}

fn type_ref(reference: &TypeRef) -> String {
    match reference {
        TypeRef::Named { segments } => segments.iter().map(component).collect::<Vec<_>>().join("."),
        TypeRef::Primitive(primitive) => primitive_of(primitive).to_owned(),
        TypeRef::Array {
            element,
            dimensions,
        } => format!("{}{}", type_ref(element), "[]".repeat(*dimensions)),
        TypeRef::Void => "void".to_owned(),
    }
}

fn component(component: &TypeNameComponent) -> String {
    if component.bounds.is_empty() {
        return component.name.clone();
    }

    let arguments = component
        .bounds
        .iter()
        .map(type_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}<{arguments}>", component.name)
}

fn type_argument(argument: &TypeBound) -> String {
    match argument {
        TypeBound::Exact { primary } => type_ref(primary),
        TypeBound::Extends {
            primary,
            additional,
        } => {
            let mut rendered = format!("? extends {}", type_ref(primary));
            for bound in additional {
                write!(rendered, " & {}", type_ref(bound)).expect("writing cannot fail");
            }
            rendered
        }
        TypeBound::Super { primary } => format!("? super {}", type_ref(primary)),
        TypeBound::Unbounded => "?".to_owned(),
    }
}

fn bound_of(bound: &TypeBound) -> String {
    match bound {
        TypeBound::Exact { primary } => type_ref(primary),
        TypeBound::Extends {
            primary,
            additional,
        } => {
            let mut rendered = format!("extends {}", type_ref(primary));
            for bound in additional {
                write!(rendered, " & {}", type_ref(bound)).expect("writing cannot fail");
            }
            rendered
        }
        TypeBound::Super { primary } => format!("super {}", type_ref(primary)),
        TypeBound::Unbounded => "?".to_owned(),
    }
}

fn primitive_of(primitive: &PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::Byte => "byte",
        PrimitiveType::Short => "short",
        PrimitiveType::Int => "int",
        PrimitiveType::Long => "long",
        PrimitiveType::Char => "char",
        PrimitiveType::Float => "float",
        PrimitiveType::Double => "double",
        PrimitiveType::Boolean => "boolean",
    }
}

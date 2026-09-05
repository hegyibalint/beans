//! Prints what a `.java` file looks like on each side of lowering.
//!
//! `cargo inspect-java` prints the bundled demonstration's semantic model.
//!
//! ```text
//! cargo inspect-java
//! cargo inspect-java path/to/A.java
//! cargo inspect-java --dump-cst --dump-semantic path/to/A.java
//! ```

use std::{
    collections::HashMap,
    env,
    error::Error,
    ffi::OsString,
    fmt::{self, Write},
    fs,
    io::{self, IsTerminal},
    path::PathBuf,
    process,
};

use beans_lang_java_model::{
    File,
    declarations::{
        Declaration, DeclarationIndex,
        types::{AccessLevel, Kind, Modifier, TypeDeclaration},
    },
    imports::Import,
    references::{NameRef, PrimitiveType, TypeBound, TypeNameComponent, TypeRef},
    scopes::{ScopeIndex, ScopeKind},
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
        let styling = Styling::new(io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none());
        print!("{}", pretty_model(&lower_into(&source), styling));
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

struct InspectionIds {
    scopes: HashMap<ScopeIndex, usize>,
    declarations: HashMap<DeclarationIndex, usize>,
}

impl InspectionIds {
    fn new(file: &File) -> Self {
        let scopes = file
            .iter_scopes()
            .enumerate()
            .map(|(number, indexed)| (indexed.index, number))
            .collect();
        let mut declarations = HashMap::new();
        assign_declaration_ids(file, File::ROOT_SCOPE_ID, &mut declarations);

        Self {
            scopes,
            declarations,
        }
    }

    fn scope_number(&self, index: ScopeIndex) -> usize {
        *self.scopes.get(&index).expect("scope is in the file")
    }

    fn declaration_number(&self, index: DeclarationIndex) -> usize {
        *self
            .declarations
            .get(&index)
            .expect("declaration is in the file")
    }
}

fn assign_declaration_ids(
    file: &File,
    scope_index: ScopeIndex,
    ids: &mut HashMap<DeclarationIndex, usize>,
) {
    let scope = file
        .scope(scope_index)
        .expect("the scope index came from the file");

    for declaration in scope.iter_declarations(file) {
        let next = ids.len();
        ids.entry(declaration.index).or_insert(next);

        if let Some(body) = body_scope(file, scope_index, declaration.index) {
            assign_declaration_ids(file, body, ids);
        }
    }
}

#[derive(Clone, Copy)]
struct Styling {
    enabled: bool,
}

impl Styling {
    fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    fn color(self, value: impl fmt::Display, ansi_color: u8) -> String {
        if self.enabled {
            format!("\u{1b}[{ansi_color}m{value}\u{1b}[0m")
        } else {
            value.to_string()
        }
    }
}

fn scope_id(index: ScopeIndex, ids: &InspectionIds, styling: Styling) -> String {
    styling.color(format_args!("S{}", ids.scope_number(index)), 36)
}

fn declaration_id(index: DeclarationIndex, ids: &InspectionIds, styling: Styling) -> String {
    styling.color(format_args!("D{}", ids.declaration_number(index)), 33)
}

fn pretty_model(file: &File, styling: Styling) -> String {
    let ids = InspectionIds::new(file);
    let mut output = String::new();

    write_scope_structure(file, File::ROOT_SCOPE_ID, 0, &ids, styling, &mut output);
    output
}

fn write_scope_structure(
    file: &File,
    index: ScopeIndex,
    depth: usize,
    ids: &InspectionIds,
    styling: Styling,
    output: &mut String,
) {
    let scope = file
        .scope(index)
        .expect("the scope index came from the file");
    let indentation = "    ".repeat(depth);
    let child_indentation = "    ".repeat(depth + 1);

    match scope.kind() {
        ScopeKind::CompilationUnit => {
            writeln!(
                output,
                "{indentation}{}: compilation unit",
                scope_id(index, ids, styling)
            )
            .expect("writing to a string cannot fail");

            if let Some(package) = &file.package_name {
                writeln!(output, "{child_indentation}package: {}", name(package))
                    .expect("writing to a string cannot fail");
            }
            for import in &file.imports {
                writeln!(output, "{child_indentation}import: {}", import_of(import))
                    .expect("writing to a string cannot fail");
            }
        }
        ScopeKind::TypeBody { owner } => {
            let owner_name = match file.declaration(owner) {
                Some(Declaration::Type(declaration)) => {
                    declaration.name.as_deref().unwrap_or("<unnamed>")
                }
                _ => "<invalid owner>",
            };
            writeln!(
                output,
                "{indentation}{}: type body of {} {owner_name}",
                scope_id(index, ids, styling),
                declaration_id(owner, ids, styling)
            )
            .expect("writing to a string cannot fail");
        }
    }

    for declaration in scope.iter_declarations(file) {
        writeln!(
            output,
            "{child_indentation}{}: {}",
            declaration_id(declaration.index, ids, styling),
            declaration_header(declaration.declaration)
        )
        .expect("writing to a string cannot fail");

        if let Declaration::Type(type_declaration) = declaration.declaration {
            write_type_relationships(type_declaration, depth + 2, output);
        }
    }

    for child in scope.child_scopes() {
        write_scope_structure(file, *child, depth + 1, ids, styling, output);
    }
}

fn body_scope(file: &File, parent: ScopeIndex, owner: DeclarationIndex) -> Option<ScopeIndex> {
    file.scope(parent)?
        .child_scopes()
        .iter()
        .copied()
        .find(|child| {
            matches!(
                file.scope(*child).map(|scope| scope.kind()),
                Some(ScopeKind::TypeBody { owner: child_owner }) if child_owner == owner
            )
        })
}

fn declaration_header(declaration: &Declaration) -> String {
    match declaration {
        Declaration::Type(declaration) => type_declaration_header(declaration),
        Declaration::Field(_) => "field".to_owned(),
        Declaration::Method(_) => "method".to_owned(),
    }
}

fn type_declaration_header(declaration: &TypeDeclaration) -> String {
    let mut rendered = String::new();

    for access in &declaration.access {
        write!(rendered, "{} ", access_level(*access)).expect("writing to a string cannot fail");
    }
    for modifier in &declaration.modifiers {
        write!(rendered, "{} ", modifier_name(*modifier)).expect("writing to a string cannot fail");
    }

    write!(
        rendered,
        "{} {}",
        type_kind(declaration.kind),
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

    rendered
}

fn write_type_relationships(declaration: &TypeDeclaration, depth: usize, output: &mut String) {
    let indentation = "  ".repeat(depth);

    if let Some(superclass) = &declaration.declared_superclass {
        writeln!(output, "{indentation}extends {}", type_ref(superclass))
            .expect("writing to a string cannot fail");
    }

    if !declaration.declared_superinterfaces.is_empty() {
        let relationship = match declaration.kind {
            Kind::Interface | Kind::AnnotationInterface => "extends",
            Kind::Class | Kind::Enum | Kind::Record => "implements",
        };
        writeln!(output, "{indentation}{relationship}").expect("writing to a string cannot fail");
        for interface in &declaration.declared_superinterfaces {
            writeln!(output, "{indentation}  {}", type_ref(interface))
                .expect("writing to a string cannot fail");
        }
    }
}

fn access_level(access: AccessLevel) -> &'static str {
    match access {
        AccessLevel::Public => "public",
        AccessLevel::Protected => "protected",
        AccessLevel::Private => "private",
    }
}

fn modifier_name(modifier: Modifier) -> &'static str {
    match modifier {
        Modifier::Abstract => "abstract",
        Modifier::Static => "static",
        Modifier::Final => "final",
        Modifier::Sealed => "sealed",
        Modifier::NonSealed => "non-sealed",
        Modifier::Strictfp => "strictfp",
    }
}

fn type_kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Class => "class",
        Kind::Enum => "enum",
        Kind::Record => "record",
        Kind::Interface => "interface",
        Kind::AnnotationInterface => "@interface",
    }
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

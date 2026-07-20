use std::{
    env,
    fmt::{Debug, Write},
    fs, io,
    path::PathBuf,
};

use model::JavaFile;
use parser::JavaParser;
use tree_sitter::{Node, Parser as TreeSitterParser};

#[allow(dead_code)]
#[path = "../../../crates/lang-java/src/model.rs"]
mod model;
#[path = "../../../crates/lang-java/src/parser.rs"]
mod parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = input_path()?;
    let source = fs::read_to_string(&path)?;

    let mut tree_sitter = TreeSitterParser::new();
    tree_sitter.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let tree = tree_sitter
        .parse(&source, None)
        .ok_or_else(|| io::Error::other("tree-sitter did not produce a tree"))?;

    println!("== source: {} ==", path.display());
    print!("{source}");
    if !source.ends_with('\n') {
        println!();
    }

    println!("\n== tree-sitter: {} ==", path.display());
    print!("{}", pretty_sexp(tree.root_node()));

    let mut java_parser = JavaParser::new();
    let java_file = java_parser.parse(&source);

    println!("\n== JavaFile: {} ==", path.display());
    print_java_file(&java_file);

    Ok(())
}

fn print_java_file(file: &JavaFile) {
    println!("JavaFile {{");
    print_debug_field("package", &file.package);
    print_debug_field("imports", &file.imports);
    println!(
        "  compilation_unit_scope: {:?}",
        file.compilation_unit_scope
    );
    println!(
        "  top_level_declarations: {:?}",
        file.top_level_declarations
    );
    println!("  declarations:");
    print_indexed(&file.declarations);
    println!("  lexical_scopes:");
    print_indexed(&file.lexical_scopes);
    println!("}}");
}

fn print_debug_field(name: &str, value: &impl Debug) {
    let rendered = format!("{value:#?}");
    let mut lines = rendered.lines();
    let first = lines.next().unwrap_or_default();
    println!("  {name}: {first}");
    for line in lines {
        println!("  {line}");
    }
}

fn print_indexed(values: &[impl Debug]) {
    for (index, value) in values.iter().enumerate() {
        let rendered = format!("{value:#?}");
        let mut lines = rendered.lines();
        let first = lines.next().unwrap_or_default();
        println!("    {index}: {first}");
        for line in lines {
            println!("    {line}");
        }
    }
}

fn pretty_sexp(root: Node<'_>) -> String {
    let mut output = String::new();
    write_node(root, None, 0, &mut output);
    output
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

fn input_path() -> Result<PathBuf, io::Error> {
    let mut arguments = env::args_os().skip(1);
    let path = arguments
        .next()
        .ok_or_else(|| io::Error::other("usage: java-inspect <file.java>"))?;

    if arguments.next().is_some() {
        return Err(io::Error::other("usage: java-inspect <file.java>"));
    }

    Ok(path.into())
}

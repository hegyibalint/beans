use std::{env, error::Error, fmt::Write, fs, io, path::PathBuf, process};

use tree_sitter::{Node, Parser};

fn main() {
    if let Err(error) = run() {
        eprintln!("ts-tree: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = input_path()?;
    let source = fs::read(&path)?;

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| io::Error::other("tree-sitter did not produce a tree"))?;

    print!("{}", pretty_tree(tree.root_node()));
    Ok(())
}

fn pretty_tree(root: Node<'_>) -> String {
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
        .ok_or_else(|| io::Error::other("usage: ts-tree <file.java>"))?;

    if arguments.next().is_some() {
        return Err(io::Error::other("usage: ts-tree <file.java>"));
    }

    Ok(path.into())
}

use std::path::Path;

use beans_core::{Location, Modifier, Signature, Symbol, SymbolId, SymbolKind};
use tree_sitter::{Node, Parser};

use crate::types::TypeRef;

/// Parse a Java source file and return all extracted symbols.
pub fn parse_java_file(path: &Path, source: &str) -> Vec<Symbol> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("failed to set Java language");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return vec![],
    };

    let root = tree.root_node();
    let source_bytes = source.as_bytes();

    let mut ctx = ParseContext {
        path,
        source: source_bytes,
        symbols: Vec::new(),
        package: String::new(),
        enclosing_stack: Vec::new(),
    };

    // First pass: find package declaration
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        if child.kind() == "package_declaration" {
            ctx.package = extract_package_name(child, source_bytes);
        }
    }

    // Second pass: extract symbols
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        extract_symbol(&mut ctx, child);
    }

    ctx.symbols
}

struct ParseContext<'a> {
    path: &'a Path,
    source: &'a [u8],
    symbols: Vec<Symbol>,
    package: String,
    /// Stack of (symbol_index, simple_name) for enclosing classes
    enclosing_stack: Vec<(usize, String)>,
}

fn extract_package_name(node: Node, source: &[u8]) -> String {
    // The package name is in a child that is either an identifier or scoped_identifier
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "scoped_identifier" | "identifier" => {
                return node_text(child, source).to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn node_text<'a>(node: Node, source: &'a [u8]) -> &'a str {
    std::str::from_utf8(&source[node.byte_range()]).unwrap_or("")
}

fn extract_symbol(ctx: &mut ParseContext, node: Node) {
    match node.kind() {
        "class_declaration" => extract_class_like(ctx, node, SymbolKind::Class),
        "interface_declaration" => extract_class_like(ctx, node, SymbolKind::Interface),
        "enum_declaration" => extract_enum(ctx, node),
        "record_declaration" => extract_class_like(ctx, node, SymbolKind::Record),
        "annotation_type_declaration" => extract_class_like(ctx, node, SymbolKind::Annotation),
        _ => {}
    }
}

fn build_fqn(ctx: &ParseContext, name: &str) -> String {
    let mut parts = Vec::new();
    if !ctx.package.is_empty() {
        parts.push(ctx.package.as_str());
    }
    for (_, enclosing_name) in &ctx.enclosing_stack {
        parts.push(enclosing_name.as_str());
    }
    parts.push(name);
    parts.join(".")
}

fn make_location(ctx: &ParseContext, node: Node) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        file: ctx.path.to_path_buf(),
        start_line: start.row as u32,
        start_col: start.column as u32,
        end_line: end.row as u32,
        end_col: end.column as u32,
    }
}

fn extract_modifiers(node: Node, _source: &[u8]) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    // In tree-sitter-java, modifiers is a positional child (no field name)
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "modifiers" {
            for j in 0..child.child_count() {
                let modifier_node = child.child(j).unwrap();
                if let Some(m) = parse_modifier(modifier_node.kind()) {
                    modifiers.push(m);
                }
            }
            break;
        }
    }
    modifiers
}

fn parse_modifier(text: &str) -> Option<Modifier> {
    match text {
        "public" => Some(Modifier::Public),
        "private" => Some(Modifier::Private),
        "protected" => Some(Modifier::Protected),
        "static" => Some(Modifier::Static),
        "abstract" => Some(Modifier::Abstract),
        "final" => Some(Modifier::Final),
        "sealed" => Some(Modifier::Sealed),
        "default" => Some(Modifier::Default),
        "synchronized" => Some(Modifier::Synchronized),
        "volatile" => Some(Modifier::Volatile),
        "transient" => Some(Modifier::Transient),
        "native" => Some(Modifier::Native),
        "strictfp" => Some(Modifier::Strictfp),
        _ => None,
    }
}

fn extract_class_like(ctx: &mut ParseContext, node: Node, kind: SymbolKind) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let fqn = build_fqn(ctx, &name);
    let modifiers = extract_modifiers(node, ctx.source);
    let location = make_location(ctx, node);

    // Extract type parameters for class signature
    let type_params = extract_type_parameters(node, ctx.source);
    let signature = if !type_params.is_empty() {
        Some(Signature::Class {
            type_parameters: type_params.iter().map(|s| beans_core::TypeParam::new(s)).collect(),
        })
    } else {
        None
    };

    let parent_idx = ctx.enclosing_stack.last().map(|(idx, _)| *idx);

    let sym_idx = ctx.symbols.len();
    ctx.symbols.push(Symbol {
        id: SymbolId(0), // placeholder, SymbolTable assigns real IDs
        fqn,
        name: name.clone(),
        kind,
        location: Some(location),
        modifiers,
        annotations: vec![],
        parent: parent_idx.map(|i| SymbolId(i)),
        children: vec![],
        relations: vec![],
        signature,
    });

    // Add as child of parent
    if let Some(parent_idx) = parent_idx {
        ctx.symbols[parent_idx]
            .children
            .push(SymbolId(sym_idx));
    }

    // Now parse body members
    ctx.enclosing_stack.push((sym_idx, name));
    extract_body_members(ctx, node);
    ctx.enclosing_stack.pop();
}

fn extract_enum(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let fqn = build_fqn(ctx, &name);
    let modifiers = extract_modifiers(node, ctx.source);
    let location = make_location(ctx, node);

    let parent_idx = ctx.enclosing_stack.last().map(|(idx, _)| *idx);

    let sym_idx = ctx.symbols.len();
    ctx.symbols.push(Symbol {
        id: SymbolId(0),
        fqn,
        name: name.clone(),
        kind: SymbolKind::Enum,
        location: Some(location),
        modifiers,
        annotations: vec![],
        parent: parent_idx.map(|i| SymbolId(i)),
        children: vec![],
        relations: vec![],
        signature: None,
    });

    if let Some(parent_idx) = parent_idx {
        ctx.symbols[parent_idx]
            .children
            .push(SymbolId(sym_idx));
    }

    // Parse enum body: constants + regular members
    ctx.enclosing_stack.push((sym_idx, name));
    if let Some(body) = node.child_by_field_name("body") {
        for i in 0..body.child_count() {
            let child = body.child(i).unwrap();
            match child.kind() {
                "enum_constant" => extract_enum_constant(ctx, child),
                "enum_body_declarations" => {
                    // Methods and fields inside enum are wrapped in this node
                    for j in 0..child.child_count() {
                        let decl = child.child(j).unwrap();
                        extract_body_member(ctx, decl);
                    }
                }
                _ => extract_body_member(ctx, child),
            }
        }
    }
    ctx.enclosing_stack.pop();
}

fn extract_enum_constant(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let fqn = build_fqn(ctx, &name);
    let location = make_location(ctx, node);
    let parent_idx = ctx.enclosing_stack.last().map(|(idx, _)| *idx);

    let sym_idx = ctx.symbols.len();
    ctx.symbols.push(Symbol {
        id: SymbolId(0),
        fqn,
        name,
        kind: SymbolKind::Field, // enum constants are fields
        location: Some(location),
        modifiers: vec![Modifier::Public, Modifier::Static, Modifier::Final],
        annotations: vec![],
        parent: parent_idx.map(|i| SymbolId(i)),
        children: vec![],
        relations: vec![],
        signature: None,
    });

    if let Some(parent_idx) = parent_idx {
        ctx.symbols[parent_idx]
            .children
            .push(SymbolId(sym_idx));
    }
}

fn extract_body_members(ctx: &mut ParseContext, node: Node) {
    // Find the body node (class_body, interface_body, etc.)
    let body = node
        .child_by_field_name("body")
        .or_else(|| {
            // Some nodes use different field names for the body
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                if child.kind().ends_with("_body") {
                    return Some(child);
                }
            }
            None
        });

    if let Some(body) = body {
        for i in 0..body.child_count() {
            let child = body.child(i).unwrap();
            extract_body_member(ctx, child);
        }
    }
}

fn extract_body_member(ctx: &mut ParseContext, node: Node) {
    match node.kind() {
        "method_declaration" => extract_method(ctx, node),
        "constructor_declaration" => extract_constructor(ctx, node),
        "field_declaration" => extract_fields(ctx, node),
        "class_declaration" => extract_class_like(ctx, node, SymbolKind::Class),
        "interface_declaration" => extract_class_like(ctx, node, SymbolKind::Interface),
        "enum_declaration" => extract_enum(ctx, node),
        "record_declaration" => extract_class_like(ctx, node, SymbolKind::Record),
        "annotation_type_declaration" => extract_class_like(ctx, node, SymbolKind::Annotation),
        _ => {}
    }
}

fn extract_method(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let fqn = build_fqn(ctx, &name);
    let modifiers = extract_modifiers(node, ctx.source);
    let location = make_location(ctx, node);

    let return_type = node
        .child_by_field_name("type")
        .map(|n| parse_type_ref(n, ctx.source))
        .unwrap_or(TypeRef::Void);

    let parameters = extract_formal_parameters(node, ctx.source);
    let type_params = extract_type_parameters(node, ctx.source);

    let signature = Signature::Method {
        return_type: return_type.to_core(),
        parameters: parameters
            .iter()
            .map(|(name, ty)| beans_core::MethodParam {
                name: name.clone(),
                param_type: ty.to_core(),
                is_varargs: false,
            })
            .collect(),
        type_parameters: type_params.iter().map(|s| beans_core::TypeParam::new(s)).collect(),
        throws: vec![],
    };

    let parent_idx = ctx.enclosing_stack.last().map(|(idx, _)| *idx);
    let sym_idx = ctx.symbols.len();

    ctx.symbols.push(Symbol {
        id: SymbolId(0),
        fqn,
        name,
        kind: SymbolKind::Method,
        location: Some(location),
        modifiers,
        annotations: vec![],
        parent: parent_idx.map(|i| SymbolId(i)),
        children: vec![],
        relations: vec![],
        signature: Some(signature),
    });

    if let Some(parent_idx) = parent_idx {
        ctx.symbols[parent_idx]
            .children
            .push(SymbolId(sym_idx));
    }
}

fn extract_constructor(ctx: &mut ParseContext, node: Node) {
    let name = match node.child_by_field_name("name") {
        Some(n) => node_text(n, ctx.source).to_string(),
        None => return,
    };

    let fqn = build_fqn(ctx, &name);
    let modifiers = extract_modifiers(node, ctx.source);
    let location = make_location(ctx, node);

    let parameters = extract_formal_parameters(node, ctx.source);
    let type_params = extract_type_parameters(node, ctx.source);

    let signature = Signature::Method {
        return_type: beans_core::TypeRef::Void,
        parameters: parameters
            .iter()
            .map(|(name, ty)| beans_core::MethodParam {
                name: name.clone(),
                param_type: ty.to_core(),
                is_varargs: false,
            })
            .collect(),
        type_parameters: type_params.iter().map(|s| beans_core::TypeParam::new(s)).collect(),
        throws: vec![],
    };

    let parent_idx = ctx.enclosing_stack.last().map(|(idx, _)| *idx);
    let sym_idx = ctx.symbols.len();

    ctx.symbols.push(Symbol {
        id: SymbolId(0),
        fqn,
        name,
        kind: SymbolKind::Constructor,
        location: Some(location),
        modifiers,
        annotations: vec![],
        parent: parent_idx.map(|i| SymbolId(i)),
        children: vec![],
        relations: vec![],
        signature: Some(signature),
    });

    if let Some(parent_idx) = parent_idx {
        ctx.symbols[parent_idx]
            .children
            .push(SymbolId(sym_idx));
    }
}

fn extract_fields(ctx: &mut ParseContext, node: Node) {
    let modifiers = extract_modifiers(node, ctx.source);

    let field_type = node
        .child_by_field_name("type")
        .map(|n| parse_type_ref(n, ctx.source))
        .unwrap_or(TypeRef::Simple("unknown".to_string()));

    let parent_idx = ctx.enclosing_stack.last().map(|(idx, _)| *idx);

    // Field declarations can have multiple declarators: `int a, b, c;`
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "variable_declarator" {
            let name = match child.child_by_field_name("name") {
                Some(n) => node_text(n, ctx.source).to_string(),
                None => continue,
            };

            // Check if declarator has array dimensions (e.g., `int arr[]`)
            let actual_type = check_declarator_array_dims(child, ctx.source, &field_type);

            let fqn = build_fqn(ctx, &name);
            let location = make_location(ctx, child);

            let sym_idx = ctx.symbols.len();
            ctx.symbols.push(Symbol {
                id: SymbolId(0),
                fqn,
                name,
                kind: SymbolKind::Field,
                location: Some(location),
                modifiers: modifiers.clone(),
                annotations: vec![],
                parent: parent_idx.map(|i| SymbolId(i)),
                children: vec![],
                relations: vec![],
                signature: Some(Signature::Field {
                    field_type: actual_type.to_core(),
                    constant_value: None,
                    initialized: false,
                }),
            });

            if let Some(parent_idx) = parent_idx {
                ctx.symbols[parent_idx]
                    .children
                    .push(SymbolId(sym_idx));
            }
        }
    }
}

fn check_declarator_array_dims<'a>(
    _declarator: Node,
    _source: &[u8],
    base_type: &TypeRef,
) -> TypeRef {
    // For simplicity, we rely on the type node for array dimensions
    // e.g. `int[] arr` is parsed with type `int[]` already
    base_type.clone()
}

fn extract_formal_parameters(node: Node, source: &[u8]) -> Vec<(String, TypeRef)> {
    let mut params = Vec::new();
    if let Some(params_node) = node.child_by_field_name("parameters") {
        for i in 0..params_node.child_count() {
            let child = params_node.child(i).unwrap();
            if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| node_text(n, source).to_string())
                    .unwrap_or_default();
                let ty = child
                    .child_by_field_name("type")
                    .map(|n| parse_type_ref(n, source))
                    .unwrap_or(TypeRef::Simple("unknown".to_string()));
                params.push((name, ty));
            }
        }
    }
    params
}

fn extract_type_parameters(node: Node, source: &[u8]) -> Vec<String> {
    let mut type_params = Vec::new();
    if let Some(tp_node) = node.child_by_field_name("type_parameters") {
        for i in 0..tp_node.child_count() {
            let child = tp_node.child(i).unwrap();
            if child.kind() == "type_parameter" {
                type_params.push(node_text(child, source).to_string());
            }
        }
    }
    type_params
}

fn parse_type_ref(node: Node, source: &[u8]) -> TypeRef {
    match node.kind() {
        "void_type" => TypeRef::Void,
        "integral_type" | "floating_point_type" | "boolean_type" => {
            TypeRef::Primitive(node_text(node, source).to_string())
        }
        "type_identifier" | "identifier" => TypeRef::Simple(node_text(node, source).to_string()),
        "scoped_type_identifier" => {
            TypeRef::Qualified(node_text(node, source).to_string())
        }
        "generic_type" => {
            // e.g., List<String>
            let base = node
                .child(0)
                .map(|n| node_text(n, source).to_string())
                .unwrap_or_default();
            let mut args = Vec::new();
            if let Some(type_args) = node.child_by_field_name("arguments") {
                // tree-sitter-java uses "type_arguments" as a child node
                for i in 0..type_args.child_count() {
                    let child = type_args.child(i).unwrap();
                    if child.kind() != "<" && child.kind() != ">" && child.kind() != "," {
                        args.push(parse_type_ref(child, source));
                    }
                }
            } else {
                // Fallback: iterate children looking for type_arguments
                for i in 0..node.child_count() {
                    let child = node.child(i).unwrap();
                    if child.kind() == "type_arguments" {
                        for j in 0..child.child_count() {
                            let arg = child.child(j).unwrap();
                            if arg.kind() != "<" && arg.kind() != ">" && arg.kind() != "," {
                                args.push(parse_type_ref(arg, source));
                            }
                        }
                    }
                }
            }
            TypeRef::Parameterized(base, args)
        }
        "array_type" => {
            // e.g., int[]
            if let Some(element) = node.child_by_field_name("element") {
                TypeRef::Array(Box::new(parse_type_ref(element, source)))
            } else if let Some(first_child) = node.child(0) {
                TypeRef::Array(Box::new(parse_type_ref(first_child, source)))
            } else {
                TypeRef::Array(Box::new(TypeRef::Simple(
                    node_text(node, source).to_string(),
                )))
            }
        }
        "wildcard" => TypeRef::Wildcard,
        _ => TypeRef::Simple(node_text(node, source).to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse(source: &str) -> Vec<Symbol> {
        parse_java_file(Path::new("Test.java"), source)
    }

    fn find_by_name<'a>(symbols: &'a [Symbol], name: &str) -> &'a Symbol {
        symbols
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("symbol '{}' not found", name))
    }

    #[test]
    fn test_parse_simple_class() {
        let source = r#"
package com.example;

public class Dog {
}
"#;
        let symbols = parse(source);
        let dog = find_by_name(&symbols, "Dog");
        assert_eq!(dog.fqn, "com.example.Dog");
        assert_eq!(dog.kind, SymbolKind::Class);
        assert!(dog.modifiers.contains(&Modifier::Public));
    }

    #[test]
    fn test_parse_class_with_members() {
        let source = r#"
package com.example;

public class Dog extends Animal implements Runnable {
    private String name;

    public Dog(String name) {
        this.name = name;
    }

    public String getName() {
        return name;
    }
}
"#;
        let symbols = parse(source);

        let dog = find_by_name(&symbols, "Dog");
        assert_eq!(dog.fqn, "com.example.Dog");
        assert_eq!(dog.kind, SymbolKind::Class);
        assert_eq!(dog.children.len(), 3); // field + constructor + method

        let name_field = find_by_name(&symbols, "name");
        assert_eq!(name_field.fqn, "com.example.Dog.name");
        assert_eq!(name_field.kind, SymbolKind::Field);
        assert!(name_field.modifiers.contains(&Modifier::Private));
        if let Some(Signature::Field { ref field_type, .. }) = name_field.signature {
            assert_eq!(field_type, "String");
        } else {
            panic!("expected Field signature");
        }

        // The class itself is also named Dog, so find the constructor specifically
        let ctor = symbols
            .iter()
            .find(|s| s.name == "Dog" && s.kind == SymbolKind::Constructor)
            .expect("constructor not found");
        assert_eq!(ctor.fqn, "com.example.Dog.Dog");
        if let Some(Signature::Method { ref parameters, .. }) = ctor.signature {
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].name, "name");
            assert_eq!(parameters[0].param_type, "String");
        } else {
            panic!("expected Method signature on constructor");
        }

        let get_name = find_by_name(&symbols, "getName");
        assert_eq!(get_name.fqn, "com.example.Dog.getName");
        assert_eq!(get_name.kind, SymbolKind::Method);
        if let Some(Signature::Method {
            ref return_type, ..
        }) = get_name.signature
        {
            assert_eq!(return_type, "String");
        } else {
            panic!("expected Method signature");
        }
    }

    #[test]
    fn test_parse_interface() {
        let source = r#"
package com.example;

public interface Repository {
    void save(String item);
    String findById(int id);
}
"#;
        let symbols = parse(source);
        let repo = find_by_name(&symbols, "Repository");
        assert_eq!(repo.fqn, "com.example.Repository");
        assert_eq!(repo.kind, SymbolKind::Interface);
        assert_eq!(repo.children.len(), 2);

        let save = find_by_name(&symbols, "save");
        assert_eq!(save.kind, SymbolKind::Method);
        if let Some(Signature::Method {
            ref return_type,
            ref parameters,
            ..
        }) = save.signature
        {
            assert_eq!(return_type, "void");
            assert_eq!(parameters.len(), 1);
        } else {
            panic!("expected Method signature");
        }
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
package com.example;

public enum Color {
    RED,
    GREEN,
    BLUE;

    public String displayName() {
        return name().toLowerCase();
    }
}
"#;
        let symbols = parse(source);
        let color = find_by_name(&symbols, "Color");
        assert_eq!(color.fqn, "com.example.Color");
        assert_eq!(color.kind, SymbolKind::Enum);

        // Enum constants
        let red = find_by_name(&symbols, "RED");
        assert_eq!(red.kind, SymbolKind::Field);
        assert_eq!(red.fqn, "com.example.Color.RED");

        let green = find_by_name(&symbols, "GREEN");
        assert_eq!(green.kind, SymbolKind::Field);

        // Method
        let display = find_by_name(&symbols, "displayName");
        assert_eq!(display.kind, SymbolKind::Method);
    }

    #[test]
    fn test_parse_nested_class() {
        let source = r#"
package com.example;

public class Outer {
    public class Inner {
        private int value;
    }
}
"#;
        let symbols = parse(source);

        let outer = find_by_name(&symbols, "Outer");
        assert_eq!(outer.fqn, "com.example.Outer");

        let inner = find_by_name(&symbols, "Inner");
        assert_eq!(inner.fqn, "com.example.Outer.Inner");
        assert_eq!(inner.kind, SymbolKind::Class);
        // Inner's parent should point to Outer's index
        assert!(inner.parent.is_some());

        let value = find_by_name(&symbols, "value");
        assert_eq!(value.fqn, "com.example.Outer.Inner.value");
    }

    #[test]
    fn test_parse_modifiers() {
        let source = r#"
package com.example;

public abstract class Base {
    protected static final int MAX = 100;
    public abstract void doWork();
}
"#;
        let symbols = parse(source);

        let base = find_by_name(&symbols, "Base");
        assert!(base.modifiers.contains(&Modifier::Public));
        assert!(base.modifiers.contains(&Modifier::Abstract));

        let max = find_by_name(&symbols, "MAX");
        assert!(max.modifiers.contains(&Modifier::Protected));
        assert!(max.modifiers.contains(&Modifier::Static));
        assert!(max.modifiers.contains(&Modifier::Final));

        let do_work = find_by_name(&symbols, "doWork");
        assert!(do_work.modifiers.contains(&Modifier::Public));
        assert!(do_work.modifiers.contains(&Modifier::Abstract));
    }

    #[test]
    fn test_parse_generic_class() {
        let source = r#"
package com.example;

public class Container<T> {
    private T value;

    public T getValue() {
        return value;
    }
}
"#;
        let symbols = parse(source);

        let container = find_by_name(&symbols, "Container");
        if let Some(Signature::Class {
            ref type_parameters,
        }) = container.signature
        {
            assert_eq!(type_parameters.len(), 1);
            assert_eq!(type_parameters[0].name, "T");
        } else {
            panic!("expected Class signature with type parameters");
        }
    }

    #[test]
    fn test_parse_generic_method_params() {
        let source = r#"
package com.example;

public class Utils {
    public void process(java.util.List<String> items) {}
}
"#;
        let symbols = parse(source);

        let process = find_by_name(&symbols, "process");
        if let Some(Signature::Method { ref parameters, .. }) = process.signature {
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].name, "items");
            // The type should contain the generic info
            assert!(
                parameters[0].param_type.to_string().contains("List"),
                "expected List in type, got: {}",
                parameters[0].param_type
            );
        } else {
            panic!("expected Method signature");
        }
    }

    #[test]
    fn test_parse_no_package() {
        let source = r#"
public class Simple {
    int x;
}
"#;
        let symbols = parse(source);
        let simple = find_by_name(&symbols, "Simple");
        assert_eq!(simple.fqn, "Simple");

        let x = find_by_name(&symbols, "x");
        assert_eq!(x.fqn, "Simple.x");
    }

    #[test]
    fn test_parse_multiple_field_declarators() {
        let source = r#"
package com.example;

public class Multi {
    private int a, b, c;
}
"#;
        let symbols = parse(source);

        let a = find_by_name(&symbols, "a");
        assert_eq!(a.fqn, "com.example.Multi.a");
        assert_eq!(a.kind, SymbolKind::Field);

        let b = find_by_name(&symbols, "b");
        assert_eq!(b.fqn, "com.example.Multi.b");

        let c = find_by_name(&symbols, "c");
        assert_eq!(c.fqn, "com.example.Multi.c");

        // The class should have 3 field children
        let multi = find_by_name(&symbols, "Multi");
        assert_eq!(multi.children.len(), 3);
    }

    #[test]
    fn test_parse_array_type() {
        let source = r#"
package com.example;

public class Arrays {
    private int[] numbers;
    private String[][] grid;
}
"#;
        let symbols = parse(source);

        let numbers = find_by_name(&symbols, "numbers");
        if let Some(Signature::Field { ref field_type, .. }) = numbers.signature {
            assert_eq!(field_type, "int[]");
        } else {
            panic!("expected Field signature");
        }
    }

    #[test]
    fn test_location_info() {
        let source = "package com.example;\n\npublic class Foo {\n}\n";
        let symbols = parse(source);
        let foo = find_by_name(&symbols, "Foo");
        let loc = foo.location.as_ref().unwrap();
        assert_eq!(loc.file, Path::new("Test.java"));
        assert_eq!(loc.start_line, 2); // 0-indexed line
    }

    #[test]
    fn test_parse_annotation_type() {
        let source = r#"
package com.example;

public @interface MyAnnotation {
    String value();
}
"#;
        let symbols = parse(source);
        let annot = find_by_name(&symbols, "MyAnnotation");
        assert_eq!(annot.kind, SymbolKind::Annotation);
        assert_eq!(annot.fqn, "com.example.MyAnnotation");
    }

    #[test]
    fn test_integration_parse_and_index() {
        use beans_core::SymbolTable;

        // A realistic multi-class Java file with interface, implementation, inner class, enum
        let source = r#"
package com.example.app;

public interface Service {
    void execute(String command);
    String status();
}

public class ServiceImpl implements Service {
    private final String name;
    private State currentState;

    public ServiceImpl(String name) {
        this.name = name;
        this.currentState = State.IDLE;
    }

    public void execute(String command) {
        currentState = State.RUNNING;
    }

    public String status() {
        return currentState.name();
    }

    public static class Config {
        private int timeout;
        private boolean verbose;

        public int getTimeout() {
            return timeout;
        }
    }

    public enum State {
        IDLE,
        RUNNING,
        STOPPED;
    }
}
"#;
        let symbols = parse_java_file(Path::new("src/com/example/app/Service.java"), source);

        // Insert all parsed symbols into a SymbolTable
        let mut table = SymbolTable::new();
        for sym in &symbols {
            table.insert(sym.clone());
        }

        // Verify FQN lookups
        assert!(table.lookup_by_fqn("com.example.app.Service").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.execute").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.status").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.name").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.Config").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.Config.timeout").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.Config.getTimeout").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.State").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.State.IDLE").is_some());
        assert!(table.lookup_by_fqn("com.example.app.ServiceImpl.State.RUNNING").is_some());

        // Verify package index
        let pkg_symbols = table.lookup_by_package("com.example.app");
        assert!(pkg_symbols.len() >= 2); // Service, ServiceImpl at minimum

        // Verify kind index
        let interfaces = table.lookup_by_kind(SymbolKind::Interface);
        assert_eq!(interfaces.len(), 1);
        let iface = table.get(interfaces[0]).unwrap();
        assert_eq!(iface.name, "Service");

        let enums = table.lookup_by_kind(SymbolKind::Enum);
        assert_eq!(enums.len(), 1);
        assert_eq!(table.get(enums[0]).unwrap().name, "State");

        // Verify children of ServiceImpl
        let impl_id = table.lookup_by_fqn("com.example.app.ServiceImpl").unwrap();
        let impl_sym = table.get(impl_id).unwrap();
        // name, currentState, constructor, execute, status, Config, State = 7 children
        assert_eq!(impl_sym.children.len(), 7);

        // Verify nested class parent
        let config_id = table.lookup_by_fqn("com.example.app.ServiceImpl.Config").unwrap();
        let config_sym = table.get(config_id).unwrap();
        assert_eq!(config_sym.kind, SymbolKind::Class);
        assert!(config_sym.modifiers.contains(&Modifier::Public));
        assert!(config_sym.modifiers.contains(&Modifier::Static));
        // Config has 3 children: timeout, verbose, getTimeout
        assert_eq!(config_sym.children.len(), 3);

        // Verify file index
        let file_syms = table.lookup_by_file(Path::new("src/com/example/app/Service.java"));
        assert_eq!(file_syms.len(), symbols.len());

        // Verify name lookup
        let executes = table.lookup_by_name("execute");
        assert_eq!(executes.len(), 2); // interface method + impl method
    }
}

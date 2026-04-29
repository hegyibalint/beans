use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{Symbol, SymbolId, SymbolKind, SymbolTable, Signature};

/// Represents a Java import statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Import {
    /// `import com.example.MyClass;`
    Single(String),
    /// `import com.example.*;`
    Wildcard(String),
    /// `import static com.example.Utils.MAX;`
    Static(String),
}

/// Given a simple name (e.g. "MyClass") at a cursor position, resolve it to an FQN
/// using the file's imports and current package context.
pub fn resolve_name(
    name: &str,
    file: &Path,
    file_imports: &HashMap<PathBuf, Vec<Import>>,
    file_packages: &HashMap<PathBuf, String>,
    table: &SymbolTable,
) -> Option<SymbolId> {
    // 1. Try as a fully qualified name directly
    if let Some(id) = table.lookup_by_fqn(name) {
        return Some(id);
    }

    let imports = file_imports.get(file).map(|v| v.as_slice()).unwrap_or(&[]);
    let current_package = file_packages
        .get(file)
        .map(|s| s.as_str())
        .unwrap_or("");

    // 2. Check explicit imports: `import com.example.MyClass;` matches "MyClass"
    for import in imports {
        if let Import::Single(fqn) = import {
            if fqn.ends_with(&format!(".{}", name)) || fqn == name {
                if let Some(id) = table.lookup_by_fqn(fqn) {
                    return Some(id);
                }
            }
        }
    }

    // 3. Check same package: current_package + "." + name
    if !current_package.is_empty() {
        let candidate = format!("{}.{}", current_package, name);
        if let Some(id) = table.lookup_by_fqn(&candidate) {
            return Some(id);
        }
    }

    // 4. Check wildcard imports: `import java.util.*;` -> try java.util.Name
    for import in imports {
        if let Import::Wildcard(package) = import {
            let candidate = format!("{}.{}", package, name);
            if let Some(id) = table.lookup_by_fqn(&candidate) {
                return Some(id);
            }
        }
    }

    // 5. Check static imports: `import static com.example.Utils.MAX;` matches "MAX"
    for import in imports {
        if let Import::Static(fqn) = import {
            if fqn.ends_with(&format!(".{}", name)) || fqn == name {
                if let Some(id) = table.lookup_by_fqn(fqn) {
                    return Some(id);
                }
            }
        }
    }

    // 6. Fallback: search by simple name (returns first match)
    let candidates = table.lookup_by_name(name);
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }

    None
}

/// Resolve a compound name like `Type.method` or `Type.field`.
/// First resolves the left part to a type, then looks up the right part as a child.
pub fn resolve_compound_name(
    text: &str,
    file: &Path,
    file_imports: &HashMap<PathBuf, Vec<Import>>,
    file_packages: &HashMap<PathBuf, String>,
    table: &SymbolTable,
) -> Option<SymbolId> {
    // Try the whole thing as an FQN first
    if let Some(id) = table.lookup_by_fqn(text) {
        return Some(id);
    }

    // Split on last dot: "Type.method" -> ("Type", "method")
    if let Some(dot_pos) = text.rfind('.') {
        let type_part = &text[..dot_pos];
        let member_part = &text[dot_pos + 1..];

        // Resolve the type part
        if let Some(type_id) = resolve_name(type_part, file, file_imports, file_packages, table) {
            // Look for the member among children
            let children = table.lookup_children(type_id);
            for child_id in children {
                if let Some(child) = table.get(child_id) {
                    if child.name == member_part {
                        return Some(child_id);
                    }
                }
            }
        }
    }

    // Fall back to simple name resolution
    resolve_name(text, file, file_imports, file_packages, table)
}

/// Find all symbols in the workspace whose FQN matches the given symbol.
/// Used for find-references: searches all files for references to a given FQN.
/// In v1, we return locations of all symbols with the same name (simple name match).
pub fn find_references_by_name(
    name: &str,
    table: &SymbolTable,
) -> Vec<SymbolId> {
    table.lookup_by_name(name)
}

/// Build a hover string for a symbol: its kind + FQN + signature.
pub fn build_hover_text(symbol: &Symbol) -> String {
    let kind_str = match symbol.kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::Record => "record",
        SymbolKind::Annotation => "@interface",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Field => "field",
        SymbolKind::Parameter => "parameter",
        SymbolKind::Package => "package",
        _ => "symbol",
    };

    match &symbol.signature {
        Some(Signature::Method {
            return_type,
            parameters,
            type_parameters,
            ..
        }) => {
            let tp = if type_parameters.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = type_parameters.iter().map(|t| t.name.as_str()).collect();
                format!("<{}>", names.join(", "))
            };
            let params: Vec<String> = parameters
                .iter()
                .map(|p| format!("{} {}", p.param_type, p.name))
                .collect();
            format!(
                "```java\n{}{} {}({})\n```\n\n{} `{}`",
                tp,
                return_type,
                symbol.name,
                params.join(", "),
                kind_str,
                symbol.fqn
            )
        }
        Some(Signature::Field { field_type, .. }) => {
            format!(
                "```java\n{} {}\n```\n\n{} `{}`",
                field_type, symbol.name, kind_str, symbol.fqn
            )
        }
        Some(Signature::Class { type_parameters }) => {
            let tp = if type_parameters.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = type_parameters.iter().map(|t| t.name.as_str()).collect();
                format!("<{}>", names.join(", "))
            };
            format!(
                "```java\n{} {}{}\n```\n\n`{}`",
                kind_str, symbol.name, tp, symbol.fqn
            )
        }
        Some(Signature::Record { type_parameters, .. }) => {
            let tp = if type_parameters.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = type_parameters.iter().map(|t| t.name.as_str()).collect();
                format!("<{}>", names.join(", "))
            };
            format!(
                "```java\nrecord {}{}\n```\n\n`{}`",
                symbol.name, tp, symbol.fqn
            )
        }
        Some(Signature::AnnotationElement { element_type, .. }) => {
            format!(
                "```java\n{} {}()\n```\n\n`{}`",
                element_type, symbol.name, symbol.fqn
            )
        }
        None => {
            format!(
                "```java\n{} {}\n```\n\n`{}`",
                kind_str, symbol.name, symbol.fqn
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Location, Modifier, Signature, Symbol, SymbolId, SymbolKind, SymbolTable};
    use std::path::PathBuf;

    fn make_table_with_classes() -> (SymbolTable, HashMap<PathBuf, Vec<Import>>, HashMap<PathBuf, String>) {
        let mut table = SymbolTable::new();
        let mut file_imports = HashMap::new();
        let mut file_packages = HashMap::new();

        // com.example.MyClass
        table.insert(Symbol {
            id: SymbolId(0),
            fqn: "com.example.MyClass".to_string(),
            name: "MyClass".to_string(),
            kind: SymbolKind::Class,
            location: Some(Location {
                file: PathBuf::from("src/MyClass.java"),
                start_line: 2,
                start_col: 0,
                end_line: 10,
                end_col: 0,
            }),
            modifiers: vec![Modifier::Public],
            annotations: vec![],
            parent: None,
            children: vec![],
            relations: vec![],
            signature: None,
        });

        // com.other.Helper
        table.insert(Symbol {
            id: SymbolId(0),
            fqn: "com.other.Helper".to_string(),
            name: "Helper".to_string(),
            kind: SymbolKind::Class,
            location: Some(Location {
                file: PathBuf::from("src/Helper.java"),
                start_line: 2,
                start_col: 0,
                end_line: 10,
                end_col: 0,
            }),
            modifiers: vec![Modifier::Public],
            annotations: vec![],
            parent: None,
            children: vec![],
            relations: vec![],
            signature: None,
        });

        let test_file = PathBuf::from("src/Test.java");
        file_imports.insert(
            test_file.clone(),
            vec![Import::Single("com.other.Helper".to_string())],
        );
        file_packages.insert(test_file, "com.example".to_string());

        (table, file_imports, file_packages)
    }

    #[test]
    fn test_resolve_by_fqn() {
        let (table, imports, packages) = make_table_with_classes();
        let file = Path::new("src/Test.java");
        let result = resolve_name("com.example.MyClass", file, &imports, &packages, &table);
        assert!(result.is_some());
        let sym = table.get(result.unwrap()).unwrap();
        assert_eq!(sym.fqn, "com.example.MyClass");
    }

    #[test]
    fn test_resolve_by_import() {
        let (table, imports, packages) = make_table_with_classes();
        let file = Path::new("src/Test.java");
        let result = resolve_name("Helper", file, &imports, &packages, &table);
        assert!(result.is_some());
        let sym = table.get(result.unwrap()).unwrap();
        assert_eq!(sym.fqn, "com.other.Helper");
    }

    #[test]
    fn test_resolve_same_package() {
        let (table, imports, packages) = make_table_with_classes();
        let file = Path::new("src/Test.java");
        // MyClass is in com.example, same as Test.java's package
        let result = resolve_name("MyClass", file, &imports, &packages, &table);
        assert!(result.is_some());
        let sym = table.get(result.unwrap()).unwrap();
        assert_eq!(sym.fqn, "com.example.MyClass");
    }

    #[test]
    fn test_resolve_wildcard_import() {
        let mut table = SymbolTable::new();
        table.insert(Symbol {
            id: SymbolId(0),
            fqn: "java.util.ArrayList".to_string(),
            name: "ArrayList".to_string(),
            kind: SymbolKind::Class,
            location: None,
            modifiers: vec![],
            annotations: vec![],
            parent: None,
            children: vec![],
            relations: vec![],
            signature: None,
        });

        let file = PathBuf::from("Test.java");
        let mut imports = HashMap::new();
        imports.insert(
            file.clone(),
            vec![Import::Wildcard("java.util".to_string())],
        );
        let packages = HashMap::new();

        let result = resolve_name("ArrayList", file.as_path(), &imports, &packages, &table);
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_compound_name() {
        let mut table = SymbolTable::new();
        let class_id = table.insert(Symbol {
            id: SymbolId(0),
            fqn: "com.example.MyClass".to_string(),
            name: "MyClass".to_string(),
            kind: SymbolKind::Class,
            location: Some(Location {
                file: PathBuf::from("MyClass.java"),
                start_line: 0,
                start_col: 0,
                end_line: 10,
                end_col: 0,
            }),
            modifiers: vec![],
            annotations: vec![],
            parent: None,
            children: vec![],
            relations: vec![],
            signature: None,
        });
        table.insert(Symbol {
            id: SymbolId(0),
            fqn: "com.example.MyClass.doWork".to_string(),
            name: "doWork".to_string(),
            kind: SymbolKind::Method,
            location: Some(Location {
                file: PathBuf::from("MyClass.java"),
                start_line: 5,
                start_col: 0,
                end_line: 8,
                end_col: 0,
            }),
            modifiers: vec![],
            annotations: vec![],
            parent: Some(class_id),
            children: vec![],
            relations: vec![],
            signature: None,
        });

        let file = PathBuf::from("Test.java");
        let mut imports = HashMap::new();
        imports.insert(
            file.clone(),
            vec![Import::Single("com.example.MyClass".to_string())],
        );
        let packages = HashMap::new();

        let result =
            resolve_compound_name("MyClass.doWork", file.as_path(), &imports, &packages, &table);
        assert!(result.is_some());
        let sym = table.get(result.unwrap()).unwrap();
        assert_eq!(sym.name, "doWork");
    }

    #[test]
    fn test_build_hover_text_method() {
        let sym = Symbol {
            id: SymbolId(0),
            fqn: "com.example.MyClass.doWork".to_string(),
            name: "doWork".to_string(),
            kind: SymbolKind::Method,
            location: None,
            modifiers: vec![Modifier::Public],
            annotations: vec![],
            parent: None,
            children: vec![],
            relations: vec![],
            signature: Some(Signature::Method {
                return_type: crate::TypeRef::simple("String"),
                parameters: vec![crate::MethodParam {
                    name: "input".to_string(),
                    param_type: crate::TypeRef::Primitive(crate::PrimitiveKind::Int),
                    is_varargs: false,
                }],
                type_parameters: vec![],
                throws: vec![],
            }),
        };
        let text = build_hover_text(&sym);
        assert!(text.contains("String"));
        assert!(text.contains("doWork"));
        assert!(text.contains("int input"));
        assert!(text.contains("com.example.MyClass.doWork"));
    }

    #[test]
    fn test_build_hover_text_class() {
        let sym = Symbol {
            id: SymbolId(0),
            fqn: "com.example.Container".to_string(),
            name: "Container".to_string(),
            kind: SymbolKind::Class,
            location: None,
            modifiers: vec![],
            annotations: vec![],
            parent: None,
            children: vec![],
            relations: vec![],
            signature: Some(Signature::Class {
                type_parameters: vec![crate::TypeParam::new("T")],
            }),
        };
        let text = build_hover_text(&sym);
        assert!(text.contains("Container<T>"));
        assert!(text.contains("com.example.Container"));
    }
}

mod parser;

use beans_lang_java_model::{
    File,
    declarations::{
        Declaration,
        types::{AccessLevel, Kind as TypeKind, Modifier, TypeDeclaration},
    },
    imports::{Import, ImportType},
    references::NameRef,
    scopes::ScopeIndex,
};
use tree_sitter::Node;

pub fn lower_into(content: &str) -> File {
    let tree = parser::parse(content);
    let root = tree.root_node();

    debug_assert_eq!(root.kind(), "program");

    let mut file = File::new();
    let mut cursor = root.walk();

    for node in root.named_children(&mut cursor) {
        match node.kind() {
            "package_declaration" => {
                file.package_name = lower_package_declaration(content, node);
            }

            "import_declaration" => {
                if let Some(import_declaration) = lower_import_declaration(content, node) {
                    file.imports.push(import_declaration);
                }
            }

            "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration" => {
                lower_type_declaration(content, node, File::ROOT_SCOPE_ID, &mut file)
            }

            _ => {}
        }
    }

    file
}

fn lower_type_declaration(content: &str, node: Node, _scope: ScopeIndex, file: &mut File) {
    debug_assert!(
        matches!(
            node.kind(),
            "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "record_declaration"
                | "annotation_type_declaration"
        ),
        "expected a type declaration, got `{}`",
        node.kind(),
    );

    let mut name = None;
    let mut kind = None;
    let mut access = Vec::new();
    let mut modifiers = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_extra() {
            continue;
        }

        match child.kind() {
            "modifiers" => {
                let mut modifier_cursor = child.walk();
                for modifier_node in child.children(&mut modifier_cursor) {
                    if let Some(access_level) = lower_type_access_modifier(modifier_node) {
                        access.push(access_level);
                    } else if let Some(modifier) = lower_type_modifier(modifier_node) {
                        modifiers.push(modifier);
                    }
                }
            }
            "class" | "enum" | "record" | "interface" | "@interface" => {
                kind = lower_type_kind(child);
            }
            "identifier" => {
                name = node_text(child, content);
            }
            "type_parameters" => {}
            "superclass" => {}
            "super_interfaces" | "extends_interfaces" => {}
            "permits" => {}
            "formal_parameters" => {}
            "class_body" | "enum_body" | "interface_body" | "annotation_type_body" => {}
            _ => {}
        }
    }

    let (Some(name), Some(kind)) = (name, kind) else {
        return;
    };

    file.declarations.push(Declaration::Type(TypeDeclaration {
        name,
        kind,
        extends: None,
        implements: Vec::new(),
        access,
        modifiers,
    }));
}

fn lower_type_access_modifier(node: Node) -> Option<AccessLevel> {
    match node.kind() {
        "public" => Some(AccessLevel::Public),
        "protected" => Some(AccessLevel::Protected),
        "private" => Some(AccessLevel::Private),
        _ => None,
    }
}

fn lower_type_modifier(node: Node) -> Option<Modifier> {
    match node.kind() {
        "abstract" => Some(Modifier::Abstract),
        "static" => Some(Modifier::Static),
        "final" => Some(Modifier::Final),
        "sealed" => Some(Modifier::Sealed),
        "non-sealed" => Some(Modifier::NonSealed),
        "strictfp" => Some(Modifier::Strictfp),
        _ => None,
    }
}

fn lower_type_kind(node: Node) -> Option<TypeKind> {
    match node.kind() {
        "class" => Some(TypeKind::Class),
        "enum" => Some(TypeKind::Enum),
        "record" => Some(TypeKind::Record),
        "interface" => Some(TypeKind::Interface),
        "@interface" => Some(TypeKind::AnnotationInterface),
        _ => None,
    }
}

fn node_text(node: Node, content: &str) -> Option<String> {
    node.utf8_text(content.as_bytes()).ok().map(str::to_owned)
}

fn lower_package_declaration(content: &str, node: Node) -> Option<NameRef> {
    debug_assert_eq!(node.kind(), "package_declaration");

    let mut cursor = node.walk();

    let name = node
        .named_children(&mut cursor)
        .find(|child| matches!(child.kind(), "identifier" | "scoped_identifier"))?;

    lower_identifier(content, name)
}

fn lower_import_declaration(content: &str, node: Node) -> Option<Import> {
    debug_assert_eq!(node.kind(), "import_declaration");

    let mut name = None;
    let mut is_static = false;
    let mut is_on_demand = false;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "static" => is_static = true,
            "asterisk" => is_on_demand = true,
            "identifier" | "scoped_identifier" => {
                name = lower_identifier(content, child);
            }
            _ => {}
        }
    }

    let typ = match (is_static, is_on_demand) {
        (false, false) => ImportType::SingleType,
        (false, true) => ImportType::OnDemandType,
        (true, false) => ImportType::SingleStaticType,
        (true, true) => ImportType::OnDemandStaticType,
    };

    Some(Import::new(name?, typ))
}

fn lower_identifier(content: &str, node: Node) -> Option<NameRef> {
    let mut components = collect_name_components(content, node)?;

    match components.len() {
        0 => None,
        1 => Some(NameRef::Simple(components.pop()?)),
        _ => Some(NameRef::Qualified(components)),
    }
}

fn collect_name_components(content: &str, node: Node) -> Option<Vec<String>> {
    match node.kind() {
        "identifier" => Some(vec![node_text(node, content)?]),

        "scoped_identifier" => {
            let scope = node.child_by_field_name("scope")?;
            let name = node.child_by_field_name("name")?;

            let mut components = collect_name_components(content, scope)?;
            components.extend(collect_name_components(content, name)?);
            Some(components)
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::lower_into;
    use beans_lang_java_model::declarations::{
        Declaration,
        types::{AccessLevel, Kind, Modifier},
    };

    const SOURCE: &str = r#"
package com.example.inventory;

// Single-type imports
import java.io.Closeable;
import java.io.IOException;

// Type imports on demand
import java.util.*;

// Single-static imports
import static java.util.Objects.requireNonNull;

// Static imports on demand
import static java.util.Comparator.*;

// Single-module imports
import module java.base;

@Deprecated
public final class Inventory<T extends Comparable<? super T>> implements Closeable {
    private static final int DEFAULT_CAPACITY = 16;
    private final Map<String, T> items;

    public Inventory(Map<String, T> items) {
        this.items = requireNonNull(items);
    }

    public List<T> sortedItems(String prefix) throws IOException {
        var result = new ArrayList<T>(DEFAULT_CAPACITY);
        items.forEach((name, item) -> {
            if (name.startsWith(prefix)) {
                result.add(item);
            }
        });
        result.sort(comparing(value -> value));
        telemetry.events.Publisher.publish(result.size());
        return result;
    }

    public String describe(Object candidate) {
        if (candidate instanceof String text && !text.isBlank()) {
            return text.strip();
        }
        return "unknown";
    }

    @Override
    public void close() throws IOException {
        items.clear();
    }

    public static final class Builder<U extends Comparable<? super U>> {
        private final Map<String, U> items = new HashMap<>();

        public Builder<U> add(String name, U item) {
            items.put(name, item);
            return this;
        }

        public Inventory<U> build() {
            return new Inventory<>(Map.copyOf(items));
        }
    }

    private final class Cursor {
        private int position;

        T next() {
            return new ArrayList<>(items.values()).get(position++);
        }
    }

    public record Snapshot<V>(List<V> values) {
        public Snapshot {
            values = List.copyOf(values);
        }
    }
}
"#;

    #[test]
    fn all_type_declaration_kinds_are_preserved() {
        let file = lower_into(
            "class C {} enum E {} record R() {} interface I {} @interface Annotation {}",
        );

        let kinds: Vec<_> = file
            .declarations
            .iter()
            .map(|declaration| match declaration {
                Declaration::Type(declaration) => declaration.kind,
                _ => panic!("expected a type declaration"),
            })
            .collect();

        assert_eq!(
            kinds,
            [
                Kind::Class,
                Kind::Enum,
                Kind::Record,
                Kind::Interface,
                Kind::AnnotationInterface,
            ]
        );
    }

    #[test]
    fn duplicate_and_conflicting_type_modifiers_are_preserved() {
        let file = lower_into(
            "public public protected private abstract abstract static final sealed non-sealed strictfp class DuplicateModifiers {}",
        );
        let Declaration::Type(declaration) = &file.declarations[0] else {
            panic!("expected a type declaration");
        };

        assert_eq!(declaration.name, "DuplicateModifiers");
        assert_eq!(
            declaration.access,
            [
                AccessLevel::Public,
                AccessLevel::Public,
                AccessLevel::Protected,
                AccessLevel::Private,
            ]
        );
        assert_eq!(
            declaration.modifiers,
            [
                Modifier::Abstract,
                Modifier::Abstract,
                Modifier::Static,
                Modifier::Final,
                Modifier::Sealed,
                Modifier::NonSealed,
                Modifier::Strictfp,
            ]
        );
    }

    #[test]
    fn compilation_unit_with_nested_declarations_and_expression_chains_lowers() {
        let file = lower_into(SOURCE);
        println!("{file:#?}");
    }
}

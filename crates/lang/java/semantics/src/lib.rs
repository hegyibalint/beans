mod parser;

use beans_lang_java_model::{
    File,
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

fn lower_type_declaration(_content: &str, node: Node, _scope: ScopeIndex, _file: &mut File) {
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
        "identifier" => {
            let text = node.utf8_text(content.as_bytes()).ok()?;
            Some(vec![text.to_owned()])
        }

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
    fn compilation_unit_with_nested_declarations_and_expression_chains_lowers() {
        let file = lower_into(SOURCE);
        println!("{file:#?}");
    }
}

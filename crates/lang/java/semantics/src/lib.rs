mod parser;

use beans_lang_java_model::{
    File,
    declarations::{
        Declaration, DeclarationIndex,
        types::{AccessLevel, Kind as TypeKind, Modifier, TypeDeclaration, TypeParameter},
    },
    imports::{Import, ImportType},
    references::{NameRef, PrimitiveType, TypeBound, TypeNameComponent, TypeRef},
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
                let _ = lower_type_declaration(content, node, File::ROOT_SCOPE_ID, &mut file);
            }

            _ => {}
        }
    }

    file
}

fn lower_type_declaration(
    content: &str,
    node: Node,
    parent_scope: ScopeIndex,
    file: &mut File,
) -> Option<DeclarationIndex> {
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

    let kind = lower_type_kind(node)?;
    let scope = file.new_scope(parent_scope);
    let mut declaration = TypeDeclaration::new(kind);
    let mut body = None;

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
                        declaration.access.push(access_level);
                    } else if let Some(modifier) = lower_type_modifier(modifier_node) {
                        declaration.modifiers.push(modifier);
                    }
                }
            }
            "class" | "enum" | "record" | "interface" | "@interface" => {}
            "identifier" => declaration.name = node_text(child, content),
            "type_parameters" => {
                declaration
                    .type_parameters
                    .extend(lower_type_parameters(content, child));
            }
            "superclass" => {
                declaration.declared_superclass = lower_single_type_clause(content, child);
            }
            "super_interfaces" | "extends_interfaces" => {
                declaration
                    .declared_superinterfaces
                    .extend(lower_type_list_clause(content, child));
            }
            "permits" => {}
            "formal_parameters" => {}
            "class_body" | "enum_body" | "interface_body" | "annotation_type_body" => {
                body = Some(child);
            }
            _ => {}
        }
    }

    let declaration = file.add_declaration(parent_scope, Declaration::Type(declaration));

    if let Some(body) = body {
        lower_type_body(content, body, scope, file);
    }

    Some(declaration)
}

fn lower_type_body(content: &str, node: Node, scope: ScopeIndex, file: &mut File) {
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration" => {
                let _ = lower_type_declaration(content, child, scope, file);
            }
            "enum_body_declarations" => lower_type_body(content, child, scope, file),
            _ => {}
        }
    }
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
        "class" | "class_declaration" => Some(TypeKind::Class),
        "enum" | "enum_declaration" => Some(TypeKind::Enum),
        "record" | "record_declaration" => Some(TypeKind::Record),
        "interface" | "interface_declaration" => Some(TypeKind::Interface),
        "@interface" | "annotation_type_declaration" => Some(TypeKind::AnnotationInterface),
        _ => None,
    }
}

fn lower_type_parameters(content: &str, node: Node) -> Vec<TypeParameter> {
    debug_assert_eq!(node.kind(), "type_parameters");

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() == "type_parameter")
        .filter_map(|child| lower_type_parameter(content, child))
        .collect()
}

fn lower_type_parameter(content: &str, node: Node) -> Option<TypeParameter> {
    debug_assert_eq!(node.kind(), "type_parameter");

    let mut name = None;
    let mut bounds = Vec::new();
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "type_identifier" => name = node_text(child, content),
            "type_bound" => bounds.extend(lower_type_bound(content, child)),
            _ => {}
        }
    }

    Some(TypeParameter {
        name: name?,
        bounds,
    })
}

fn lower_type_bound(content: &str, node: Node) -> Option<TypeBound> {
    debug_assert_eq!(node.kind(), "type_bound");

    let mut cursor = node.walk();
    let mut bounds = node
        .named_children(&mut cursor)
        .filter_map(|child| lower_type_ref(content, child));
    let primary = bounds.next()?;

    Some(TypeBound::Extends {
        primary,
        additional: bounds.collect(),
    })
}

fn lower_single_type_clause(content: &str, node: Node) -> Option<TypeRef> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| lower_type_ref(content, child))
}

fn lower_type_list_clause(content: &str, node: Node) -> Vec<TypeRef> {
    let mut clause_cursor = node.walk();
    let Some(type_list) = node
        .named_children(&mut clause_cursor)
        .find(|child| child.kind() == "type_list")
    else {
        return Vec::new();
    };

    let mut list_cursor = type_list.walk();
    type_list
        .named_children(&mut list_cursor)
        .filter_map(|child| lower_type_ref(content, child))
        .collect()
}

fn lower_type_ref(content: &str, node: Node) -> Option<TypeRef> {
    match node.kind() {
        "type_identifier" | "scoped_type_identifier" | "generic_type" => Some(TypeRef::Named {
            segments: lower_type_name_segments(content, node)?,
        }),
        "integral_type" | "floating_point_type" | "boolean_type" => {
            let primitive = match node_text(node, content)?.as_str() {
                "byte" => PrimitiveType::Byte,
                "short" => PrimitiveType::Short,
                "int" => PrimitiveType::Int,
                "long" => PrimitiveType::Long,
                "char" => PrimitiveType::Char,
                "float" => PrimitiveType::Float,
                "double" => PrimitiveType::Double,
                "boolean" => PrimitiveType::Boolean,
                _ => return None,
            };
            Some(TypeRef::Primitive(primitive))
        }
        "array_type" => {
            let element = lower_type_ref(content, node.child_by_field_name("element")?)?;
            let dimensions_node = node.child_by_field_name("dimensions")?;
            let mut cursor = dimensions_node.walk();
            let dimensions = dimensions_node
                .children(&mut cursor)
                .filter(|child| child.kind() == "[")
                .count();

            Some(TypeRef::Array {
                element: Box::new(element),
                dimensions,
            })
        }
        "annotated_type" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .find_map(|child| lower_type_ref(content, child))
        }
        "void_type" => Some(TypeRef::Void),
        _ => None,
    }
}

fn lower_type_name_segments(content: &str, node: Node) -> Option<Vec<TypeNameComponent>> {
    match node.kind() {
        "type_identifier" => Some(vec![TypeNameComponent {
            name: node_text(node, content)?,
            bounds: Vec::new(),
        }]),
        "scoped_type_identifier" => {
            let mut segments = Vec::new();
            let mut cursor = node.walk();

            for child in node.named_children(&mut cursor) {
                if let Some(mut child_segments) = lower_type_name_segments(content, child) {
                    segments.append(&mut child_segments);
                }
            }

            (!segments.is_empty()).then_some(segments)
        }
        "generic_type" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.named_children(&mut cursor).collect();
            let base = children
                .iter()
                .find_map(|child| lower_type_name_segments(content, *child))?;
            let mut segments = base;
            let arguments = children
                .iter()
                .find(|child| child.kind() == "type_arguments")
                .map(|arguments| lower_type_arguments(content, *arguments))
                .unwrap_or_default();
            segments.last_mut()?.bounds = arguments;
            Some(segments)
        }
        "annotated_type" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .find_map(|child| lower_type_name_segments(content, child))
        }
        _ => None,
    }
}

fn lower_type_arguments(content: &str, node: Node) -> Vec<TypeBound> {
    debug_assert_eq!(node.kind(), "type_arguments");

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter_map(|child| match child.kind() {
            "wildcard" => lower_wildcard(content, child),
            _ => lower_type_ref(content, child).map(|primary| TypeBound::Exact { primary }),
        })
        .collect()
}

fn lower_wildcard(content: &str, node: Node) -> Option<TypeBound> {
    debug_assert_eq!(node.kind(), "wildcard");

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    let referenced_type = children
        .iter()
        .find_map(|child| lower_type_ref(content, *child));

    match (
        children.iter().find(|child| child.kind() == "extends"),
        children.iter().find(|child| child.kind() == "super"),
        referenced_type,
    ) {
        (Some(_), None, Some(primary)) => Some(TypeBound::Extends {
            primary,
            additional: Vec::new(),
        }),
        (None, Some(_), Some(primary)) => Some(TypeBound::Super { primary }),
        (None, None, None) => Some(TypeBound::Unbounded),
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
    use beans_lang_java_model::{
        File,
        declarations::{
            Declaration,
            types::{AccessLevel, Kind, Modifier},
        },
        references::{TypeBound, TypeRef},
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
            .declarations()
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
        assert_eq!(
            file.scope(File::ROOT_SCOPE_ID)
                .unwrap()
                .declarations()
                .iter()
                .map(|index| index.as_usize())
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn duplicate_and_conflicting_type_modifiers_are_preserved() {
        let file = lower_into(
            "public public protected private abstract abstract static final sealed non-sealed strictfp class DuplicateModifiers {}",
        );
        let Declaration::Type(declaration) = &file.declarations()[0] else {
            panic!("expected a type declaration");
        };

        assert_eq!(declaration.name.as_deref(), Some("DuplicateModifiers"));
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
    fn type_parameter_without_declared_bound_has_empty_bounds() {
        let file = lower_into("class C<T> {}");
        let Declaration::Type(declaration) = &file.declarations()[0] else {
            panic!("expected a type declaration");
        };

        assert!(declaration.type_parameters[0].bounds.is_empty());
    }

    #[test]
    fn unbounded_wildcard_is_preserved() {
        let file = lower_into("class C implements Comparable<?> {}");
        let Declaration::Type(declaration) = &file.declarations()[0] else {
            panic!("expected a type declaration");
        };
        let TypeRef::Named { segments } = &declaration.declared_superinterfaces[0] else {
            panic!("expected a named superinterface");
        };

        assert_eq!(segments[0].bounds, [TypeBound::Unbounded]);
    }

    #[test]
    fn compilation_unit_with_nested_declarations_and_expression_chains_lowers() {
        let file = lower_into(SOURCE);
        println!("{file:#?}");
    }
}

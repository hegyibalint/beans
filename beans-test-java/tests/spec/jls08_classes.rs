use beans_core::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §8.1 — Class Declarations
mod jls_8_1_class_declarations {
    use super::*;

    #[test]
    fn class_with_members() {
        fixture()
            .file("com/example/Dog.java", r#"
                package com.example;
                public class <cur:class>Dog {
                    private String name;
                    private int age;
                    public String <cur:getter>getName() { return name; }
                    public int getAge() { return age; }
                }
            "#)
            .assert_at("class")
                .kind(SymbolKind::Class)
                .fqn("com.example.Dog")
                .children_include(&["name", "age", "getName", "getAge"])
                .children_count(4)
                .modifiers(vec![Modifier::Public])
            .assert_at("getter")
                .kind(SymbolKind::Method)
                .fqn("com.example.Dog.getName")
                .signature_return("String")
                .parent_fqn("com.example.Dog")
            .run();
    }
}

// §8.1.1 — Class Modifiers
mod jls_8_1_1_class_modifiers {
    // TODO: abstract, sealed, final, static classes
}

// §8.1.2 — Generic Classes and Type Parameters
mod jls_8_1_2_generic_classes {
    // TODO: type parameter parsing, bounds
}

// §8.1.3 — Inner Classes and Enclosing Instances
mod jls_8_1_3_inner_classes {
    // TODO: inner class parsing, enclosing instance
}

// §8.1.4 — Superclasses and Subclasses
mod jls_8_1_4_superclasses {
    // TODO: extends resolution
}

// §8.1.5 — Superinterfaces
mod jls_8_1_5_superinterfaces {
    // TODO: implements resolution
}

// §8.1.6 — Permitted Direct Subclasses
mod jls_8_1_6_sealed_classes {
    // TODO: sealed/permits parsing
}

// §8.3 — Field Declarations
mod jls_8_3_field_declarations {
    // TODO: field types, modifiers, multiple declarators
}

// §8.4 — Method Declarations
mod jls_8_4_method_declarations {
    use super::*;

    #[test]
    fn method_with_parameters() {
        fixture()
            .file("com/example/Calculator.java", r#"
                package com.example;
                public class Calculator {
                    public int <cur:add>add(int a, int b) { return a + b; }
                }
            "#)
            .assert_at("add")
                .kind(SymbolKind::Method)
                .signature_return("int")
                .signature_params(&[("a", "int"), ("b", "int")])
            .run();
    }
}

// §8.4.8 — Inheritance, Overriding, and Hiding
mod jls_8_4_8_inheritance_overriding {
    // TODO: override resolution, covariant return types
}

// §8.5 — Member Class and Interface Declarations
mod jls_8_5_member_classes {
    // TODO: nested/inner class declarations
}

// §8.8 — Constructor Declarations
mod jls_8_8_constructors {
    // TODO: constructor parsing, default constructors
}

// §8.9 — Enum Classes
mod jls_8_9_enums {
    // TODO: enum constants, enum body declarations
}

// §8.10 — Record Classes
mod jls_8_10_records {
    // TODO: record components, canonical constructors
}

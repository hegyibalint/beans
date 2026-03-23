mod prelude;

use beans_core::{Modifier, SymbolKind};
use prelude::fixture;

// =============================================================================
// Import resolution tests
// =============================================================================

#[test]
fn single_import_resolves() {
    fixture()
        .file("com/example/model/User.java", r#"
            package com.example.model;
            public class User {
                private String name;
                public String getName() { return name; }
            }
        "#)
        .file("com/example/service/UserService.java", r#"
            package com.example.service;
            import com.example.model.User;
            public class UserService {
                private <cur:field_type>User currentUser;
            }
        "#)
        .assert_at("field_type")
            .resolves_to("com.example.model.User")
            .kind(SymbolKind::Class)
        .run();
}

#[test]
fn wildcard_import_resolves() {
    fixture()
        .file("com/example/model/User.java", r#"
            package com.example.model;
            public class User {}
        "#)
        .file("com/example/model/Order.java", r#"
            package com.example.model;
            public class Order {}
        "#)
        .file("com/example/service/Service.java", r#"
            package com.example.service;
            import com.example.model.*;
            public class Service {
                private <cur:user_ref>User user;
                private <cur:order_ref>Order order;
            }
        "#)
        .assert_at("user_ref")
            .resolves_to("com.example.model.User")
        .assert_at("order_ref")
            .resolves_to("com.example.model.Order")
        .run();
}

#[test]
fn same_package_resolves_without_import() {
    fixture()
        .file("com/example/Foo.java", r#"
            package com.example;
            public class Foo {}
        "#)
        .file("com/example/Bar.java", r#"
            package com.example;
            public class Bar {
                private <cur:foo_ref>Foo foo;
            }
        "#)
        .assert_at("foo_ref")
            .resolves_to("com.example.Foo")
            .kind(SymbolKind::Class)
        .run();
}

// =============================================================================
// Symbol resolution tests
// =============================================================================

#[test]
fn class_children_and_signatures() {
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

// =============================================================================
// Multi-file cross-package resolution
// =============================================================================

#[test]
fn cross_package_resolution() {
    fixture()
        .file("com/example/model/User.java", r#"
            package com.example.model;
            public class User {
                private String name;
                public String getName() { return name; }
            }
        "#)
        .file("com/example/model/Order.java", r#"
            package com.example.model;
            public class Order {
                private int id;
            }
        "#)
        .file("com/example/service/UserService.java", r#"
            package com.example.service;
            import com.example.model.User;
            import com.example.model.Order;
            public class UserService {
                private <cur:user_field>User user;
                private <cur:order_field>Order order;
                public <cur:return_type>User getUser() { return user; }
            }
        "#)
        .assert_at("user_field")
            .resolves_to("com.example.model.User")
        .assert_at("order_field")
            .resolves_to("com.example.model.Order")
        .assert_at("return_type")
            .resolves_to("com.example.model.User")
        .run();
}

// =============================================================================
// Skip and expected failure demonstration
// =============================================================================

#[test]
fn skip_unimplemented_feature() {
    fixture()
        .file("com/example/Foo.java", r#"
            package com.example;
            public class Foo {
                public void <cur:method>doWork() {}
            }
        "#)
        .assert_at("method")
            .skip("completion testing not yet implemented")
            .kind(SymbolKind::Method)
        .run();
}

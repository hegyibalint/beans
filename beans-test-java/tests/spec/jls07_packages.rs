use beans_core::SymbolKind;

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §7.4 — Package Declarations
mod jls_7_4_package_declarations {
    use super::*;

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
}

// §7.5.1 — Single-Type-Import Declarations
mod jls_7_5_1_single_type_import {
    use super::*;

    #[test]
    fn basic() {
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
    fn cross_package_multiple_imports() {
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
}

// §7.5.2 — Type-Import-on-Demand Declarations
mod jls_7_5_2_type_import_on_demand {
    use super::*;

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
}

// §7.5.3 — Single-Static-Import Declarations
mod jls_7_5_3_single_static_import {
    // TODO: tests for static import of fields and methods
}

// §7.5.4 — Static-Import-on-Demand Declarations
mod jls_7_5_4_static_import_on_demand {
    // TODO: tests for static wildcard imports
}

// §7.6 — Top Level Class and Interface Declarations
mod jls_7_6_top_level_declarations {
    // TODO: tests for top-level class/interface visibility
}

use beans_core::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §7.3 — Compilation Units (implicit java.lang.* import)
mod jls_7_3_compilation_unit {
    use super::*;

    #[test]
    fn dot_completion_java_lang_implicit_import() {
        fixture()
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        <cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("String", SymbolKind::Class));
                assert!(items.has("Object", SymbolKind::Class));
                assert!(items.has("Integer", SymbolKind::Class));
                assert!(items.has("System", SymbolKind::Class));
            })
            .expected_failure("java.lang implicit import type completion not yet implemented")
            .run();
    }
}

// §7.4 — Package Declarations
mod jls_7_4_package_declarations {
    use super::*;

    // @keep — cross-file same-package; Foo field type resolves_to com.example.Foo without import
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

    // @keep — cross-file same-package; Alpha and Beta mutually reference each other's types
    #[test]
    fn same_package_multiple_classes_cross_reference() {
        fixture()
            .file("com/example/Alpha.java", r#"
                package com.example;
                public class Alpha {
                    private <cur:beta_ref>Beta beta;
                }
            "#)
            .file("com/example/Beta.java", r#"
                package com.example;
                public class Beta {
                    private <cur:alpha_ref>Alpha alpha;
                }
            "#)
            .assert_at("beta_ref")
                .resolves_to("com.example.Beta")
                .kind(SymbolKind::Class)
            .assert_at("alpha_ref")
                .resolves_to("com.example.Alpha")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — cross-file default package; Helper resolves_to the Helper class with no package declaration
    #[test]
    fn default_package_class_resolution() {
        fixture()
            .file("Helper.java", r#"
                public class Helper {
                    public static void help() {}
                }
            "#)
            .file("Main.java", r#"
                public class Main {
                    private <cur:helper_ref>Helper h;
                }
            "#)
            .assert_at("helper_ref")
                .resolves_to("Helper")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — cross-file; Widget resolves_to a class in a 5-segment deep package
    #[test]
    fn deeply_nested_package() {
        fixture()
            .file("com/example/deep/nested/pkg/Widget.java", r#"
                package com.example.deep.nested.pkg;
                public class Widget {}
            "#)
            .file("com/example/deep/nested/pkg/Factory.java", r#"
                package com.example.deep.nested.pkg;
                public class Factory {
                    public <cur:ret>Widget create() { return null; }
                }
            "#)
            .assert_at("ret")
                .resolves_to("com.example.deep.nested.pkg.Widget")
                .kind(SymbolKind::Class)
            .run();
    }

    #[test]
    fn dot_completion_same_package_types() {
        fixture()
            .file("com/example/Helper.java", r#"
                package com.example;
                public class Helper {
                    public void doWork() {}
                    public String describe() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Helper helper) {
                        helper.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("doWork", SymbolKind::Method));
                assert!(items.has("describe", SymbolKind::Method));
            })
            .expected_failure("same-package member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_package_private_visibility() {
        fixture()
            .file("com/example/Peer.java", r#"
                package com.example;
                public class Peer {
                    public void pubMethod() {}
                    void packageMethod() {}
                    private void privMethod() {}
                }
            "#)
            .file("com/example/Consumer.java", r#"
                package com.example;
                public class Consumer {
                    public void use(Peer peer) {
                        peer.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("pubMethod", SymbolKind::Method));
                assert!(items.has("packageMethod", SymbolKind::Method));
                assert!(!items.has("privMethod", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    // @keep — cross-file; same-package types used as both return type and parameter type in method signature
    #[test]
    fn same_package_type_in_method_signature() {
        fixture()
            .file("com/app/Request.java", r#"
                package com.app;
                public class Request {}
            "#)
            .file("com/app/Response.java", r#"
                package com.app;
                public class Response {}
            "#)
            .file("com/app/Handler.java", r#"
                package com.app;
                public class Handler {
                    public <cur:resp>Response handle(<cur:req>Request request) {
                        return null;
                    }
                }
            "#)
            .assert_at("resp")
                .resolves_to("com.app.Response")
                .kind(SymbolKind::Class)
            .assert_at("req")
                .resolves_to("com.app.Request")
                .kind(SymbolKind::Class)
            .run();
    }
}

// §7.5.1 — Single-Type-Import Declarations
mod jls_7_5_1_single_type_import {
    use super::*;

    // @keep — cross-package import; field type User resolves_to com.example.model.User via single-type import
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

    // @keep — cross-package; multiple imports (User, Order) resolve at field sites and return type
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

    // @keep — cross-package import from 5-segment package; Hasher resolves via single-type import
    #[test]
    fn import_from_deep_package() {
        fixture()
            .file("org/lib/internal/util/crypto/Hasher.java", r#"
                package org.lib.internal.util.crypto;
                public class Hasher {}
            "#)
            .file("com/app/Service.java", r#"
                package com.app;
                import org.lib.internal.util.crypto.Hasher;
                public class Service {
                    private <cur:hasher>Hasher hasher;
                }
            "#)
            .assert_at("hasher")
                .resolves_to("org.lib.internal.util.crypto.Hasher")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — import shadowing; explicit import of com.other.List wins over com.app.List in same package
    #[test]
    fn import_shadows_same_name_in_current_package() {
        fixture()
            .file("com/other/List.java", r#"
                package com.other;
                public class List {}
            "#)
            .file("com/app/List.java", r#"
                package com.app;
                public class List {}
            "#)
            .file("com/app/Consumer.java", r#"
                package com.app;
                import com.other.List;
                public class Consumer {
                    private <cur:list_ref>List items;
                }
            "#)
            .assert_at("list_ref")
                .resolves_to("com.other.List")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — cross-package import; extends clause type Animal resolves via single-type import
    #[test]
    fn imported_type_in_extends_clause() {
        fixture()
            .file("com/base/Animal.java", r#"
                package com.base;
                public class Animal {}
            "#)
            .file("com/zoo/Dog.java", r#"
                package com.zoo;
                import com.base.Animal;
                public class Dog extends <cur:parent>Animal {}
            "#)
            .assert_at("parent")
                .resolves_to("com.base.Animal")
                .kind(SymbolKind::Class)
            .run();
    }

    #[test]
    fn dot_completion_via_single_type_import() {
        fixture()
            .file("com/example/model/User.java", r#"
                package com.example.model;
                public class User {
                    private String secret;
                    public String getName() { return null; }
                    public int getAge() { return 0; }
                }
            "#)
            .file("com/example/app/App.java", r#"
                package com.example.app;
                import com.example.model.User;
                public class App {
                    public void run(User user) {
                        user.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getAge", SymbolKind::Method));
                assert!(!items.has("secret", SymbolKind::Field));
            })
            .expected_failure("cross-package member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_import_statement_type_names() {
        fixture()
            .file("com/example/model/User.java", r#"
                package com.example.model;
                public class User {}
            "#)
            .file("com/example/model/Order.java", r#"
                package com.example.model;
                public class Order {}
            "#)
            .file("com/example/model/Product.java", r#"
                package com.example.model;
                public class Product {}
            "#)
            .file("com/example/app/App.java", r#"
                package com.example.app;
                import com.example.model.<cur>
                public class App {}
            "#)
            .complete_default(|items| {
                assert!(items.has("User", SymbolKind::Class));
                assert!(items.has("Order", SymbolKind::Class));
                assert!(items.has("Product", SymbolKind::Class));
            })
            .expected_failure("import statement type completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_new_expression_type_context() {
        fixture()
            .file("com/example/model/User.java", r#"
                package com.example.model;
                public class User {
                    public User(String name) {}
                    public User() {}
                }
            "#)
            .file("com/example/app/App.java", r#"
                package com.example.app;
                import com.example.model.User;
                public class App {
                    public void run() {
                        Object u = new <cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("User", SymbolKind::Class));
            })
            .expected_failure("new expression type completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_explicit_import_shadows_java_lang() {
        fixture()
            .file("com/example/String.java", r#"
                package com.example;
                public class String {}
            "#)
            .file("com/example/app/App.java", r#"
                package com.example.app;
                import com.example.String;
                public class App {
                    private <cur>String s;
                }
            "#)
            .complete_default(|items| {
                // com.example.String should be offered, not java.lang.String
                assert!(items.has("String", SymbolKind::Class));
            })
            .expected_failure("import shadowing of java.lang types in completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_enum_members_via_import() {
        fixture()
            .file("com/model/Status.java", r#"
                package com.model;
                public enum Status {
                    ACTIVE, INACTIVE
                }
            "#)
            .file("com/app/App.java", r#"
                package com.app;
                import com.model.Status;
                public class App {
                    public void run() {
                        Status.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("ACTIVE", SymbolKind::Field));
                assert!(items.has("INACTIVE", SymbolKind::Field));
                assert!(items.has("values", SymbolKind::Method));
                assert!(items.has("valueOf", SymbolKind::Method));
            })
            .expected_failure("enum member dot completion via import not yet implemented")
            .run();
    }

    // @keep — cross-package import; Config type in local variable declaration resolves via import
    #[test]
    fn imported_type_in_local_variable() {
        fixture()
            .file("com/example/model/Config.java", r#"
                package com.example.model;
                public class Config {}
            "#)
            .file("com/example/app/Boot.java", r#"
                package com.example.app;
                import com.example.model.Config;
                public class Boot {
                    public void start() {
                        <cur:local_type>Config cfg = null;
                    }
                }
            "#)
            .assert_at("local_type")
                .resolves_to("com.example.model.Config")
                .kind(SymbolKind::Class)
            .run();
    }
}

// §7.5.2 — Type-Import-on-Demand Declarations
mod jls_7_5_2_type_import_on_demand {
    use super::*;

    // @keep — wildcard import; User and Order both resolve via import com.example.model.*
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

    // @keep — import priority; explicit import com.b.Util wins over wildcard import com.a.*
    #[test]
    fn explicit_import_wins_over_wildcard() {
        fixture()
            .file("com/a/Util.java", r#"
                package com.a;
                public class Util {}
            "#)
            .file("com/b/Util.java", r#"
                package com.b;
                public class Util {}
            "#)
            .file("com/app/App.java", r#"
                package com.app;
                import com.a.*;
                import com.b.Util;
                public class App {
                    private <cur:util_ref>Util u;
                }
            "#)
            .assert_at("util_ref")
                .resolves_to("com.b.Util")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — wildcard import; extends clause Shape resolves via import com.base.*
    #[test]
    fn wildcard_import_in_extends() {
        fixture()
            .file("com/base/Shape.java", r#"
                package com.base;
                public class Shape {}
            "#)
            .file("com/draw/Circle.java", r#"
                package com.draw;
                import com.base.*;
                public class Circle extends <cur:shape>Shape {}
            "#)
            .assert_at("shape")
                .resolves_to("com.base.Shape")
                .kind(SymbolKind::Class)
            .run();
    }

    #[test]
    fn dot_completion_via_wildcard_import() {
        fixture()
            .file("com/example/model/Order.java", r#"
                package com.example.model;
                public class Order {
                    public int getId() { return 0; }
                    public String getStatus() { return null; }
                }
            "#)
            .file("com/example/app/App.java", r#"
                package com.example.app;
                import com.example.model.*;
                public class App {
                    public void run(Order order) {
                        order.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getId", SymbolKind::Method));
                assert!(items.has("getStatus", SymbolKind::Method));
            })
            .expected_failure("cross-package member completion via wildcard import not yet implemented")
            .run();
    }

    // @keep — wildcard import; implements Printable resolves via import com.api.*
    #[test]
    fn wildcard_import_in_implements() {
        fixture()
            .file("com/api/Printable.java", r#"
                package com.api;
                public interface Printable {}
            "#)
            .file("com/impl_pkg/Report.java", r#"
                package com.impl_pkg;
                import com.api.*;
                public class Report implements <cur:iface>Printable {}
            "#)
            .assert_at("iface")
                .resolves_to("com.api.Printable")
                .kind(SymbolKind::Interface)
            .run();
    }
}

// §7.5.3 — Single-Static-Import Declarations
mod jls_7_5_3_single_static_import {
    use super::*;

    // @keep — static import of field; MAX_SIZE usage resolves_to Constants.MAX_SIZE via import static
    #[test]
    fn static_import_of_field() {
        fixture()
            .file("com/example/Constants.java", r#"
                package com.example;
                public class Constants {
                    public static final int MAX_SIZE = 100;
                }
            "#)
            .file("com/app/App.java", r#"
                package com.app;
                import static com.example.Constants.MAX_SIZE;
                public class App {
                    private int limit = <cur:max>MAX_SIZE;
                }
            "#)
            .assert_at("max")
                .resolves_to("com.example.Constants.MAX_SIZE")
                .kind(SymbolKind::Field)
            .run();
    }

    // @keep — static import of method; clamp() call resolves_to MathUtils.clamp via import static
    #[test]
    fn static_import_of_method() {
        fixture()
            .file("com/example/MathUtils.java", r#"
                package com.example;
                public class MathUtils {
                    public static int clamp(int val, int min, int max) {
                        return Math.min(Math.max(val, min), max);
                    }
                }
            "#)
            .file("com/app/Calc.java", r#"
                package com.app;
                import static com.example.MathUtils.clamp;
                public class Calc {
                    public int bounded(int x) {
                        return <cur:clamp_call>clamp(x, 0, 100);
                    }
                }
            "#)
            .assert_at("clamp_call")
                .resolves_to("com.example.MathUtils.clamp")
                .kind(SymbolKind::Method)
            .run();
    }

    #[test]
    fn dot_completion_static_import_members() {
        fixture()
            .file("com/example/MathUtils.java", r#"
                package com.example;
                public class MathUtils {
                    public static int clamp(int val, int min, int max) { return 0; }
                    public static double normalize(double val) { return 0.0; }
                    private static int helper(int x) { return x; }
                }
            "#)
            .file("com/app/Calc.java", r#"
                package com.app;
                import static com.example.MathUtils.<cur>
                public class Calc {}
            "#)
            .complete_default(|items| {
                assert!(items.has("clamp", SymbolKind::Method));
                assert!(items.has("normalize", SymbolKind::Method));
                assert!(!items.has("helper", SymbolKind::Method));
            })
            .expected_failure("static import member completion not yet implemented")
            .run();
    }

    // @keep — static import of enum constant; RED usage resolves_to Color.RED via import static
    #[test]
    fn static_import_of_enum_constant() {
        fixture()
            .file("com/example/Color.java", r#"
                package com.example;
                public enum Color {
                    RED, GREEN, BLUE
                }
            "#)
            .file("com/app/Painter.java", r#"
                package com.app;
                import static com.example.Color.RED;
                public class Painter {
                    private Color defaultColor = <cur:red>RED;
                }
            "#)
            .assert_at("red")
                .resolves_to("com.example.Color.RED")
                .kind(SymbolKind::Field)
            .run();
    }
}

// §7.5.4 — Static-Import-on-Demand Declarations
mod jls_7_5_4_static_import_on_demand {
    use super::*;

    // @keep — static wildcard import; MIN and MAX resolve via import static com.example.Limits.*
    #[test]
    fn static_wildcard_import_of_fields() {
        fixture()
            .file("com/example/Limits.java", r#"
                package com.example;
                public class Limits {
                    public static final int MIN = 0;
                    public static final int MAX = 1000;
                }
            "#)
            .file("com/app/Validator.java", r#"
                package com.app;
                import static com.example.Limits.*;
                public class Validator {
                    public boolean isValid(int v) {
                        return v >= <cur:min_ref>MIN && v <= <cur:max_ref>MAX;
                    }
                }
            "#)
            .assert_at("min_ref")
                .resolves_to("com.example.Limits.MIN")
                .kind(SymbolKind::Field)
            .assert_at("max_ref")
                .resolves_to("com.example.Limits.MAX")
                .kind(SymbolKind::Field)
            .run();
    }

    #[test]
    fn dot_completion_static_wildcard_unqualified() {
        fixture()
            .file("com/example/Constants.java", r#"
                package com.example;
                public class Constants {
                    public static final int MAX = 100;
                    public static final int MIN = 0;
                }
            "#)
            .file("com/app/App.java", r#"
                package com.app;
                import static com.example.Constants.*;
                public class App {
                    public void run() {
                        int x = <cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("MAX", SymbolKind::Field));
                assert!(items.has("MIN", SymbolKind::Field));
            })
            .expected_failure("unqualified static wildcard import completion not yet implemented")
            .run();
    }

    // @keep — cross-file: static wildcard import resolves multiple static methods (format, wrap)
    #[test]
    fn static_wildcard_import_of_methods() {
        fixture()
            .file("com/example/Helpers.java", r#"
                package com.example;
                public class Helpers {
                    public static String format(String s) { return s.trim(); }
                    public static String wrap(String s) { return "[" + s + "]"; }
                }
            "#)
            .file("com/app/Formatter.java", r#"
                package com.app;
                import static com.example.Helpers.*;
                public class Formatter {
                    public String apply(String input) {
                        return <cur:wrap_call>wrap(<cur:format_call>format(input));
                    }
                }
            "#)
            .assert_at("format_call")
                .resolves_to("com.example.Helpers.format")
                .kind(SymbolKind::Method)
            .assert_at("wrap_call")
                .resolves_to("com.example.Helpers.wrap")
                .kind(SymbolKind::Method)
            .run();
    }

    // @keep — cross-file: explicit static import takes precedence over static wildcard
    #[test]
    fn explicit_static_import_wins_over_static_wildcard() {
        fixture()
            .file("com/a/Consts.java", r#"
                package com.a;
                public class Consts {
                    public static final String NAME = "a";
                }
            "#)
            .file("com/b/Consts.java", r#"
                package com.b;
                public class Consts {
                    public static final String NAME = "b";
                }
            "#)
            .file("com/app/App.java", r#"
                package com.app;
                import static com.a.Consts.*;
                import static com.b.Consts.NAME;
                public class App {
                    private String val = <cur:name_ref>NAME;
                }
            "#)
            .assert_at("name_ref")
                .resolves_to("com.b.Consts.NAME")
                .kind(SymbolKind::Field)
            .run();
    }
}

// §7.6 — Top Level Class and Interface Declarations
mod jls_7_6_top_level_declarations {
    use super::*;

    // @keep — single file with two top-level classes: Helper resolves by simple name from Multi
    #[test]
    fn multiple_top_level_classes_in_same_file() {
        fixture()
            .file("com/example/Multi.java", r#"
                package com.example;
                public class Multi {
                    private <cur:helper_ref>Helper h;
                }
                class Helper {}
            "#)
            .assert_at("helper_ref")
                .resolves_to("com.example.Helper")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — record kind (SymbolKind::Record) detection is non-obvious; verifies Record kind correctly assigned
    #[test]
    fn top_level_record() {
        fixture()
            .file("com/example/Point.java", r#"
                package com.example;
                public record <cur:rec>Point(int x, int y) {}
            "#)
            .assert_at("rec")
                .fqn("com.example.Point")
                .kind(SymbolKind::Record)
                .name("Point")
                .modifiers(vec![Modifier::Public])
            .run();
    }

    // @keep — annotation kind (SymbolKind::Annotation) detection is non-obvious; verifies Annotation kind correctly assigned
    #[test]
    fn top_level_annotation() {
        fixture()
            .file("com/example/Marker.java", r#"
                package com.example;
                public @interface <cur:ann>Marker {}
            "#)
            .assert_at("ann")
                .fqn("com.example.Marker")
                .kind(SymbolKind::Annotation)
                .name("Marker")
                .modifiers(vec![Modifier::Public])
            .run();
    }
}

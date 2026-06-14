use beans::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §6.2 — Names and Identifiers
mod jls_6_2_names_and_identifiers {
    use super::*;

    /// Simple name resolution within same package.
    // @keep — cross-file resolution; cursor on field type usage resolves_to Logger in same package
    #[test]
    fn simple_name_resolves_in_same_package() {
        fixture()
            .file(
                "com/example/Logger.java",
                r#"
                package com.example;
                public class Logger {
                    public void log(String msg) {}
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    private <cur:simple>Logger logger;
                }
            "#,
            )
            .assert_at("simple")
            .resolves_to("com.example.Logger")
            .kind(SymbolKind::Class)
            .run();
    }

    /// Qualified name via import resolves to the target class.
    // @keep — cross-file; cursor on imported class name used in expression resolves_to StringUtils
    #[test]
    fn qualified_name_via_import() {
        fixture()
            .file(
                "com/example/util/StringUtils.java",
                r#"
                package com.example.util;
                public class StringUtils {
                    public static String trim(String s) { return s.trim(); }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                import com.example.util.StringUtils;
                public class App {
                    public void run() {
                        <cur:ref>StringUtils.trim("  hello  ");
                    }
                }
            "#,
            )
            .assert_at("ref")
            .resolves_to("com.example.util.StringUtils")
            .kind(SymbolKind::Class)
            .run();
    }

    /// Qualified field access: Constants.MAX_SIZE resolves to the field.
    // @keep — cross-file; `access` cursor on Constants.MAX_SIZE usage resolves_to the field
    #[test]
    fn qualified_field_access_via_class_name() {
        fixture()
            .file(
                "com/example/Constants.java",
                r#"
                package com.example;
                public class Constants {
                    public static final int <cur:decl>MAX_SIZE = 100;
                }
            "#,
            )
            .file(
                "com/example/Processor.java",
                r#"
                package com.example;
                public class Processor {
                    private int limit = Constants.<cur:access>MAX_SIZE;
                }
            "#,
            )
            .assert_at("decl")
            .kind(SymbolKind::Field)
            .fqn("com.example.Constants.MAX_SIZE")
            .assert_at("access")
            .resolves_to("com.example.Constants.MAX_SIZE")
            .kind(SymbolKind::Field)
            .run();
    }

    /// Member access on a field should resolve, and hover should show the field type.
    /// Also exercises §15.11 (field access expressions).
    // @keep — cross-file; cursor on `this.address` usage resolves_to Person.address field
    #[test]
    fn member_access_hover_shows_field_info() {
        fixture()
            .file(
                "com/example/Address.java",
                r#"
                package com.example;
                public class Address {
                    public String city;
                }
            "#,
            )
            .file(
                "com/example/Person.java",
                r#"
                package com.example;
                public class Person {
                    public Address address;
                    public void print() {
                        String c = this.<cur:addr_ref>address;
                    }
                }
            "#,
            )
            .assert_at("addr_ref")
            .resolves_to("com.example.Person.address")
            .hover_contains("Address")
            .run();
    }
}

// §6.3 — Scope of a Declaration
mod jls_6_3_scope {
    use super::*;

    /// Fields are visible throughout the class body, even before their textual declaration.
    // @keep — cursor on forward field reference in method body resolves_to Counter.count (usage before decl)
    #[test]
    fn field_visible_before_declaration() {
        fixture()
            .file(
                "com/example/Counter.java",
                r#"
                package com.example;
                public class Counter {
                    public int getCount() {
                        return <cur:forward_ref>count;
                    }
                    private int count;
                }
            "#,
            )
            .assert_at("forward_ref")
            .resolves_to("com.example.Counter.count")
            .kind(SymbolKind::Field)
            .run();
    }

    /// Same-package types appear in type-position completions without import.
    #[test]
    fn same_package_types_visible_without_import() {
        fixture()
            .file(
                "com/example/Logger.java",
                r#"
                package com.example;
                public class Logger {}
            "#,
            )
            .file(
                "com/example/Config.java",
                r#"
                package com.example;
                public class Config {}
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    private <cur> field;
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("Logger", SymbolKind::Class));
                assert!(items.has("Config", SymbolKind::Class));
            })
            .expected_failure("type-position completion not yet implemented")
            .run();
    }

    /// Single-type-imported class appears in type-position completions.
    #[test]
    fn imported_type_visible_in_type_position() {
        fixture()
            .file(
                "com/example/util/StringUtils.java",
                r#"
                package com.example.util;
                public class StringUtils {}
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                import com.example.util.StringUtils;
                public class App {
                    private <cur> field;
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("StringUtils", SymbolKind::Class));
            })
            .expected_failure("type-position completion not yet implemented")
            .run();
    }

    /// A class declared in a separate file in the same package is visible at a
    /// type-position cursor, even if that file is processed after the file with the cursor.
    #[test]
    fn forward_reference_same_package_type_visible() {
        fixture()
            .file(
                "com/example/First.java",
                r#"
                package com.example;
                public class First {
                    private <cur> ref;
                }
            "#,
            )
            .file(
                "com/example/Second.java",
                r#"
                package com.example;
                public class Second {}
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("Second", SymbolKind::Class));
            })
            .expected_failure("type-position completion not yet implemented")
            .run();
    }

    /// Variables declared in a for-loop header are in scope inside the loop body,
    /// and out of scope after the loop ends. The class field is used as the positive
    /// anchor since no Variable SymbolKind exists yet.
    #[test]
    fn for_loop_variable_out_of_scope_after_loop() {
        fixture()
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public int count;
                    public void test() {
                        for (int i = 0; i < 10; i++) {
                            int inside = i * 2;
                            <cur:in_loop>
                        }
                        <cur:after_loop>
                    }
                }
            "#,
            )
            .complete("in_loop", |items| {
                // Field is always in scope — positive anchor to drive expected_failure
                assert!(items.has("count", SymbolKind::Field));
                assert!(items.has("test", SymbolKind::Method));
            })
            .expected_failure("local variable scope in completion not yet implemented")
            .complete("after_loop", |items| {
                // Field and method still visible after the loop
                assert!(items.has("count", SymbolKind::Field));
                assert!(items.has("test", SymbolKind::Method));
                // `i` and `inside` are out of scope here; no Variable kind to assert on
            })
            .expected_failure("local variable scope in completion not yet implemented")
            .run();
    }

    /// Reference to a method parameter inside the body — parameters not yet resolved.
    // @keep — cursor on parameter reference `a` in method body (usage site, not declaration)
    #[test]
    fn method_parameter_reference_in_body() {
        fixture()
            .file(
                "com/example/Calculator.java",
                r#"
                package com.example;
                public class Calculator {
                    public int add(int a, int b) {
                        int result = <cur:a_ref>a + b;
                        return result;
                    }
                }
            "#,
            )
            .assert_at("a_ref")
            .name("a")
            .kind(SymbolKind::Parameter)
            .parent_fqn("com.example.Calculator.add")
            .expected_failure("parameter references in method body not yet resolved")
            .run();
    }
}

// §6.4 — Shadowing and Obscuring
mod jls_6_4_shadowing {
    use super::*;

    /// Parameter `name` shadows field `name`; this.name resolves to the field.
    /// Parameter kind is not yet supported, but this-qualified access works.
    // @keep — `this_field` cursor on `this.name` usage resolves_to Person.name field (this-qualified access)
    #[test]
    fn parameter_shadows_field_with_this_access() {
        fixture()
            .file(
                "com/example/Person.java",
                r#"
                package com.example;
                public class Person {
                    private String name;
                    public void setName(String <cur:param>name) {
                        this.<cur:this_field>name = name;
                    }
                }
            "#,
            )
            .assert_at("param")
            .kind(SymbolKind::Parameter)
            .name("name")
            .expected_failure("parameters not yet indexed in symbol table")
            .assert_at("this_field")
            .resolves_to("com.example.Person.name")
            .kind(SymbolKind::Field)
            .run();
    }

    /// `this.` completion inside a method where a parameter shadows a field:
    /// the field should appear because `this.` bypasses the parameter scope.
    #[test]
    fn this_dot_shows_field_when_parameter_shadows() {
        fixture()
            .file(
                "com/example/Person.java",
                r#"
                package com.example;
                public class Person {
                    private String name;
                    public void setName(String name) {
                        this.<cur:this_dot>
                    }
                }
            "#,
            )
            .complete("this_dot", |items| {
                assert!(items.has("name", SymbolKind::Field));
            })
            .expected_failure("this-dot member completion not yet implemented")
            .run();
    }

    /// Unqualified completion inside a method where a parameter shadows a field:
    /// the parameter should win, so `name` appears as Parameter, not Field.
    #[test]
    fn parameter_shadows_field_in_unqualified_completion() {
        fixture()
            .file(
                "com/example/Person.java",
                r#"
                package com.example;
                public class Person {
                    private String name;
                    public void setName(String name) {
                        <cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("name", SymbolKind::Parameter));
                assert!(!items.has("name", SymbolKind::Field));
            })
            .expected_failure("parameter scope in unqualified completion not yet implemented")
            .run();
    }

    /// `this.` after a local variable of the same name as a field:
    /// `this.` should surface the field, not the local.
    #[test]
    fn this_dot_shows_field_when_local_shadows() {
        fixture()
            .file(
                "com/example/Counter.java",
                r#"
                package com.example;
                public class Counter {
                    private int count = 0;
                    public void reset() {
                        int count = 10;
                        this.<cur:this_dot>
                    }
                }
            "#,
            )
            .complete("this_dot", |items| {
                assert!(items.has("count", SymbolKind::Field));
            })
            .expected_failure("this-dot member completion not yet implemented")
            .run();
    }

    /// `this.` inside an inner class should show Inner's own members and
    /// NOT the outer class's members (`Outer.this.` is required for those).
    #[test]
    fn inner_class_this_dot_shows_own_members() {
        fixture()
            .file(
                "com/example/Outer.java",
                r#"
                package com.example;
                public class Outer {
                    private int value = 1;
                    private int outerOnly = 5;
                    class Inner {
                        private int value = 2;
                        public void test() {
                            this.<cur:inner_this>
                        }
                    }
                }
            "#,
            )
            .complete("inner_this", |items| {
                assert!(items.has("value", SymbolKind::Field));
                assert!(items.has("test", SymbolKind::Method));
                assert!(!items.has("outerOnly", SymbolKind::Field));
            })
            .expected_failure("inner class this-dot completion not yet implemented")
            .run();
    }

    /// A same-package type named `Vector` shadows `java.util.Vector` from a wildcard import.
    /// Dot-completion on `v` should show `com.example.Vector` members, not java.util.Vector.
    #[test]
    fn local_class_shadows_imported_wildcard_type() {
        fixture()
            .file(
                "com/example/Vector.java",
                r#"
                package com.example;
                public class Vector {
                    public int x;
                    public int y;
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                import java.util.*;
                public class App {
                    public void test(Vector v) {
                        v.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("x", SymbolKind::Field));
                assert!(items.has("y", SymbolKind::Field));
                assert!(!items.has("add", SymbolKind::Method));
                assert!(!items.has("size", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    /// A single-static-import shadows a static-on-demand import for the same name.
    /// `MAX` from the single-static-import (Limits.MAX) should appear; `MIN` (not imported
    /// individually) should not appear.
    #[test]
    fn single_static_import_shadows_on_demand_import() {
        fixture()
            .file(
                "com/example/Defaults.java",
                r#"
                package com.example;
                public class Defaults {
                    public static final int MAX = 50;
                }
            "#,
            )
            .file(
                "com/example/Limits.java",
                r#"
                package com.example;
                public class Limits {
                    public static final int MAX = 100;
                    public static final int MIN = 0;
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                import static com.example.Limits.MAX;
                import static com.example.Defaults.*;
                public class App {
                    public void test() {
                        int val = <cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("MAX", SymbolKind::Field));
                assert!(!items.has("MIN", SymbolKind::Field));
            })
            .expected_failure("static import completion not yet implemented")
            .run();
    }

    /// A local variable whose name matches a type obscures that type in expression context
    /// (§6.4.2). Dot-completion on the variable should show members of the variable's
    /// declared type (Object), not the class's static members.
    #[test]
    fn variable_obscures_type_name_in_completion() {
        fixture()
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void test() {
                        Object String = new Object();
                        String.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                // Object's instance methods should appear
                assert!(items.has("hashCode", SymbolKind::Method));
                assert!(items.has("toString", SymbolKind::Method));
                // java.lang.String static methods should NOT appear
                assert!(!items.has("valueOf", SymbolKind::Method));
            })
            .expected_failure("variable-obscures-type completion not yet implemented")
            .run();
    }

    /// Inner class field shadows outer class field — unqualified reference
    /// in inner class should resolve to the inner field.
    // @keep — cursor on `value` access inside Inner class body should resolve to inner field (scope resolution)
    #[test]
    fn inner_class_field_shadows_outer() {
        fixture()
            .file(
                "com/example/Outer.java",
                r#"
                package com.example;
                public class Outer {
                    private int value = 1;
                    class Inner {
                        private int value = 2;
                        public int getValue() {
                            return <cur:inner_val>value;
                        }
                    }
                }
            "#,
            )
            .assert_at("inner_val")
            .resolves_to("com.example.Outer.Inner.value")
            .kind(SymbolKind::Field)
            .expected_failure(
                "unqualified field ref in inner class not yet scope-resolved to inner",
            )
            .run();
    }
}

// §6.5 — Determining the Meaning of a Name
mod jls_6_5_meaning_of_names {
    use super::*;

    /// Name used as return type resolves to the class.
    // @keep — cross-file; cursor on return type Invoice resolves_to com.example.model.Invoice (usage site)
    #[test]
    fn name_in_return_type_context() {
        fixture()
            .file(
                "com/example/model/Invoice.java",
                r#"
                package com.example.model;
                public class Invoice {
                    private double total;
                }
            "#,
            )
            .file(
                "com/example/service/Billing.java",
                r#"
                package com.example.service;
                import com.example.model.Invoice;
                public class Billing {
                    public <cur:ret_type>Invoice createInvoice() {
                        return new Invoice();
                    }
                }
            "#,
            )
            .assert_at("ret_type")
            .resolves_to("com.example.model.Invoice")
            .kind(SymbolKind::Class)
            .run();
    }

    /// Name in expression context referencing a parameter — not yet resolved.
    // @keep — cursor on parameter reference `name` in expression body (usage site, not declaration)
    #[test]
    fn name_in_expression_context_as_parameter() {
        fixture()
            .file(
                "com/example/Greeter.java",
                r#"
                package com.example;
                public class Greeter {
                    public String greet(String name) {
                        String greeting = "Hello, " + <cur:name_expr>name;
                        return greeting;
                    }
                }
            "#,
            )
            .assert_at("name_expr")
            .name("name")
            .kind(SymbolKind::Parameter)
            .expected_failure("parameter references in expression context not yet resolved")
            .run();
    }

    /// Unqualified method invocation resolves to the method in the same class.
    // @keep — cursor on unqualified `format(...)` call resolves_to Formatter.format (same-class invocation)
    #[test]
    fn unqualified_method_invocation() {
        fixture()
            .file(
                "com/example/Formatter.java",
                r#"
                package com.example;
                public class Formatter {
                    public String format(String template) {
                        return template.trim();
                    }
                    public void run() {
                        String result = <cur:invoke>format("  hello  ");
                    }
                }
            "#,
            )
            .assert_at("invoke")
            .resolves_to("com.example.Formatter.format")
            .kind(SymbolKind::Method)
            .run();
    }

    /// Name in a cast expression resolves as a type.
    // @keep — cross-file; cursor on cast type (Dog) resolves_to com.example.Dog (usage in cast expression)
    #[test]
    fn name_in_cast_context() {
        fixture()
            .file(
                "com/example/Animal.java",
                r#"
                package com.example;
                public class Animal {}
            "#,
            )
            .file(
                "com/example/Dog.java",
                r#"
                package com.example;
                public class Dog extends Animal {
                    public void bark() {}
                }
            "#,
            )
            .file(
                "com/example/Kennel.java",
                r#"
                package com.example;
                public class Kennel {
                    public void handle(Animal a) {
                        (<cur:cast_type>Dog) a;
                    }
                }
            "#,
            )
            .assert_at("cast_type")
            .resolves_to("com.example.Dog")
            .kind(SymbolKind::Class)
            .run();
    }

    /// Name in instanceof expression resolves as a type.
    // @keep — cross-file; cursor on instanceof type Circle resolves_to com.example.Circle (usage site)
    #[test]
    fn name_in_instanceof_context() {
        fixture()
            .file(
                "com/example/Shape.java",
                r#"
                package com.example;
                public class Shape {}
            "#,
            )
            .file(
                "com/example/Circle.java",
                r#"
                package com.example;
                public class Circle extends Shape {
                    public double radius;
                }
            "#,
            )
            .file(
                "com/example/Renderer.java",
                r#"
                package com.example;
                public class Renderer {
                    public void draw(Shape s) {
                        if (s instanceof <cur:instanceof_type>Circle) {
                            System.out.println("circle");
                        }
                    }
                }
            "#,
            )
            .assert_at("instanceof_type")
            .resolves_to("com.example.Circle")
            .kind(SymbolKind::Class)
            .run();
    }

    /// Dot-completion after a class name should show static members only.
    #[test]
    fn static_member_completion_via_class_name() {
        fixture()
            .file(
                "com/example/MathUtils.java",
                r#"
                package com.example;
                public class MathUtils {
                    public static int MAX = 100;
                    public static int clamp(int v) { return v; }
                    public int instanceField;
                    public void instanceMethod() {}
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run() {
                        MathUtils.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("MAX", SymbolKind::Field));
                assert!(items.has("clamp", SymbolKind::Method));
                assert!(!items.has("instanceField", SymbolKind::Field));
                assert!(!items.has("instanceMethod", SymbolKind::Method));
            })
            .expected_failure("static member completion not yet implemented")
            .run();
    }

    /// Unqualified completion inside a method body should include the class's own methods.
    #[test]
    fn unqualified_method_completion_in_method_body() {
        fixture()
            .file(
                "com/example/Calculator.java",
                r#"
                package com.example;
                public class Calculator {
                    public int add(int a, int b) { return a + b; }
                    public int multiply(int a, int b) { return a * b; }
                    public void compute() {
                        <cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("add", SymbolKind::Method));
                assert!(items.has("multiply", SymbolKind::Method));
                assert!(items.has("compute", SymbolKind::Method));
            })
            .expected_failure("unqualified method completion not yet implemented")
            .run();
    }

    /// Dot-completion after `OuterClass.` in a type context should show public nested
    /// classes only — not private nested classes and not instance fields.
    #[test]
    fn nested_class_visible_via_outer_dot_completion() {
        fixture()
            .file(
                "com/example/Container.java",
                r#"
                package com.example;
                public class Container {
                    public static class Entry { public String key; }
                    private static class InternalEntry { public String data; }
                    public int size;
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    private Container.<cur> field;
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("Entry", SymbolKind::Class));
                assert!(!items.has("InternalEntry", SymbolKind::Class));
                assert!(!items.has("size", SymbolKind::Field));
            })
            .expected_failure("nested class completion via qualified name not yet implemented")
            .run();
    }

    /// Methods inherited from a superclass should appear in `this.` completions
    /// within the subclass, alongside the subclass's own methods.
    #[test]
    fn inherited_methods_visible_in_this_dot_completion() {
        fixture()
            .file(
                "com/example/Animal.java",
                r#"
                package com.example;
                public class Animal {
                    public void eat() {}
                    public String name() { return "animal"; }
                }
            "#,
            )
            .file(
                "com/example/Dog.java",
                r#"
                package com.example;
                public class Dog extends Animal {
                    public void bark() {}
                    public void tricks() {
                        this.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("bark", SymbolKind::Method));
                assert!(items.has("tricks", SymbolKind::Method));
                assert!(items.has("eat", SymbolKind::Method));
                assert!(items.has("name", SymbolKind::Method));
            })
            .expected_failure("inherited method completion not yet implemented")
            .run();
    }

    /// Per §6.5.7.1 (the "comb rule"), method lookup in an anonymous class searches the
    /// anonymous class's own superclass hierarchy before the lexically enclosing class.
    /// `this.` inside the anonymous class body should show Base's `action`, not App's.
    #[test]
    fn anonymous_class_this_dot_prefers_superclass_over_enclosing() {
        fixture()
            .file(
                "com/example/Base.java",
                r#"
                package com.example;
                public class Base {
                    public void action(String s) {}
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    void action(int i) {}
                    void run() {
                        new Base() {
                            {
                                this.<cur>
                            }
                        };
                    }
                }
            "#,
            )
            .complete_default(|items| {
                // The anonymous class's supertype (Base) contributes `action(String)`
                assert!(items.has("action", SymbolKind::Method));
                // App.action(int) should NOT appear — enclosing class methods are shadowed
            })
            .expected_failure("anonymous class this-dot completion not yet implemented")
            .run();
    }

    /// Class name used for static member access resolves to the class.
    // @keep — cross-file; cursor on AppConfig in static access expression resolves_to the class
    #[test]
    fn class_name_in_static_access() {
        fixture()
            .file(
                "com/example/AppConfig.java",
                r#"
                package com.example;
                public class AppConfig {
                    public static final String VERSION = "1.0";
                }
            "#,
            )
            .file(
                "com/example/Main.java",
                r#"
                package com.example;
                public class Main {
                    public void printVersion() {
                        String v = <cur:cls_ref>AppConfig.VERSION;
                    }
                }
            "#,
            )
            .assert_at("cls_ref")
            .resolves_to("com.example.AppConfig")
            .kind(SymbolKind::Class)
            .run();
    }
}

// §6.6 — Access Control
mod jls_6_6_access_control {
    use super::*;

    /// Protected method declared in base class, called from subclass in different package.
    // @keep — cross-file cross-package; `prot_call` cursor on init() call in Derived resolves_to Base.init
    #[test]
    fn protected_method_resolved_in_subclass() {
        fixture()
            .file(
                "com/example/base/Base.java",
                r#"
                package com.example.base;
                public class Base {
                    protected void <cur:prot_method>init() {}
                }
            "#,
            )
            .file(
                "com/example/impl/Derived.java",
                r#"
                package com.example.impl;
                import com.example.base.Base;
                public class Derived extends Base {
                    public void setup() {
                        <cur:prot_call>init();
                    }
                }
            "#,
            )
            .assert_at("prot_method")
            .kind(SymbolKind::Method)
            .modifiers(vec![Modifier::Protected])
            .fqn("com.example.base.Base.init")
            .assert_at("prot_call")
            .resolves_to("com.example.base.Base.init")
            .run();
    }

    /// Private members must not appear in dot-completions from a different class.
    #[test]
    fn private_members_not_visible_from_another_class() {
        fixture()
            .file(
                "com/example/Secret.java",
                r#"
                package com.example;
                public class Secret {
                    public String publicData;
                    private String secretData;
                    public void publicMethod() {}
                    private void secretMethod() {}
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(Secret s) {
                        s.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("publicData", SymbolKind::Field));
                assert!(items.has("publicMethod", SymbolKind::Method));
                assert!(!items.has("secretData", SymbolKind::Field));
                assert!(!items.has("secretMethod", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    /// Package-private members are visible within the same package but hidden across packages.
    #[test]
    fn package_private_access_control_completion() {
        fixture()
            .file(
                "com/example/internal/Helper.java",
                r#"
                package com.example.internal;
                public class Helper {
                    public String publicField;
                    String packageField;
                    public void publicMethod() {}
                    void packageMethod() {}
                }
            "#,
            )
            .file(
                "com/example/internal/SamePackage.java",
                r#"
                package com.example.internal;
                public class SamePackage {
                    public void test(Helper h) {
                        h.<cur:same_pkg>
                    }
                }
            "#,
            )
            .file(
                "com/example/other/DiffPackage.java",
                r#"
                package com.example.other;
                import com.example.internal.Helper;
                public class DiffPackage {
                    public void test(Helper h) {
                        h.<cur:diff_pkg>
                    }
                }
            "#,
            )
            .complete("same_pkg", |items| {
                assert!(items.has("publicField", SymbolKind::Field));
                assert!(items.has("packageField", SymbolKind::Field));
                assert!(items.has("publicMethod", SymbolKind::Method));
                assert!(items.has("packageMethod", SymbolKind::Method));
            })
            .expected_failure("package-private access control in completion not yet implemented")
            .complete("diff_pkg", |items| {
                assert!(items.has("publicField", SymbolKind::Field));
                assert!(items.has("publicMethod", SymbolKind::Method));
                assert!(!items.has("packageField", SymbolKind::Field));
                assert!(!items.has("packageMethod", SymbolKind::Method));
            })
            .expected_failure("package-private access control in completion not yet implemented")
            .run();
    }

    /// Protected members appear in subclass `this.` completions but not from an unrelated class.
    #[test]
    fn protected_members_access_control_completion() {
        fixture()
            .file(
                "com/example/base/Base.java",
                r#"
                package com.example.base;
                public class Base {
                    public void publicMethod() {}
                    protected void protectedMethod() {}
                    private void privateMethod() {}
                }
            "#,
            )
            .file(
                "com/example/sub/Sub.java",
                r#"
                package com.example.sub;
                import com.example.base.Base;
                public class Sub extends Base {
                    public void test() {
                        this.<cur:sub_this>
                    }
                }
            "#,
            )
            .file(
                "com/example/other/Unrelated.java",
                r#"
                package com.example.other;
                import com.example.base.Base;
                public class Unrelated {
                    public void test(Base b) {
                        b.<cur:unrelated>
                    }
                }
            "#,
            )
            .complete("sub_this", |items| {
                assert!(items.has("publicMethod", SymbolKind::Method));
                assert!(items.has("protectedMethod", SymbolKind::Method));
                assert!(!items.has("privateMethod", SymbolKind::Method));
            })
            .expected_failure("protected access control in completion not yet implemented")
            .complete("unrelated", |items| {
                assert!(items.has("publicMethod", SymbolKind::Method));
                assert!(!items.has("protectedMethod", SymbolKind::Method));
                assert!(!items.has("privateMethod", SymbolKind::Method));
            })
            .expected_failure("protected access control in completion not yet implemented")
            .run();
    }

    /// Per §6.6.1, private members are accessible within the entire body of the top-level
    /// class that declares them — including nested classes. A nested class accessing an
    /// instance of the outer class should see its private members in dot-completions.
    #[test]
    fn private_members_of_enclosing_class_visible_in_nested() {
        fixture()
            .file(
                "com/example/Outer.java",
                r#"
                package com.example;
                public class Outer {
                    private int secretField;
                    private void secretMethod() {}
                    public int publicField;
                    class Inner {
                        public void test(Outer o) {
                            o.<cur>
                        }
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("secretField", SymbolKind::Field));
                assert!(items.has("secretMethod", SymbolKind::Method));
                assert!(items.has("publicField", SymbolKind::Field));
            })
            .expected_failure("private member visibility in nested class not yet implemented")
            .run();
    }

    /// Per §6.6.2.1, protected instance members are accessible in a subclass only when the
    /// qualifying type is the subclass (or below). `this.` can access protected members;
    /// an arbitrary `Base` reference from a different package cannot.
    #[test]
    fn protected_access_this_vs_arbitrary_base_ref() {
        fixture()
            .file(
                "com/example/base/Base.java",
                r#"
                package com.example.base;
                public class Base {
                    protected void init() {}
                    public void start() {}
                }
            "#,
            )
            .file(
                "com/example/sub/Sub.java",
                r#"
                package com.example.sub;
                import com.example.base.Base;
                public class Sub extends Base {
                    public void test(Base other) {
                        this.<cur:self>
                        other.<cur:other_ref>
                    }
                }
            "#,
            )
            .complete("self", |items| {
                // `this` has type Sub, so protected access is permitted
                assert!(items.has("init", SymbolKind::Method));
                assert!(items.has("start", SymbolKind::Method));
            })
            .expected_failure("this-dot completion in subclass not yet implemented")
            .complete("other_ref", |items| {
                // `other` is typed as Base from a different package — protected not accessible
                assert!(items.has("start", SymbolKind::Method));
                assert!(!items.has("init", SymbolKind::Method));
            })
            .expected_failure(
                "protected access filtering on arbitrary base ref not yet implemented",
            )
            .run();
    }

    /// Package-private class and method, accessed from same package.
    // @keep — cross-file same-package; `pkg_call` cursor on Helper.format() call resolves_to the method
    #[test]
    fn package_private_method_resolved_in_same_package() {
        fixture()
            .file(
                "com/example/internal/Helper.java",
                r#"
                package com.example.internal;
                class Helper {
                    static String <cur:pkg_method>format(String s) {
                        return s.trim();
                    }
                }
            "#,
            )
            .file(
                "com/example/internal/Service.java",
                r#"
                package com.example.internal;
                public class Service {
                    public String process(String input) {
                        return Helper.<cur:pkg_call>format(input);
                    }
                }
            "#,
            )
            .assert_at("pkg_method")
            .kind(SymbolKind::Method)
            .modifiers(vec![Modifier::Static])
            .fqn("com.example.internal.Helper.format")
            .assert_at("pkg_call")
            .resolves_to("com.example.internal.Helper.format")
            .run();
    }
}

// §6.7 — Fully Qualified Names and Canonical Names
mod jls_6_7_fqn {
    use super::*;

    /// Builder pattern: nested class and its methods have correct FQNs and children.
    /// Also exercises §8.5 (member type declarations).
    // @evolve — add dot-completion test: in a consumer file, cursor after `new HttpRequest.Builder().`, expect withUrl, withMethod, withBody, build
    #[test]
    fn builder_pattern_nested_class() {
        fixture()
            .file(
                "com/example/model/HttpRequest.java",
                r#"
                package com.example.model;
                public class HttpRequest {
                    private final String url;
                    private final String method;

                    private HttpRequest(Builder builder) {
                        this.url = builder.url;
                        this.method = builder.method;
                    }

                    public static class <cur:builder>Builder {
                        private String url;
                        private String method;
                        public Builder <cur:with_url>withUrl(String url) {
                            this.url = url;
                            return this;
                        }
                        public HttpRequest build() {
                            return new HttpRequest(this);
                        }
                    }
                }
            "#,
            )
            .assert_at("builder")
            .kind(SymbolKind::Class)
            .fqn("com.example.model.HttpRequest.Builder")
            .parent_fqn("com.example.model.HttpRequest")
            .children_include(&["url", "method", "withUrl", "build"])
            .children_count(4)
            .assert_at("with_url")
            .kind(SymbolKind::Method)
            .fqn("com.example.model.HttpRequest.Builder.withUrl")
            .parent_fqn("com.example.model.HttpRequest.Builder")
            .signature_return("Builder")
            .signature_params(&[("url", "String")])
            .run();
    }

    #[test]
    fn dot_completion_on_builder() {
        fixture()
            .file("com/example/model/HttpRequest.java", r#"
                package com.example.model;
                public class HttpRequest {
                    private final String url;
                    private final String method;
                    private final String body;

                    private HttpRequest(Builder builder) {
                        this.url = builder.url;
                        this.method = builder.method;
                        this.body = builder.body;
                    }

                    public static class Builder {
                        private String url;
                        private String method;
                        private String body;
                        public Builder withUrl(String url) { this.url = url; return this; }
                        public Builder withMethod(String method) { this.method = method; return this; }
                        public Builder withBody(String body) { this.body = body; return this; }
                        public HttpRequest build() { return new HttpRequest(this); }
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import com.example.model.HttpRequest;
                public class App {
                    public void send() {
                        new HttpRequest.Builder().<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("withUrl", SymbolKind::Method));
                assert!(items.has("withMethod", SymbolKind::Method));
                assert!(items.has("withBody", SymbolKind::Method));
                assert!(items.has("build", SymbolKind::Method));
                assert!(!items.has("url", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

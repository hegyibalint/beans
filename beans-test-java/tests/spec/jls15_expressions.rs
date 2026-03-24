use beans_core::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §15.9 — Class Instance Creation Expressions
mod jls_15_9_class_instance_creation {
    use super::*;

    // @keep — cross-file; new-expression cursor resolves_to User (expected_failure for Constructor kind vs Class)
    #[test]
    fn new_simple_class() {
        fixture()
            .file("com/example/User.java", r#"
                package com.example;
                public class <cur:user_cls>User {
                    private String name;
                    public User(String name) { this.name = name; }
                    public String getName() { return name; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        User u = new <cur:new_user>User("Alice");
                    }
                }
            "#)
            .assert_at("user_cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.User")
            .assert_at("new_user")
                .resolves_to("com.example.User")
                .kind(SymbolKind::Constructor)
                .expected_failure("new expression should resolve to constructor, not class")
            .run();
    }

    // @keep — constructor kind not yet distinguished from class in Config; expected_failure documents the gap
    #[test]
    fn constructor_declaration() {
        fixture()
            .file("com/example/Config.java", r#"
                package com.example;
                public class Config {
                    private final String env;
                    public <cur:ctor>Config(String env) { this.env = env; }
                    public String getEnv() { return env; }
                }
            "#)
            .assert_at("ctor")
                .kind(SymbolKind::Constructor)
                .fqn("com.example.Config.Config")
                .signature_params(&[("env", "String")])
                .expected_failure("constructor declarations not yet distinguished from class")
            .run();
    }

    // @keep — cross-file: qualified inner class type (Outer.Inner) not yet resolved; expected_failure
    #[test]
    fn new_qualified_inner_class() {
        fixture()
            .file("com/example/Outer.java", r#"
                package com.example;
                public class <cur:outer_cls>Outer {
                    public class Inner {
                        private int value;
                        public Inner(int value) { this.value = value; }
                    }
                }
            "#)
            .file("com/example/Client.java", r#"
                package com.example;
                public class Client {
                    public void test() {
                        Outer outer = new Outer();
                        Outer.<cur:inner_ref>Inner inner = outer.new Inner(42);
                    }
                }
            "#)
            .assert_at("outer_cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Outer")
            .assert_at("inner_ref")
                .resolves_to("com.example.Outer.Inner")
                .kind(SymbolKind::Class)
                .expected_failure("qualified inner class type resolution not yet implemented")
            .run();
    }

    // @keep — cross-file: anonymous class creation at new EventHandler() resolves to the interface
    #[test]
    fn anonymous_class_creation() {
        fixture()
            .file("com/example/EventHandler.java", r#"
                package com.example;
                public interface <cur:handler_iface>EventHandler {
                    void handle(String event);
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void setup() {
                        EventHandler h = new <cur:anon>EventHandler() {
                            @Override
                            public void handle(String event) {
                                System.out.println(event);
                            }
                        };
                    }
                }
            "#)
            .assert_at("handler_iface")
                .kind(SymbolKind::Interface)
                .fqn("com.example.EventHandler")
            .assert_at("anon")
                .resolves_to("com.example.EventHandler")
            .run();
    }

    // @keep — cross-file: diamond inference in `new Box<>()` resolves to Box class
    #[test]
    fn diamond_inference() {
        fixture()
            .file("com/example/Box.java", r#"
                package com.example;
                public class <cur:box_cls>Box<T> {
                    private T value;
                    public Box(T value) { this.value = value; }
                    public T getValue() { return value; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void test() {
                        Box<String> box = new <cur:diamond>Box<>("hello");
                    }
                }
            "#)
            .assert_at("box_cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Box")
            .assert_at("diamond")
                .resolves_to("com.example.Box")
            .run();
    }
}

// §15.11 — Field Access Expressions
mod jls_15_11_field_access {
    use super::*;

    #[test]
    fn dot_completion_static_members() {
        fixture()
            .file("com/example/MathUtils.java", r#"
                package com.example;
                public class MathUtils {
                    public static int MAX = 100;
                    public static double sqrt(double x) { return Math.sqrt(x); }
                    private static int seed = 42;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        MathUtils.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("MAX", SymbolKind::Field));
                assert!(items.has("sqrt", SymbolKind::Method));
                assert!(!items.has("seed", SymbolKind::Field));
            })
            .expected_failure("static member completion not yet implemented")
            .run();
    }

    // @keep — cross-file: qualified field access `a.x` resolves to Point.x field declaration
    #[test]
    fn qualified_field_access() {
        fixture()
            .file("com/example/Point.java", r#"
                package com.example;
                public class Point {
                    public final int <cur:x_decl>x;
                    public final int y;
                    public Point(int x, int y) { this.x = x; this.y = y; }
                }
            "#)
            .file("com/example/Geometry.java", r#"
                package com.example;
                public class Geometry {
                    public double distance(Point a, Point b) {
                        int dx = a.<cur:x_access>x - b.x;
                        return Math.sqrt(dx * dx);
                    }
                }
            "#)
            .assert_at("x_decl")
                .kind(SymbolKind::Field)
                .fqn("com.example.Point.x")
                .modifiers(vec![Modifier::Public, Modifier::Final])
            .assert_at("x_access")
                .resolves_to("com.example.Point.x")
                .kind(SymbolKind::Field)
            .run();
    }

    // @keep — cross-file: static field access `Constants.MAX_SIZE` resolves to field declaration
    #[test]
    fn static_field_access() {
        fixture()
            .file("com/example/Constants.java", r#"
                package com.example;
                public class Constants {
                    public static final int <cur:max_decl>MAX_SIZE = 100;
                    public static final String DEFAULT_NAME = "unknown";
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void test() {
                        int max = Constants.<cur:max_ref>MAX_SIZE;
                    }
                }
            "#)
            .assert_at("max_decl")
                .kind(SymbolKind::Field)
                .fqn("com.example.Constants.MAX_SIZE")
                .modifiers(vec![Modifier::Public, Modifier::Static, Modifier::Final])
            .assert_at("max_ref")
                .resolves_to("com.example.Constants.MAX_SIZE")
                .kind(SymbolKind::Field)
            .run();
    }

    // @keep — cross-file: `super.tag` access in subclass resolves to Base.tag field declaration
    #[test]
    fn super_field_access() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    protected String <cur:base_tag>tag = "base";
                }
            "#)
            .file("com/example/Derived.java", r#"
                package com.example;
                public class Derived extends Base {
                    private String label = "derived";
                    public String getBaseTag() {
                        return super.<cur:super_tag>tag;
                    }
                }
            "#)
            .assert_at("base_tag")
                .kind(SymbolKind::Field)
                .fqn("com.example.Base.tag")
                .modifiers(vec![Modifier::Protected])
            .assert_at("super_tag")
                .resolves_to("com.example.Base.tag")
            .run();
    }

    #[test]
    fn dot_completion_this() {
        fixture()
            .file("com/example/Widget.java", r#"
                package com.example;
                public class Widget {
                    public String name;
                    private int id;
                    public void render() {}
                    private void init() {}

                    public void setup() {
                        this.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("name", SymbolKind::Field));
                assert!(items.has("id", SymbolKind::Field));
                assert!(items.has("render", SymbolKind::Method));
                assert!(items.has("init", SymbolKind::Method));
            })
            .expected_failure("this member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_super() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public void baseMethod() {}
                    protected String tag = "base";
                }
            "#)
            .file("com/example/Child.java", r#"
                package com.example;
                public class Child extends Base {
                    private int extra;

                    public void doWork() {
                        super.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("baseMethod", SymbolKind::Method));
                assert!(items.has("tag", SymbolKind::Field));
                assert!(!items.has("extra", SymbolKind::Field));
            })
            .expected_failure("super member completion not yet implemented")
            .run();
    }

    // @keep — single file: `this.width` access resolves to Rectangle.width field declaration
    #[test]
    fn this_field_access() {
        fixture()
            .file("com/example/Rectangle.java", r#"
                package com.example;
                public class Rectangle {
                    private int <cur:width_decl>width;
                    private int height;

                    public void setWidth(int width) {
                        this.<cur:this_width>width = width;
                    }
                }
            "#)
            .assert_at("width_decl")
                .kind(SymbolKind::Field)
                .fqn("com.example.Rectangle.width")
                .modifiers(vec![Modifier::Private])
            .assert_at("this_width")
                .resolves_to("com.example.Rectangle.width")
            .run();
    }
}

// §15.12 — Method Invocation Expressions
mod jls_15_12_method_invocation {
    use super::*;

    // @keep — cross-file: static method call `StringUtils.trimAndLower()` resolves to declaration
    #[test]
    fn static_method_call() {
        fixture()
            .file("com/example/StringUtils.java", r#"
                package com.example;
                public class StringUtils {
                    public static String <cur:trim_decl>trimAndLower(String input) {
                        return input.trim().toLowerCase();
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void test() {
                        String result = StringUtils.<cur:trim_call>trimAndLower("  HELLO  ");
                    }
                }
            "#)
            .assert_at("trim_decl")
                .kind(SymbolKind::Method)
                .fqn("com.example.StringUtils.trimAndLower")
                .modifiers(vec![Modifier::Public, Modifier::Static])
                .signature_return("String")
                .signature_params(&[("input", "String")])
            .assert_at("trim_call")
                .resolves_to("com.example.StringUtils.trimAndLower")
                .kind(SymbolKind::Method)
            .run();
    }

    // @evolve — Account has getBalance() and deposit(double): add dot-completion test: cursor after `acct.` in Bank.audit(), expect `getBalance()` (double) and `deposit(double)` methods
    #[test]
    fn instance_method_call() {
        fixture()
            .file("com/example/Account.java", r#"
                package com.example;
                public class Account {
                    private double balance;
                    public double <cur:get_bal>getBalance() { return balance; }
                    public void deposit(double amount) { balance += amount; }
                }
            "#)
            .file("com/example/Bank.java", r#"
                package com.example;
                public class Bank {
                    public void audit(Account acct) {
                        double bal = acct.<cur:bal_call>getBalance();
                    }
                }
            "#)
            .assert_at("get_bal")
                .kind(SymbolKind::Method)
                .fqn("com.example.Account.getBalance")
                .signature_return("double")
            .assert_at("bal_call")
                .resolves_to("com.example.Account.getBalance")
            .run();
    }

    // @evolve — Logger has logMessage(String) and logError(String, Throwable): add dot-completion test: cursor after `logger.`, expect both overloads as separate entries
    #[test]
    fn overloaded_method_declarations() {
        fixture()
            .file("com/example/Logger.java", r#"
                package com.example;
                public class Logger {
                    public void <cur:log_msg>logMessage(String message) {}
                    public void <cur:log_err>logError(String message, Throwable cause) {}
                }
            "#)
            .assert_at("log_msg")
                .kind(SymbolKind::Method)
                .fqn("com.example.Logger.logMessage")
                .signature_params(&[("message", "String")])
            .assert_at("log_err")
                .kind(SymbolKind::Method)
                .fqn("com.example.Logger.logError")
                .signature_params(&[("message", "String"), ("cause", "Throwable")])
            .run();
    }

    #[test]
    fn dot_completion_on_account() {
        fixture()
            .file("com/example/Account.java", r#"
                package com.example;
                public class Account {
                    private double balance;
                    public double getBalance() { return balance; }
                    public void deposit(double amount) { balance += amount; }
                }
            "#)
            .file("com/example/Bank.java", r#"
                package com.example;
                public class Bank {
                    public void audit(Account acct) {
                        acct.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getBalance", SymbolKind::Method));
                assert!(items.has("deposit", SymbolKind::Method));
                assert!(!items.has("balance", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_logger_overloads() {
        fixture()
            .file("com/example/Logger.java", r#"
                package com.example;
                public class Logger {
                    public void logMessage(String message) {}
                    public void logError(String message, Throwable cause) {}
                    private String prefix;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Logger logger) {
                        logger.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("logMessage", SymbolKind::Method));
                assert!(items.has("logError", SymbolKind::Method));
                assert!(!items.has("prefix", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_new_expr() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    public void start() {}
                    public void stop() {}
                    private int port;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        new Service().<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("start", SymbolKind::Method));
                assert!(items.has("stop", SymbolKind::Method));
                assert!(!items.has("port", SymbolKind::Field));
            })
            .expected_failure("completion on new expression result not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_chained_method_call() {
        fixture()
            .file("com/example/MyBuilder.java", r#"
                package com.example;
                public class MyBuilder {
                    public MyBuilder append(String s) { return this; }
                    public String toString() { return ""; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(MyBuilder builder) {
                        builder.append("x").<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("append", SymbolKind::Method));
                assert!(items.has("toString", SymbolKind::Method));
            })
            .expected_failure("completion on chained method call return type not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_builder_pattern_chain() {
        fixture()
            .file("com/example/Request.java", r#"
                package com.example;
                public class Request {
                    public void execute() {}
                    public static class Builder {
                        public Builder withUrl(String url) { return this; }
                        public Builder withMethod(String method) { return this; }
                        public Request build() { return new Request(); }
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        new Request.Builder().withUrl("x").withMethod("POST").<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("withUrl", SymbolKind::Method));
                assert!(items.has("build", SymbolKind::Method));
                assert!(!items.has("execute", SymbolKind::Method));
            })
            .expected_failure("completion on chained builder pattern not yet implemented")
            .run();
    }

    // @keep — cross-file: overloaded print(Object) and print(String); most-specific overload resolution not yet implemented; expected_failure
    #[test]
    fn overloaded_same_name_methods() {
        fixture()
            .file("com/example/Printer.java", r#"
                package com.example;
                public class Printer {
                    public void <cur:print_obj>print(Object obj) {}
                    public void print(String str) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void test(Printer p) {
                        p.<cur:call_str>print("hello");
                    }
                }
            "#)
            .assert_at("print_obj")
                .kind(SymbolKind::Method)
                .fqn("com.example.Printer.print")
                .signature_params(&[("obj", "Object")])
                .expected_failure("overloaded method declarations may not resolve to expected overload")
            .assert_at("call_str")
                .resolves_to("com.example.Printer.print")
                .signature_params(&[("str", "String")])
                .expected_failure("most-specific overload resolution not yet implemented")
            .run();
    }

    // @keep — cross-file: chained builder calls (withUrl, buildRequest) resolve to correct declarations
    #[test]
    fn chained_method_calls_builder_pattern() {
        fixture()
            .file("com/example/Request.java", r#"
                package com.example;
                public class Request {
                    private String requestUrl;
                    private String requestMethod;
                    private String requestBody;

                    public static class Builder {
                        private String requestUrl;
                        private String requestMethod;
                        private String requestBody;

                        public Builder <cur:set_url>withUrl(String requestUrl) { this.requestUrl = requestUrl; return this; }
                        public Builder withMethod(String requestMethod) { this.requestMethod = requestMethod; return this; }
                        public Builder withBody(String requestBody) { this.requestBody = requestBody; return this; }
                        public Request <cur:build_decl>buildRequest() { return new Request(); }
                    }
                }
            "#)
            .file("com/example/Client.java", r#"
                package com.example;
                public class Client {
                    public void send() {
                        Request.Builder b = new Request.Builder();
                        Request req = b.<cur:url_call>withUrl("https://example.com")
                            .withMethod("POST")
                            .withBody("{}")
                            .<cur:build_call>buildRequest();
                    }
                }
            "#)
            .assert_at("set_url")
                .kind(SymbolKind::Method)
                .signature_return("Builder")
            .assert_at("build_decl")
                .kind(SymbolKind::Method)
                .signature_return("Request")
            .assert_at("url_call")
                .resolves_to("com.example.Request.Builder.withUrl")
            .assert_at("build_call")
                .resolves_to("com.example.Request.Builder.buildRequest")
            .run();
    }

    // @keep — cross-file (3 files): inherited makeSound() invocation on Animal variable; expected_failure for dynamic dispatch
    #[test]
    fn inherited_method_invocation() {
        fixture()
            .file("com/example/Animal.java", r#"
                package com.example;
                public class Animal {
                    public String <cur:make_sound>makeSound() { return "..."; }
                }
            "#)
            .file("com/example/Dog.java", r#"
                package com.example;
                public class Dog extends Animal {
                    @Override
                    public String makeSound() { return "Woof"; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void test() {
                        Animal a = new Dog();
                        String sound = a.<cur:sound_call>makeSound();
                    }
                }
            "#)
            .assert_at("make_sound")
                .kind(SymbolKind::Method)
                .fqn("com.example.Animal.makeSound")
                .expected_failure("overridden method declaration resolution ambiguous across files")
            .assert_at("sound_call")
                .resolves_to("com.example.Animal.makeSound")
                .expected_failure("method call on variable whose type has overridden methods")
            .run();
    }

    // @keep — single file: this.checkNotNull() resolves to private method in same class
    #[test]
    fn this_method_invocation() {
        fixture()
            .file("com/example/Validator.java", r#"
                package com.example;
                public class Validator {
                    private boolean <cur:check_decl>checkNotNull(Object obj) { return obj != null; }

                    public boolean validate(String input) {
                        return this.<cur:this_call>checkNotNull(input) && input.length() > 0;
                    }
                }
            "#)
            .assert_at("check_decl")
                .kind(SymbolKind::Method)
                .fqn("com.example.Validator.checkNotNull")
                .modifiers(vec![Modifier::Private])
            .assert_at("this_call")
                .resolves_to("com.example.Validator.checkNotNull")
            .run();
    }

    // @keep — cross-file: super.setupProcessor() call not yet resolved; expected_failure documents gap
    #[test]
    fn super_method_invocation() {
        fixture()
            .file("com/example/BaseProcessor.java", r#"
                package com.example;
                public class BaseProcessor {
                    public void <cur:setup_decl>setupProcessor() { }
                }
            "#)
            .file("com/example/ChildProcessor.java", r#"
                package com.example;
                public class ChildProcessor extends BaseProcessor {
                    @Override
                    public void setupProcessor() {
                        super.<cur:super_setup>setupProcessor();
                    }
                }
            "#)
            .assert_at("setup_decl")
                .kind(SymbolKind::Method)
                .fqn("com.example.BaseProcessor.setupProcessor")
                .expected_failure("overridden method declaration resolution ambiguous across files")
            .assert_at("super_setup")
                .resolves_to("com.example.BaseProcessor.setupProcessor")
                .expected_failure("super method invocation resolution not yet implemented")
            .run();
    }

    // @keep — varargs parameter type (Object...) not yet correctly represented; expected_failure
    #[test]
    fn varargs_method_declaration() {
        fixture()
            .file("com/example/Formatter.java", r#"
                package com.example;
                public class Formatter {
                    public String <cur:format_decl>formatMessage(String template, Object... args) {
                        return String.format(template, args);
                    }
                }
            "#)
            .assert_at("format_decl")
                .kind(SymbolKind::Method)
                .fqn("com.example.Formatter.formatMessage")
                .signature_return("String")
                .signature_params(&[("template", "String"), ("args", "Object...")])
                .expected_failure("varargs parameter type representation not yet implemented")
            .run();
    }
}

// §15.13 — Method Reference Expressions
mod jls_15_13_method_references {
    use super::*;

    // @keep — static method reference Parser::parseValue not yet resolved; expected_failure
    #[test]
    fn static_method_reference() {
        fixture()
            .file("com/example/Parser.java", r#"
                package com.example;
                import java.util.List;
                import java.util.stream.Collectors;
                public class Parser {
                    public static int <cur:parse_decl>parseValue(String s) {
                        return Integer.parseInt(s);
                    }

                    public List<Integer> parseAll(List<String> inputs) {
                        return inputs.stream()
                            .map(<cur:method_ref>Parser::parseValue)
                            .collect(Collectors.toList());
                    }
                }
            "#)
            .assert_at("parse_decl")
                .kind(SymbolKind::Method)
                .fqn("com.example.Parser.parseValue")
                .modifiers(vec![Modifier::Public, Modifier::Static])
            .assert_at("method_ref")
                .resolves_to("com.example.Parser.parseValue")
                .expected_failure("static method reference resolution not yet implemented")
            .run();
    }

    // @keep — instance method reference String::length not yet resolved; expected_failure
    #[test]
    fn instance_method_reference_on_type() {
        fixture()
            .file("com/example/Sorting.java", r#"
                package com.example;
                import java.util.List;
                import java.util.stream.Collectors;
                public class Sorting {
                    public List<String> sortByLength(List<String> items) {
                        return items.stream()
                            .sorted(java.util.Comparator.comparingInt(<cur:ref_length>String::length))
                            .collect(Collectors.toList());
                    }
                }
            "#)
            .assert_at("ref_length")
                .resolves_to("java.lang.String.length")
                .expected_failure("instance method reference on type not yet implemented")
            .run();
    }

    // @keep — constructor reference ArrayList::new not yet resolved; expected_failure
    #[test]
    fn constructor_reference() {
        fixture()
            .file("com/example/Factory.java", r#"
                package com.example;
                import java.util.List;
                import java.util.stream.Collectors;
                import java.util.ArrayList;
                public class Factory {
                    public List<String> toList(java.util.stream.Stream<String> stream) {
                        return stream.collect(Collectors.toCollection(<cur:ctor_ref>ArrayList::new));
                    }
                }
            "#)
            .assert_at("ctor_ref")
                .resolves_to("java.util.ArrayList.ArrayList")
                .expected_failure("constructor reference resolution not yet implemented")
            .run();
    }

    // @keep — this method reference this::onEvent not yet resolved; expected_failure
    #[test]
    fn this_method_reference() {
        fixture()
            .file("com/example/EventBus.java", r#"
                package com.example;
                import java.util.function.Consumer;
                public class EventBus {
                    private void <cur:on_event_decl>onEvent(String event) {
                        System.out.println("Received: " + event);
                    }

                    public Consumer<String> getHandler() {
                        return <cur:this_ref>this::onEvent;
                    }
                }
            "#)
            .assert_at("on_event_decl")
                .kind(SymbolKind::Method)
                .fqn("com.example.EventBus.onEvent")
            .assert_at("this_ref")
                .resolves_to("com.example.EventBus.onEvent")
                .expected_failure("this method reference resolution not yet implemented")
            .run();
    }

    // @keep — cross-file: super method reference super::describeWidget not yet resolved; expected_failure
    #[test]
    fn super_method_reference() {
        fixture()
            .file("com/example/Widget.java", r#"
                package com.example;
                import java.util.function.Supplier;
                public class Widget {
                    public String <cur:describe_widget>describeWidget() { return "Widget"; }
                }
            "#)
            .file("com/example/Button.java", r#"
                package com.example;
                import java.util.function.Supplier;
                public class Button extends Widget {
                    @Override
                    public String describeWidget() { return "Button"; }

                    public Supplier<String> parentDescription() {
                        return <cur:super_ref>super::describeWidget;
                    }
                }
            "#)
            .assert_at("describe_widget")
                .kind(SymbolKind::Method)
                .fqn("com.example.Widget.describeWidget")
                .expected_failure("overridden method declaration resolution ambiguous across files")
            .assert_at("super_ref")
                .resolves_to("com.example.Widget.describeWidget")
                .expected_failure("super method reference resolution not yet implemented")
            .run();
    }
}

// §15.27 — Lambda Expressions
mod jls_15_27_lambdas {
    use super::*;

    // @keep — lambda parameter with explicit type (String s) not yet indexed; expected_failure
    #[test]
    fn lambda_explicit_parameter_types() {
        fixture()
            .file("com/example/Filtering.java", r#"
                package com.example;
                import java.util.List;
                import java.util.function.Predicate;
                public class Filtering {
                    public Predicate<String> nonEmpty() {
                        return (String <cur:param_s>s) -> !s.isEmpty();
                    }
                }
            "#)
            .assert_at("param_s")
                .kind(SymbolKind::Parameter)
                .name("s")
                .expected_failure("lambda parameter symbol extraction not yet implemented")
            .run();
    }

    // @keep — inferred lambda parameters (a, b) not yet resolved; expected_failure
    #[test]
    fn lambda_inferred_parameter_types() {
        fixture()
            .file("com/example/Sorting.java", r#"
                package com.example;
                import java.util.Comparator;
                public class Sorting {
                    public Comparator<String> byLength() {
                        return (<cur:a>a, <cur:b>b) -> a.length() - b.length();
                    }
                }
            "#)
            .assert_at("a")
                .kind(SymbolKind::Parameter)
                .name("a")
                .expected_failure("inferred lambda parameter resolution not yet implemented")
            .assert_at("b")
                .kind(SymbolKind::Parameter)
                .name("b")
                .expected_failure("inferred lambda parameter resolution not yet implemented")
            .run();
    }

    // @keep — cross-file: lambda in Dispatcher targeting EventListener; interface and SAM method verified
    #[test]
    fn lambda_targeting_functional_interface() {
        fixture()
            .file("com/example/EventSystem.java", r#"
                package com.example;
                @FunctionalInterface
                public interface <cur:listener_iface>EventListener {
                    void <cur:on_event>onEvent(String eventType, Object payload);
                }
            "#)
            .file("com/example/Dispatcher.java", r#"
                package com.example;
                public class Dispatcher {
                    private EventListener listener;

                    public void register() {
                        this.listener = (<cur:type_param>eventType, payload) -> {
                            System.out.println(eventType);
                        };
                    }
                }
            "#)
            .assert_at("listener_iface")
                .kind(SymbolKind::Interface)
                .fqn("com.example.EventListener")
            .assert_at("on_event")
                .kind(SymbolKind::Method)
                .fqn("com.example.EventListener.onEvent")
                .signature_params(&[("eventType", "String"), ("payload", "Object")])
            .assert_at("type_param")
                .kind(SymbolKind::Parameter)
                .name("eventType")
                .expected_failure("lambda parameter in functional interface context not yet implemented")
            .run();
    }

    // @keep — single file: lambda captures and resolves enclosing Counter.prefix field
    #[test]
    fn lambda_accessing_enclosing_field() {
        fixture()
            .file("com/example/Counter.java", r#"
                package com.example;
                import java.util.List;
                public class Counter {
                    private final String <cur:prefix_decl>prefix;

                    public Counter(String prefix) { this.prefix = prefix; }

                    public void printAll(List<String> items) {
                        items.forEach(item -> System.out.println(<cur:prefix_ref>prefix + ": " + item));
                    }
                }
            "#)
            .assert_at("prefix_decl")
                .kind(SymbolKind::Field)
                .fqn("com.example.Counter.prefix")
                .modifiers(vec![Modifier::Private, Modifier::Final])
            .assert_at("prefix_ref")
                .resolves_to("com.example.Counter.prefix")
            .run();
    }

    // @evolve — UserService.User has getName() and getAge(): add dot-completion test: cursor after `u.` inside filter/map lambda, expect `getAge()` (int) and `getName()` (String)
    #[test]
    fn lambda_in_stream_pipeline() {
        fixture()
            .file("com/example/UserService.java", r#"
                package com.example;
                import java.util.List;
                import java.util.stream.Collectors;
                public class UserService {
                    public static class User {
                        private String <cur:name_field>name;
                        private int age;
                        public String <cur:get_name>getName() { return name; }
                        public int getAge() { return age; }
                    }

                    public List<String> getAdultNames(List<User> users) {
                        return users.stream()
                            .filter(u -> u.<cur:get_age_call>getAge() >= 18)
                            .map(u -> u.<cur:get_name_call>getName())
                            .collect(Collectors.toList());
                    }
                }
            "#)
            .assert_at("name_field")
                .kind(SymbolKind::Field)
                .fqn("com.example.UserService.User.name")
            .assert_at("get_name")
                .kind(SymbolKind::Method)
                .fqn("com.example.UserService.User.getName")
            .assert_at("get_age_call")
                .resolves_to("com.example.UserService.User.getAge")
            .assert_at("get_name_call")
                .resolves_to("com.example.UserService.User.getName")
            .run();
    }

    #[test]
    fn dot_completion_on_user_in_lambda() {
        fixture()
            .file("com/example/UserService.java", r#"
                package com.example;
                public class UserService {
                    public static class User {
                        private String name;
                        private int age;
                        public String getName() { return name; }
                        public int getAge() { return age; }
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import java.util.List;
                import java.util.stream.Collectors;
                public class App {
                    public List<String> process(List<UserService.User> users) {
                        return users.stream()
                            .filter(u -> u.<cur> >= 18)
                            .map(u -> u.getName())
                            .collect(Collectors.toList());
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getAge", SymbolKind::Method));
                assert!(items.has("getName", SymbolKind::Method));
                assert!(!items.has("name", SymbolKind::Field));
            })
            .expected_failure("member completion inside lambda not yet implemented")
            .run();
    }

    // @keep — method return type is Function<String, String>; verifies complex generic return type in signature
    #[test]
    fn lambda_multi_line_body() {
        fixture()
            .file("com/example/Transform.java", r#"
                package com.example;
                import java.util.function.Function;
                public class Transform {
                    public Function<String, String> <cur:transform_decl>createTransform() {
                        return (String input) -> {
                            String trimmed = input.trim();
                            String lower = trimmed.toLowerCase();
                            return lower;
                        };
                    }
                }
            "#)
            .assert_at("transform_decl")
                .kind(SymbolKind::Method)
                .fqn("com.example.Transform.createTransform")
                .signature_return("Function<String, String>")
            .run();
    }
}

// §15.28 — Switch Expressions
mod jls_15_28_switch_expressions {
    use super::*;

    // @keep — cross-file: Day enum constant SATURDAY resolves from switch case label
    #[test]
    fn switch_expression_arrow_labels() {
        fixture()
            .file("com/example/DayKind.java", r#"
                package com.example;
                public enum <cur:day_enum>Day {
                    MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY, SATURDAY, SUNDAY
                }
            "#)
            .file("com/example/Schedule.java", r#"
                package com.example;
                public class Schedule {
                    public String <cur:categorize>categorize(Day day) {
                        return switch (day) {
                            case MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY -> "weekday";
                            case <cur:saturday_ref>SATURDAY, SUNDAY -> "weekend";
                        };
                    }
                }
            "#)
            .assert_at("day_enum")
                .kind(SymbolKind::Enum)
                .fqn("com.example.Day")
            .assert_at("categorize")
                .kind(SymbolKind::Method)
                .signature_return("String")
            .assert_at("saturday_ref")
                .resolves_to("com.example.Day.SATURDAY")
            .run();
    }

    // @keep — single file: Grading.toLetterGrade indexed correctly despite yield in switch block
    #[test]
    fn switch_expression_with_yield() {
        fixture()
            .file("com/example/Grading.java", r#"
                package com.example;
                public class Grading {
                    public String <cur:to_letter>toLetterGrade(int score) {
                        return switch (score / 10) {
                            case 10, 9 -> "A";
                            case 8 -> "B";
                            case 7 -> "C";
                            default -> {
                                String result = score >= 60 ? "D" : "F";
                                yield result;
                            }
                        };
                    }
                }
            "#)
            .assert_at("to_letter")
                .kind(SymbolKind::Method)
                .fqn("com.example.Grading.toLetterGrade")
                .signature_return("String")
                .signature_params(&[("score", "int")])
            .run();
    }

    // @keep — cross-file: TrafficLight enum with 3 constants; RED resolves in switch case; children_count verified
    #[test]
    fn switch_expression_enum_exhaustiveness() {
        fixture()
            .file("com/example/TrafficLight.java", r#"
                package com.example;
                public enum <cur:light_enum>TrafficLight {
                    <cur:red>RED, <cur:yellow>YELLOW, <cur:green>GREEN
                }
            "#)
            .file("com/example/Driver.java", r#"
                package com.example;
                public class Driver {
                    public String <cur:action_decl>action(TrafficLight light) {
                        return switch (light) {
                            case <cur:red_ref>RED -> "stop";
                            case YELLOW -> "slow down";
                            case GREEN -> "go";
                        };
                    }
                }
            "#)
            .assert_at("light_enum")
                .kind(SymbolKind::Enum)
                .fqn("com.example.TrafficLight")
                .children_include(&["RED", "YELLOW", "GREEN"])
                .children_count(3)
            .assert_at("red")
                .kind(SymbolKind::Field)
                .fqn("com.example.TrafficLight.RED")
            .assert_at("action_decl")
                .kind(SymbolKind::Method)
                .signature_return("String")
                .signature_params(&[("light", "TrafficLight")])
            .assert_at("red_ref")
                .resolves_to("com.example.TrafficLight.RED")
            .run();
    }

    #[test]
    fn dot_completion_on_switch_result() {
        fixture()
            .file("com/example/Handler.java", r#"
                package com.example;
                public class Handler {
                    public void handle() {}
                    public String describe() { return "handler"; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(int x, Handler h1, Handler h2) {
                        (switch (x) { case 1 -> h1; default -> h2; }).<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("handle", SymbolKind::Method));
                assert!(items.has("describe", SymbolKind::Method));
            })
            .expected_failure("completion on switch expression result type not yet implemented")
            .run();
    }

    // @keep — cross-file (4 files): Shape sealed interface and Circle record resolve in switch pattern cases
    #[test]
    fn switch_expression_sealed_type_pattern() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public sealed interface <cur:shape_iface>Shape
                    permits Circle, Rectangle {}
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public record <cur:circle_rec>Circle(double radius) implements Shape {}
            "#)
            .file("com/example/Rectangle.java", r#"
                package com.example;
                public record Rectangle(double width, double height) implements Shape {}
            "#)
            .file("com/example/Areas.java", r#"
                package com.example;
                public class Areas {
                    public double <cur:area_decl>area(Shape shape) {
                        return switch (shape) {
                            case <cur:circle_case>Circle c -> Math.PI * c.radius() * c.radius();
                            case Rectangle r -> r.width() * r.height();
                        };
                    }
                }
            "#)
            .assert_at("shape_iface")
                .kind(SymbolKind::Interface)
                .fqn("com.example.Shape")
                .modifiers(vec![Modifier::Public, Modifier::Sealed])
            .assert_at("circle_rec")
                .kind(SymbolKind::Record)
                .fqn("com.example.Circle")
            .assert_at("area_decl")
                .kind(SymbolKind::Method)
                .signature_return("double")
            .assert_at("circle_case")
                .resolves_to("com.example.Circle")
            .run();
    }
}

// §15.10.3 — Array Access / §15.16 — Cast / §15.25 — Conditional / §15.8.5 — Parenthesized
mod jls_15_misc_completion {
    use super::*;

    #[test]
    fn dot_completion_on_array_element() {
        fixture()
            .file("com/example/Task.java", r#"
                package com.example;
                public class Task {
                    public void execute() {}
                    public String describe() { return "task"; }
                    private int priority;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Task[] tasks) {
                        tasks[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("execute", SymbolKind::Method));
                assert!(items.has("describe", SymbolKind::Method));
                assert!(!items.has("priority", SymbolKind::Field));
            })
            .expected_failure("completion on array element access not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_cast_expression() {
        fixture()
            .file("com/example/Animal.java", r#"
                package com.example;
                public interface Animal {
                    void eat();
                }
            "#)
            .file("com/example/Dog.java", r#"
                package com.example;
                public class Dog implements Animal {
                    public void eat() {}
                    public void fetch() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Animal animal) {
                        ((Dog) animal).<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("fetch", SymbolKind::Method));
                assert!(items.has("eat", SymbolKind::Method));
            })
            .expected_failure("completion on cast expression target type not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_ternary_result() {
        fixture()
            .file("com/example/Processor.java", r#"
                package com.example;
                public class Processor {
                    public void run() {}
                    public String status() { return "ok"; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void test(boolean flag, Processor procA, Processor procB) {
                        (flag ? procA : procB).<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("run", SymbolKind::Method));
                assert!(items.has("status", SymbolKind::Method));
            })
            .expected_failure("completion on ternary expression result type not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_parenthesized_expression() {
        fixture()
            .file("com/example/Calculator.java", r#"
                package com.example;
                public class Calculator {
                    public int add(int a, int b) { return a + b; }
                    public int multiply(int a, int b) { return a * b; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Calculator calc) {
                        (calc).<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("add", SymbolKind::Method));
                assert!(items.has("multiply", SymbolKind::Method));
            })
            .expected_failure("completion on parenthesized expression not yet implemented")
            .run();
    }
}

// §15.13 — Method Reference completion
mod jls_15_13_method_reference_completion {
    use super::*;

    #[test]
    fn dot_completion_method_reference_type() {
        fixture()
            .file("com/example/Converter.java", r#"
                package com.example;
                public class Converter {
                    public static String format(int x) { return String.valueOf(x); }
                    public static int parse(String s) { return Integer.parseInt(s); }
                    private static void init() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import java.util.function.Function;
                public class App {
                    Function<Integer, String> f = Converter::<cur>;
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("format", SymbolKind::Method));
                assert!(items.has("parse", SymbolKind::Method));
                assert!(!items.has("init", SymbolKind::Method));
            })
            .expected_failure("method reference completion not yet implemented")
            .run();
    }
}

// §15 — Enum constant member completion
mod jls_15_enum_constant_completion {
    use super::*;

    #[test]
    fn dot_completion_on_enum_constant() {
        fixture()
            .file("com/example/Color.java", r#"
                package com.example;
                public enum Color {
                    RED, GREEN, BLUE;
                    public String hex() { return ""; }
                    public int rgb() { return 0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Color.RED.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("hex", SymbolKind::Method));
                assert!(items.has("rgb", SymbolKind::Method));
                assert!(!items.has("RED", SymbolKind::Field));
            })
            .expected_failure("enum constant member completion not yet implemented")
            .run();
    }
}

// §15.12 — Deeper completion scenarios: generics, inheritance, interfaces, context positions
mod jls_15_12_deeper_completion {
    use super::*;

    #[test]
    fn dot_completion_generic_method_return_type() {
        fixture()
            .file("com/example/Container.java", r#"
                package com.example;
                public class Container<T> {
                    private T value;
                    public T get() { return value; }
                    public void set(T value) { this.value = value; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Container<String> container) {
                        container.get().<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("length", SymbolKind::Method));
                assert!(items.has("isEmpty", SymbolKind::Method));
                assert!(!items.has("get", SymbolKind::Method));
            })
            .expected_failure("generic return type resolution for completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_inherited_methods_on_subclass_variable() {
        fixture()
            .file("com/example/Vehicle.java", r#"
                package com.example;
                public class Vehicle {
                    public void start() {}
                    public int speed() { return 0; }
                }
            "#)
            .file("com/example/Car.java", r#"
                package com.example;
                public class Car extends Vehicle {
                    public void honk() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Car car) {
                        car.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("honk", SymbolKind::Method));
                assert!(items.has("start", SymbolKind::Method));
                assert!(items.has("speed", SymbolKind::Method));
            })
            .expected_failure("inherited member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_multiple_interface_methods() {
        fixture()
            .file("com/example/Readable.java", r#"
                package com.example;
                public interface Readable {
                    String read();
                    boolean hasNext();
                }
            "#)
            .file("com/example/Writable.java", r#"
                package com.example;
                public interface Writable {
                    void write(String data);
                }
            "#)
            .file("com/example/ReadWriteStream.java", r#"
                package com.example;
                public class ReadWriteStream implements Readable, Writable {
                    public String read() { return ""; }
                    public boolean hasNext() { return false; }
                    public void write(String data) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(ReadWriteStream stream) {
                        stream.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("read", SymbolKind::Method));
                assert!(items.has("hasNext", SymbolKind::Method));
                assert!(items.has("write", SymbolKind::Method));
            })
            .expected_failure("multi-interface inherited method completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_fluent_stream_pipeline() {
        fixture()
            .file("com/example/MyStream.java", r#"
                package com.example;
                import java.util.List;
                import java.util.function.Function;
                import java.util.function.Predicate;
                public class MyStream<T> {
                    public MyStream<T> filter(Predicate<T> p) { return this; }
                    public <R> MyStream<R> map(Function<T, R> f) { return null; }
                    public List<T> toList() { return null; }
                }
            "#)
            .file("com/example/Pipeline.java", r#"
                package com.example;
                public class Pipeline {
                    public MyStream<String> items() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Pipeline pipeline) {
                        pipeline.items().filter(x -> true).<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("filter", SymbolKind::Method));
                assert!(items.has("map", SymbolKind::Method));
                assert!(items.has("toList", SymbolKind::Method));
                assert!(!items.has("items", SymbolKind::Method));
            })
            .expected_failure("chained stream pipeline completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_method_call_result_as_receiver() {
        fixture()
            .file("com/example/Connection.java", r#"
                package com.example;
                public class Connection {
                    public String prepare(String sql) { return sql; }
                    public void close() {}
                }
            "#)
            .file("com/example/Database.java", r#"
                package com.example;
                public class Database {
                    public Connection connect() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Database db) {
                        db.connect().<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("prepare", SymbolKind::Method));
                assert!(items.has("close", SymbolKind::Method));
                assert!(!items.has("connect", SymbolKind::Method));
            })
            .expected_failure("completion on method call return type not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_inside_method_argument() {
        fixture()
            .file("com/example/Config.java", r#"
                package com.example;
                public class Config {
                    public String getTemplate() { return ""; }
                    public int getTimeout() { return 0; }
                }
            "#)
            .file("com/example/Formatter.java", r#"
                package com.example;
                public class Formatter {
                    public String format(String template) { return template; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Formatter formatter, Config config) {
                        formatter.format(config.<cur>)
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getTemplate", SymbolKind::Method));
                assert!(items.has("getTimeout", SymbolKind::Method));
            })
            .expected_failure("completion inside method argument not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_assignment_rhs() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    public void execute() {}
                    public String name() { return ""; }
                }
            "#)
            .file("com/example/Registry.java", r#"
                package com.example;
                public class Registry {
                    public Service lookup(String name) { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Registry registry) {
                        Service svc = registry.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("lookup", SymbolKind::Method));
            })
            .expected_failure("completion on assignment RHS not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_in_return_expression() {
        fixture()
            .file("com/example/Entity.java", r#"
                package com.example;
                public class Entity {
                    public int id;
                    public String label;
                }
            "#)
            .file("com/example/Repository.java", r#"
                package com.example;
                import java.util.List;
                public class Repository {
                    public Entity find(int id) { return null; }
                    public List<Entity> findAll() { return null; }
                    private Object conn;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public Entity getEntity(Repository repo) {
                        return repo.<cur>;
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("find", SymbolKind::Method));
                assert!(items.has("findAll", SymbolKind::Method));
                assert!(!items.has("conn", SymbolKind::Field));
            })
            .expected_failure("completion in return expression not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_this_in_constructor() {
        fixture()
            .file("com/example/Settings.java", r#"
                package com.example;
                public class Settings {
                    private String name;
                    private int timeout;
                    public void validate() {}

                    public Settings(String name, int timeout) {
                        this.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("name", SymbolKind::Field));
                assert!(items.has("timeout", SymbolKind::Field));
                assert!(items.has("validate", SymbolKind::Method));
            })
            .expected_failure("this completion inside constructor not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_method_parameter() {
        fixture()
            .file("com/example/Email.java", r#"
                package com.example;
                public class Email {
                    public String getSubject() { return ""; }
                    public String getBody() { return ""; }
                    private String raw;
                }
            "#)
            .file("com/example/Mailer.java", r#"
                package com.example;
                public class Mailer {
                    public void send(Email email) {
                        email.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getSubject", SymbolKind::Method));
                assert!(items.has("getBody", SymbolKind::Method));
                assert!(!items.has("raw", SymbolKind::Field));
            })
            .expected_failure("parameter type member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_local_variable_from_constructor() {
        fixture()
            .file("com/example/Timer.java", r#"
                package com.example;
                public class Timer {
                    public void start() {}
                    public void stop() {}
                    public long elapsed() { return 0L; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Timer t = new Timer();
                        t.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("start", SymbolKind::Method));
                assert!(items.has("stop", SymbolKind::Method));
                assert!(items.has("elapsed", SymbolKind::Method));
            })
            .expected_failure("local variable type member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_after_prior_statement_with_nested_calls() {
        fixture()
            .file("com/example/Request.java", r#"
                package com.example;
                public class Request {
                    public String getPath() { return ""; }
                    public String getMethod() { return ""; }
                }
            "#)
            .file("com/example/Logger.java", r#"
                package com.example;
                public class Logger {
                    public void log(String msg) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Logger logger, Request request) {
                        logger.log(request.getPath());
                        request.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getPath", SymbolKind::Method));
                assert!(items.has("getMethod", SymbolKind::Method));
            })
            .expected_failure("variable completion after prior nested-call statement not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_for_each_loop_variable() {
        fixture()
            .file("com/example/Item.java", r#"
                package com.example;
                public class Item {
                    public String label() { return ""; }
                    public double price() { return 0.0; }
                    private int sku;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import java.util.List;
                public class App {
                    public void run(List<Item> items) {
                        for (Item item : items) {
                            item.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("label", SymbolKind::Method));
                assert!(items.has("price", SymbolKind::Method));
                assert!(!items.has("sku", SymbolKind::Field));
            })
            .expected_failure("for-each loop variable completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_instanceof_pattern_variable() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public interface Shape {}
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public class Circle implements Shape {
                    public double radius() { return 0.0; }
                    public double area() { return 0.0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Shape shape) {
                        if (shape instanceof Circle c) {
                            c.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("radius", SymbolKind::Method));
                assert!(items.has("area", SymbolKind::Method));
            })
            .expected_failure("instanceof pattern variable completion not yet implemented")
            .run();
    }
}

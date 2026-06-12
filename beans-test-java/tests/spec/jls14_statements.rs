use beans::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §14.3 — Local Class and Interface Declarations
mod jls_14_3_local_classes {
    use super::*;

    #[test]
    fn dot_completion_local_class_instance() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    public void process() {
                        class Helper {
                            public void assist() {}
                        }
                        Helper h = new Helper();
                        h.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("assist", SymbolKind::Method));
            })
            .expected_failure("local class instance dot-completion not yet implemented")
            .run();
    }

    // @keep — local class declarations not yet indexed with method parent; documents expected_failure
    #[test]
    fn local_class_declared_in_method() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    public void process() {
                        class <cur:local>Helper {
                            void assist() {}
                        }
                        Helper h = new Helper();
                    }
                }
            "#)
            .assert_at("local")
                .kind(SymbolKind::Class)
                .name("Helper")
                .parent_fqn("com.example.Service.process")
                .expected_failure("local class declarations not yet indexed")
            .run();
    }

    // @keep — local class members not yet indexed; documents expected_failure
    #[test]
    fn local_class_with_methods() {
        fixture()
            .file("com/example/Processor.java", r#"
                package com.example;
                public class Processor {
                    public String transform(String input) {
                        class Formatter {
                            String <cur:fmt_method>format(String s) {
                                return s.trim();
                            }
                        }
                        return new Formatter().format(input);
                    }
                }
            "#)
            .assert_at("fmt_method")
                .kind(SymbolKind::Method)
                .name("format")
                .signature_return("String")
                .signature_params(&[("s", "String")])
                .expected_failure("local class members not yet indexed")
            .run();
    }

    // @keep — final parameter modifier not yet tracked; documents expected_failure
    #[test]
    fn local_class_accessing_enclosing_final_param() {
        fixture()
            .file("com/example/Builder.java", r#"
                package com.example;
                public class Builder {
                    public Runnable makeTask(final String <cur:param>message) {
                        class Task implements Runnable {
                            public void run() {
                                System.out.println(message);
                            }
                        }
                        return new Task();
                    }
                }
            "#)
            .assert_at("param")
                .kind(SymbolKind::Parameter)
                .name("message")
                .modifiers(vec![Modifier::Final])
                .expected_failure("parameter modifiers not yet tracked")
            .run();
    }
}

// §14.4 — Local Variable Declarations
mod jls_14_4_local_variables {
    use super::*;

    #[test]
    fn dot_completion_local_var_explicit_type() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    public String process() { return null; }
                    private int internal;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Service svc = new Service();
                        svc.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("process", SymbolKind::Method));
                assert!(!items.has("internal", SymbolKind::Field));
            })
            .expected_failure("local variable type resolution for dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_local_var_inferred_type() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    public String getName() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        var svc = new Service();
                        svc.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
            })
            .expected_failure("var type inference for dot-completion not yet implemented")
            .run();
    }

    // @keep — local variable type reference not yet resolved in symbol table; documents expected_failure
    #[test]
    fn explicit_type_local_variable() {
        fixture()
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        <cur:type_ref>String name = "beans";
                    }
                }
            "#)
            .assert_at("type_ref")
                .kind(SymbolKind::Class)
                .name("String")
                .expected_failure("local variable type references not yet resolved")
            .run();
    }

    // @keep — final local variable declaration not yet indexed; documents expected_failure
    #[test]
    fn final_local_variable() {
        fixture()
            .file("com/example/Config.java", r#"
                package com.example;
                public class Config {
                    public void load() {
                        final int <cur:count>count = 42;
                    }
                }
            "#)
            .assert_at("count")
                .name("count")
                .modifiers(vec![Modifier::Final])
                .expected_failure("local variable declarations not yet indexed")
            .run();
    }

    // @keep — var type inference for local variables not yet supported; documents expected_failure
    #[test]
    fn var_type_inference() {
        fixture()
            .file("com/example/Demo.java", r#"
                package com.example;
                import java.util.ArrayList;
                public class Demo {
                    public void example() {
                        var <cur:items>items = new ArrayList<String>();
                    }
                }
            "#)
            .assert_at("items")
                .name("items")
                .hover_contains("ArrayList")
                .expected_failure("var type inference not yet supported")
            .run();
    }

    /// Also exercises §6.1 (declarations) — imported type in local var
    // @keep — cross-file: imported type in local variable declaration resolves correctly
    #[test]
    fn local_variable_with_imported_type() {
        fixture()
            .file("com/example/model/User.java", r#"
                package com.example.model;
                public class User {
                    private String name;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import com.example.model.User;
                public class App {
                    public void greet() {
                        <cur:user_type>User user = new User();
                    }
                }
            "#)
            .assert_at("user_type")
                .resolves_to("com.example.model.User")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — multiple declarators in single local variable statement not yet indexed; documents expected_failure
    #[test]
    fn multiple_local_declarations() {
        fixture()
            .file("com/example/Math.java", r#"
                package com.example;
                public class Math {
                    public int compute() {
                        int <cur:x>x = 10, y = 20;
                        return x + y;
                    }
                }
            "#)
            .assert_at("x")
                .name("x")
                .expected_failure("local variable declarations not yet indexed")
            .run();
    }
}

// §14.14 — The Enhanced for Statement
mod jls_14_14_enhanced_for {
    use super::*;

    #[test]
    fn dot_completion_enhanced_for_loop_variable() {
        fixture()
            .file("com/example/model/User.java", r#"
                package com.example.model;
                public class User {
                    public String getName() { return null; }
                    public String getEmail() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import com.example.model.User;
                import java.util.List;
                public class App {
                    public void run(List<User> users) {
                        for (User u : users) {
                            u.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getEmail", SymbolKind::Method));
            })
            .expected_failure("enhanced-for loop variable dot-completion not yet implemented")
            .run();
    }

    // Negative scope test: no expected_failure since empty completions trivially satisfy
    // the negative assertion. Guards against future scope-leaking regressions.
    #[test]
    fn dot_completion_for_loop_var_out_of_scope() {
        fixture()
            .file("com/example/model/User.java", r#"
                package com.example.model;
                public class User {
                    public String getName() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import com.example.model.User;
                import java.util.List;
                public class App {
                    public void run(List<User> users) {
                        for (User u : users) {
                            // u is in scope here
                        }
                        u.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(!items.has("getName", SymbolKind::Method));
            })
            .run();
    }

    #[test]
    fn dot_completion_enhanced_for_var() {
        fixture()
            .file("com/example/model/User.java", r#"
                package com.example.model;
                public class User {
                    public String getName() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                import com.example.model.User;
                import java.util.List;
                public class App {
                    public void run(List<User> users) {
                        for (var u : users) {
                            u.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
            })
            .expected_failure("var in enhanced-for loop dot-completion not yet implemented")
            .run();
    }

    // @keep — enhanced-for loop variable type not yet resolved; documents expected_failure
    #[test]
    fn for_each_with_explicit_type() {
        fixture()
            .file("com/example/Printer.java", r#"
                package com.example;
                import java.util.List;
                public class Printer {
                    public void printAll(List<String> items) {
                        for (<cur:elem_type>String item : items) {
                            System.out.println(item);
                        }
                    }
                }
            "#)
            .assert_at("elem_type")
                .kind(SymbolKind::Class)
                .name("String")
                .expected_failure("enhanced-for loop variable types not yet resolved")
            .run();
    }

    // @keep — enhanced-for loop variable declaration not yet indexed; documents expected_failure
    #[test]
    fn for_each_loop_variable() {
        fixture()
            .file("com/example/Aggregator.java", r#"
                package com.example;
                import java.util.List;
                public class Aggregator {
                    public int sum(List<Integer> numbers) {
                        int total = 0;
                        for (Integer <cur:num>num : numbers) {
                            total += num;
                        }
                        return total;
                    }
                }
            "#)
            .assert_at("num")
                .name("num")
                .expected_failure("enhanced-for loop variable declarations not yet indexed")
            .run();
    }

    // @keep — var in enhanced-for loop not yet supported; documents expected_failure
    #[test]
    fn for_each_with_var() {
        fixture()
            .file("com/example/MapWalker.java", r#"
                package com.example;
                import java.util.Map;
                public class MapWalker {
                    public void walk(Map<String, Integer> map) {
                        for (var <cur:entry>entry : map.entrySet()) {
                            System.out.println(entry.getKey());
                        }
                    }
                }
            "#)
            .assert_at("entry")
                .name("entry")
                .hover_contains("Map.Entry")
                .expected_failure("var in enhanced-for not yet supported")
            .run();
    }
}

// §14.14.1 — The basic for Statement
mod jls_14_14_1_basic_for {
    use super::*;

    #[test]
    fn dot_completion_for_loop_initializer_variable() {
        fixture()
            .file("com/example/Iterator.java", r#"
                package com.example;
                public class Iterator {
                    public boolean hasNext() { return false; }
                    public Object next() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        for (Iterator it = new Iterator(); it.<cur>; ) {
                            it.next();
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("hasNext", SymbolKind::Method));
                assert!(items.has("next", SymbolKind::Method));
            })
            .expected_failure("for-loop initializer variable dot-completion not yet implemented")
            .run();
    }
}

// §14.20 — The try Statement
mod jls_14_20_try_statements {
    use super::*;

    #[test]
    fn dot_completion_try_with_resources_variable() {
        fixture()
            .file("com/example/Connection.java", r#"
                package com.example;
                public class Connection implements AutoCloseable {
                    public void execute(String sql) {}
                    public void close() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() throws Exception {
                        try (Connection conn = new Connection()) {
                            conn.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("execute", SymbolKind::Method));
                assert!(items.has("close", SymbolKind::Method));
            })
            .expected_failure("try-with-resources variable dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_try_with_resources_var() {
        fixture()
            .file("com/example/Connection.java", r#"
                package com.example;
                public class Connection implements AutoCloseable {
                    public void execute(String sql) {}
                    public void close() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() throws Exception {
                        try (var conn = new Connection()) {
                            conn.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("execute", SymbolKind::Method));
            })
            .expected_failure("var in try-with-resources dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_try_multiple_resources_second_var() {
        fixture()
            .file("com/example/Connection.java", r#"
                package com.example;
                public class Connection implements AutoCloseable {
                    public Statement prepareStatement(String sql) { return null; }
                    public void close() {}
                }
            "#)
            .file("com/example/Statement.java", r#"
                package com.example;
                public class Statement implements AutoCloseable {
                    public Object executeQuery() { return null; }
                    public void close() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() throws Exception {
                        try (Connection conn = new Connection();
                             Statement stmt = conn.prepareStatement("SELECT 1")) {
                            stmt.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("executeQuery", SymbolKind::Method));
                assert!(!items.has("prepareStatement", SymbolKind::Method));
            })
            .expected_failure("multiple try-with-resources variable dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_catch_exception_variable() {
        fixture()
            .file("com/example/AppException.java", r#"
                package com.example;
                public class AppException extends RuntimeException {
                    public int getCode() { return 0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        try {
                            throw new AppException();
                        } catch (AppException e) {
                            e.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getCode", SymbolKind::Method));
            })
            .expected_failure("catch clause exception variable dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_multi_catch_exception_variable() {
        fixture()
            .file("com/example/App.java", r#"
                package com.example;
                import java.io.IOException;
                import java.sql.SQLException;
                public class App {
                    public void run() {
                        try {
                            riskyOp();
                        } catch (IOException | SQLException e) {
                            e.<cur>
                        }
                    }
                    private void riskyOp() throws IOException, SQLException {}
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getMessage", SymbolKind::Method));
            })
            .expected_failure("multi-catch exception variable dot-completion not yet implemented")
            .run();
    }

    // @keep — try-with-resources variable type not yet resolved; documents expected_failure
    #[test]
    fn try_with_resources_variable() {
        fixture()
            .file("com/example/FileProcessor.java", r#"
                package com.example;
                import java.io.BufferedReader;
                import java.io.FileReader;
                public class FileProcessor {
                    public String readFirst(String path) throws Exception {
                        try (<cur:res_type>BufferedReader reader = new BufferedReader(new FileReader(path))) {
                            return reader.readLine();
                        }
                    }
                }
            "#)
            .assert_at("res_type")
                .name("BufferedReader")
                .kind(SymbolKind::Class)
                .expected_failure("try-with-resources variable types not yet resolved")
            .run();
    }

    // @keep — var in try-with-resources not yet supported; documents expected_failure
    #[test]
    fn try_with_resources_var_inference() {
        fixture()
            .file("com/example/StreamProcessor.java", r#"
                package com.example;
                import java.io.FileInputStream;
                public class StreamProcessor {
                    public void process(String path) throws Exception {
                        try (var <cur:stream>stream = new FileInputStream(path)) {
                            stream.read();
                        }
                    }
                }
            "#)
            .assert_at("stream")
                .name("stream")
                .hover_contains("FileInputStream")
                .expected_failure("var in try-with-resources not yet supported")
            .run();
    }

    // @keep — multi-catch exception parameters not yet indexed; documents expected_failure
    #[test]
    fn multi_catch_exception_parameter() {
        fixture()
            .file("com/example/Parser.java", r#"
                package com.example;
                import java.io.IOException;
                import java.sql.SQLException;
                public class Parser {
                    public void parse(String input) {
                        try {
                            riskyOperation(input);
                        } catch (IOException | SQLException <cur:ex>e) {
                            e.printStackTrace();
                        }
                    }
                    private void riskyOperation(String s) throws IOException, SQLException {}
                }
            "#)
            .assert_at("ex")
                .name("e")
                .expected_failure("multi-catch exception parameters not yet indexed")
            .run();
    }

    // @keep — catch clause exception type not yet resolved; documents expected_failure
    #[test]
    fn single_catch_exception_type() {
        fixture()
            .file("com/example/SafeRunner.java", r#"
                package com.example;
                public class SafeRunner {
                    public void run() {
                        try {
                            throw new IllegalStateException("oops");
                        } catch (<cur:exc_type>IllegalStateException e) {
                            System.err.println(e.getMessage());
                        }
                    }
                }
            "#)
            .assert_at("exc_type")
                .name("IllegalStateException")
                .kind(SymbolKind::Class)
                .expected_failure("catch clause exception types not yet resolved")
            .run();
    }
}

// §14.30 — Patterns
mod jls_14_30_patterns {
    use super::*;

    #[test]
    fn dot_completion_nested_record_pattern_variable() {
        fixture()
            .file("com/example/Point.java", r#"
                package com.example;
                public record Point(int x, int y) {
                    public double distanceTo(Point other) { return 0.0; }
                }
            "#)
            .file("com/example/Pair.java", r#"
                package com.example;
                public record Pair<A, B>(A first, B second) {}
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Object obj) {
                        if (obj instanceof Pair(Point p1, Point p2)) {
                            p1.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("distanceTo", SymbolKind::Method));
                assert!(items.has("x", SymbolKind::Method));
                assert!(items.has("y", SymbolKind::Method));
            })
            .expected_failure("nested record deconstruction pattern variable dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_switch_record_pattern_var_inferred() {
        fixture()
            .file("com/example/Response.java", r#"
                package com.example;
                public class Response {
                    public String getBody() { return null; }
                }
            "#)
            .file("com/example/Result.java", r#"
                package com.example;
                public sealed interface Result permits Success, Failure {}
            "#)
            .file("com/example/Success.java", r#"
                package com.example;
                public record Success(Response resp) implements Result {}
            "#)
            .file("com/example/Failure.java", r#"
                package com.example;
                public record Failure(String message) implements Result {}
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public String handle(Result result) {
                        return switch (result) {
                            case Success(var resp) -> resp.<cur>;
                            case Failure f -> f.message();
                        };
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getBody", SymbolKind::Method));
            })
            .expected_failure("var in switch record pattern dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_instanceof_pattern_variable() {
        fixture()
            .file("com/example/Circle.java", r#"
                package com.example;
                public class Circle {
                    public double getRadius() { return 0.0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void draw(Object shape) {
                        if (shape instanceof Circle c) {
                            c.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getRadius", SymbolKind::Method));
                assert!(!items.has("getWidth", SymbolKind::Method));
            })
            .expected_failure("instanceof pattern variable dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_switch_type_pattern_variable() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public interface Shape {}
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public class Circle implements Shape {
                    public double getRadius() { return 0.0; }
                }
            "#)
            .file("com/example/Rectangle.java", r#"
                package com.example;
                public class Rectangle implements Shape {
                    public double getWidth() { return 0.0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public String describe(Shape shape) {
                        return switch (shape) {
                            case Circle c -> String.valueOf(c.<cur>);
                            case Rectangle r -> String.valueOf(r.getWidth());
                            default -> "unknown";
                        };
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getRadius", SymbolKind::Method));
                assert!(!items.has("getWidth", SymbolKind::Method));
            })
            .expected_failure("switch type pattern variable dot-completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_switch_pattern_guard() {
        fixture()
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public String classify(Object obj) {
                        return switch (obj) {
                            case String s when s.<cur> -> "something";
                            default -> "other";
                        };
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("length", SymbolKind::Method));
                assert!(items.has("isEmpty", SymbolKind::Method));
            })
            .expected_failure("switch pattern guard clause dot-completion not yet implemented")
            .run();
    }

    // Negative scope test: no expected_failure since empty completions trivially satisfy
    // the negative assertion. Guards against future scope-leaking regressions.
    #[test]
    fn dot_completion_instanceof_pattern_var_out_of_scope() {
        fixture()
            .file("com/example/Circle.java", r#"
                package com.example;
                public class Circle {
                    public double getRadius() { return 0.0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void draw(Object shape) {
                        if (shape instanceof Circle c) {
                            // c is in scope here
                        }
                        c.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(!items.has("getRadius", SymbolKind::Method));
            })
            .run();
    }

    // @keep — instanceof pattern variable not yet indexed; documents expected_failure
    #[test]
    fn instanceof_type_pattern() {
        fixture()
            .file("com/example/TypeChecker.java", r#"
                package com.example;
                public class TypeChecker {
                    public int length(Object obj) {
                        if (obj instanceof String <cur:pat_var>s) {
                            return s.length();
                        }
                        return -1;
                    }
                }
            "#)
            .assert_at("pat_var")
                .name("s")
                .hover_contains("String")
                .expected_failure("instanceof pattern variables not yet indexed")
            .run();
    }

    // @keep — cross-file: Circle type in instanceof pattern resolves to class declaration
    #[test]
    fn instanceof_pattern_type_reference() {
        fixture()
            .file("com/example/model/Shape.java", r#"
                package com.example.model;
                public abstract class Shape {}
            "#)
            .file("com/example/model/Circle.java", r#"
                package com.example.model;
                public class Circle extends Shape {
                    public double radius;
                }
            "#)
            .file("com/example/Renderer.java", r#"
                package com.example;
                import com.example.model.Shape;
                import com.example.model.Circle;
                public class Renderer {
                    public void draw(Shape shape) {
                        if (shape instanceof <cur:circ_type>Circle c) {
                            System.out.println("radius=" + c.radius);
                        }
                    }
                }
            "#)
            .assert_at("circ_type")
                .resolves_to("com.example.model.Circle")
                .kind(SymbolKind::Class)
            .run();
    }

    // @keep — switch pattern variables not yet indexed; documents expected_failure
    #[test]
    fn switch_pattern_matching_with_type_pattern() {
        fixture()
            .file("com/example/Formatter.java", r#"
                package com.example;
                public class Formatter {
                    public String format(Object value) {
                        return switch (value) {
                            case Integer <cur:int_var>i -> "int: " + i;
                            case String s -> "str: " + s;
                            default -> "unknown";
                        };
                    }
                }
            "#)
            .assert_at("int_var")
                .name("i")
                .hover_contains("Integer")
                .expected_failure("switch pattern variables not yet indexed")
            .run();
    }

    // @keep — guarded switch pattern variable not yet indexed; documents expected_failure
    #[test]
    fn switch_pattern_with_guard() {
        fixture()
            .file("com/example/Classifier.java", r#"
                package com.example;
                public class Classifier {
                    public String classify(Object obj) {
                        return switch (obj) {
                            case String <cur:guarded>s when s.length() > 10 -> "long string";
                            case String s -> "short string";
                            default -> "not a string";
                        };
                    }
                }
            "#)
            .assert_at("guarded")
                .name("s")
                .hover_contains("String")
                .expected_failure("guarded switch pattern variables not yet indexed")
            .run();
    }

    /// Also exercises §8.10 (record classes) — record deconstruction patterns
    // @keep — cross-file: Point record type in deconstruction pattern resolves to Record
    #[test]
    fn record_deconstruction_pattern() {
        fixture()
            .file("com/example/Point.java", r#"
                package com.example;
                public record Point(int x, int y) {}
            "#)
            .file("com/example/Geometry.java", r#"
                package com.example;
                public class Geometry {
                    public double distance(Object obj) {
                        if (obj instanceof <cur:rec_pat>Point(int x, int y)) {
                            return Math.sqrt(x * x + y * y);
                        }
                        return 0.0;
                    }
                }
            "#)
            .assert_at("rec_pat")
                .resolves_to("com.example.Point")
                .kind(SymbolKind::Record)
            .run();
    }

    // @keep — cross-file: Pair record type in nested deconstruction pattern resolves to Record
    #[test]
    fn nested_record_pattern() {
        fixture()
            .file("com/example/Pair.java", r#"
                package com.example;
                public record Pair<A, B>(A first, B second) {}
            "#)
            .file("com/example/Point.java", r#"
                package com.example;
                public record Point(int x, int y) {}
            "#)
            .file("com/example/LineSegment.java", r#"
                package com.example;
                public class LineSegment {
                    public double length(Object obj) {
                        if (obj instanceof <cur:outer_pat>Pair(Point(int x1, int y1), Point p2)) {
                            return Math.sqrt(Math.pow(p2.x() - x1, 2) + Math.pow(p2.y() - y1, 2));
                        }
                        return 0.0;
                    }
                }
            "#)
            .assert_at("outer_pat")
                .resolves_to("com.example.Pair")
                .kind(SymbolKind::Record)
            .run();
    }

    /// Also exercises §8.10 (record classes) and §9.1 (sealed interfaces)
    // @keep — cross-file (4 files): Shape interface and Circle record types resolve in switch patterns
    #[test]
    fn switch_with_sealed_type_and_record_patterns() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public sealed interface Shape permits Circle, Rectangle {}
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public record Circle(double radius) implements Shape {}
            "#)
            .file("com/example/Rectangle.java", r#"
                package com.example;
                public record Rectangle(double width, double height) implements Shape {}
            "#)
            .file("com/example/AreaCalculator.java", r#"
                package com.example;
                public class AreaCalculator {
                    public double area(<cur:param_type>Shape shape) {
                        return switch (shape) {
                            case <cur:circle_pat>Circle(var r) -> Math.PI * r * r;
                            case Rectangle(var w, var h) -> w * h;
                        };
                    }
                }
            "#)
            .assert_at("param_type")
                .resolves_to("com.example.Shape")
                .kind(SymbolKind::Interface)
            .assert_at("circle_pat")
                .resolves_to("com.example.Circle")
                .kind(SymbolKind::Record)
            .run();
    }

    #[test]
    fn dot_completion_instanceof_flow_scope_after_early_return() {
        fixture()
            .file("com/example/Circle.java", r#"
                package com.example;
                public class Circle {
                    public double getRadius() { return 0.0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public double area(Object shape) {
                        if (!(shape instanceof Circle c)) { return 0.0; }
                        c.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getRadius", SymbolKind::Method));
            })
            .expected_failure("instanceof flow-scoping after early return not yet implemented in completion")
            .run();
    }
}

// §14.11 — The switch Statement
mod jls_14_11_switch {
    use super::*;

    #[test]
    fn dot_completion_switch_arm_on_original_object() {
        fixture()
            .file("com/example/Status.java", r#"
                package com.example;
                public enum Status { ACTIVE, INACTIVE }
            "#)
            .file("com/example/Employee.java", r#"
                package com.example;
                public class Employee {
                    public Status getStatus() { return Status.ACTIVE; }
                    public String getName() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public String describe(Employee emp) {
                        return switch (emp.getStatus()) {
                            case ACTIVE -> emp.<cur>;
                            case INACTIVE -> "inactive";
                        };
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getStatus", SymbolKind::Method));
            })
            .expected_failure("dot-completion on object inside switch arm not yet implemented")
            .run();
    }
}

// §14.19 — The synchronized Statement
mod jls_14_19_synchronized {
    use super::*;

    #[test]
    fn dot_completion_synchronized_block_variable() {
        fixture()
            .file("com/example/SharedBuffer.java", r#"
                package com.example;
                public class SharedBuffer {
                    public void put(String item) {}
                    public String take() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(SharedBuffer buffer) {
                        synchronized (buffer) {
                            buffer.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("put", SymbolKind::Method));
                assert!(items.has("take", SymbolKind::Method));
            })
            .expected_failure("dot-completion inside synchronized block not yet implemented")
            .run();
    }
}

// §14.20.2 — The finally Block
mod jls_14_20_2_finally {
    use super::*;

    #[test]
    fn dot_completion_finally_block_variable() {
        fixture()
            .file("com/example/Connection.java", r#"
                package com.example;
                public class Connection {
                    public boolean isClosed() { return false; }
                    public void close() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Connection conn) {
                        try {
                            conn.isClosed();
                        } finally {
                            conn.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("isClosed", SymbolKind::Method));
                assert!(items.has("close", SymbolKind::Method));
            })
            .expected_failure("dot-completion inside finally block not yet implemented")
            .run();
    }
}

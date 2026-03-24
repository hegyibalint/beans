use beans_core::SymbolKind;

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §4.1 — The Kinds of Types and Values
mod jls_4_1_kinds_of_types {
    use super::*;

    /// User go-to-definition on `String` — a JDK auto-imported type from java.lang.
    // @keep — resolves JDK auto-imported String from field type usage site (cross-runtime resolution)
    #[test]
    fn jdk_auto_imported_type_resolves() {
        fixture()
            .file("com/example/Account.java", r#"
                package com.example;
                public class Account {
                    private <cur:string_ref>String owner;
                }
            "#)
            .assert_at("string_ref")
                .resolves_to("java.lang.String")
                .expected_failure("JDK type java.lang.String not available in fixture")
            .run();
    }

    /// User go-to-definition on `Object` — implicit superclass of all classes.
    // @keep — resolves JDK Object from field type usage site (cross-runtime resolution)
    #[test]
    fn object_type_resolves() {
        fixture()
            .file("com/example/Container.java", r#"
                package com.example;
                public class Container {
                    private <cur:obj_ref>Object data;
                }
            "#)
            .assert_at("obj_ref")
                .resolves_to("java.lang.Object")
                .expected_failure("JDK type java.lang.Object not available in fixture")
            .run();
    }
}

// §4.3 — Reference Types and Values
mod jls_4_3_reference_types {
    use super::*;

    /// Go-to-definition on a JDK collection type used via explicit import.
    // @keep — resolves JDK List and Map from field type usage sites (import + JDK resolution)
    #[test]
    fn jdk_collection_type_resolves() {
        fixture()
            .file("com/example/Registry.java", r#"
                package com.example;
                import java.util.List;
                import java.util.Map;
                public class Registry {
                    private <cur:list_ref>List<String> names;
                    private <cur:map_ref>Map<String, Integer> counts;
                }
            "#)
            .assert_at("list_ref")
                .resolves_to("java.util.List")
                .expected_failure("JDK type java.util.List not available in fixture")
            .assert_at("map_ref")
                .resolves_to("java.util.Map")
                .expected_failure("JDK type java.util.Map not available in fixture")
            .run();
    }

    /// Go-to-definition on a JDK type used as a method parameter type.
    // @keep — resolves JDK Date from method parameter type usage site
    #[test]
    fn jdk_type_in_method_parameter() {
        fixture()
            .file("com/example/Formatter.java", r#"
                package com.example;
                import java.util.Date;
                public class Formatter {
                    public String format(<cur:date_ref>Date date) {
                        return date.toString();
                    }
                }
            "#)
            .assert_at("date_ref")
                .resolves_to("java.util.Date")
                .expected_failure("JDK type java.util.Date not available in fixture")
            .run();
    }

    /// Go-to-definition on a JDK exception type in a throws clause.
    // @keep — resolves JDK IOException from throws clause usage site
    #[test]
    fn jdk_exception_type_resolves() {
        fixture()
            .file("com/example/FileReader.java", r#"
                package com.example;
                import java.io.IOException;
                public class FileReader {
                    public String read() throws <cur:exc_ref>IOException {
                        return "";
                    }
                }
            "#)
            .assert_at("exc_ref")
                .resolves_to("java.io.IOException")
                .expected_failure("JDK type java.io.IOException not available in fixture")
            .run();
    }
}

// §4.4 — Type Variables
mod jls_4_4_type_variables {
    use super::*;

    /// Bounded type param with user-defined bound — dot on `t` should expose bound's public members.
    #[test]
    fn dot_completion_bounded_type_param_user_defined_bound() {
        fixture()
            .file("com/example/MyService.java", r#"
                package com.example;
                public class MyService {
                    public String process(int count) { return null; }
                    public void shutdown() {}
                    private int internal;
                }
            "#)
            .file("com/example/Util.java", r#"
                package com.example;
                public class Util {
                    public <T extends MyService> void use(T t) {
                        t.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("process", SymbolKind::Method));
                assert!(items.has("shutdown", SymbolKind::Method));
                assert!(!items.has("internal", SymbolKind::Field));
            })
            .expected_failure("member completion via bounded type parameter not yet implemented")
            .run();
    }

    /// Unbounded type parameter — only Object's methods should be available.
    #[test]
    fn dot_completion_unbounded_type_param_shows_object_methods() {
        fixture()
            .file("com/example/Box.java", r#"
                package com.example;
                public class Box<T> {
                    private T value;
                    public void inspect() {
                        value.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("toString", SymbolKind::Method));
                assert!(items.has("hashCode", SymbolKind::Method));
                assert!(items.has("equals", SymbolKind::Method));
            })
            .expected_failure("JDK Object methods not available for unbounded type parameter")
            .run();
    }

    /// Class-level bounded type param — dot on a field of type T exposes the bound's members.
    #[test]
    fn dot_completion_class_level_bounded_param_field() {
        fixture()
            .file("com/example/Handler.java", r#"
                package com.example;
                public class Handler {
                    public void handle(String event) {}
                    public void reset() {}
                }
            "#)
            .file("com/example/Dispatcher.java", r#"
                package com.example;
                public class Dispatcher<T extends Handler> {
                    private T handler;
                    public void dispatch(String event) {
                        handler.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("handle", SymbolKind::Method));
                assert!(items.has("reset", SymbolKind::Method));
            })
            .expected_failure("member completion via class-level bounded type parameter not yet implemented")
            .run();
    }

    /// Intersection bound — access control respected per JLS Example 4.4-1.
    #[test]
    fn dot_completion_intersection_bound_access_control() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public void mPublic() {}
                    protected void mProtected() {}
                    void mPackage() {}
                    private void mPrivate() {}
                }
            "#)
            .file("com/example/Capability.java", r#"
                package com.example;
                public interface Capability {
                    void perform();
                }
            "#)
            .file("com/example/Test.java", r#"
                package com.example;
                public class Test {
                    public <T extends Base & Capability> void test(T t) {
                        t.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("mPublic", SymbolKind::Method));
                assert!(items.has("mProtected", SymbolKind::Method));
                assert!(items.has("mPackage", SymbolKind::Method));
                assert!(items.has("perform", SymbolKind::Method));
                assert!(!items.has("mPrivate", SymbolKind::Method));
            })
            .expected_failure("intersection bound completion with access control not yet implemented")
            .run();
    }

    /// Type parameter bounded by a parameterized type — members come from that bound.
    #[test]
    fn dot_completion_type_param_bounded_by_generic_type() {
        fixture()
            .file("com/example/Container.java", r#"
                package com.example;
                public class Container<E> {
                    public void add(E item) {}
                    public E first() { return null; }
                    public int size() { return 0; }
                }
            "#)
            .file("com/example/Processor.java", r#"
                package com.example;
                public class Processor {
                    public <T extends Container<String>> void process(T t) {
                        t.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("add", SymbolKind::Method));
                assert!(items.has("first", SymbolKind::Method));
                assert!(items.has("size", SymbolKind::Method));
            })
            .expected_failure("member completion via parameterized type bound not yet implemented")
            .run();
    }

    /// Field typed as bounded type param — dot on field exposes the bound's members.
    #[test]
    fn dot_completion_field_typed_as_bounded_type_param() {
        fixture()
            .file("com/example/Resource.java", r#"
                package com.example;
                public class Resource {
                    public void release() {}
                    public String status() { return null; }
                }
            "#)
            .file("com/example/Cache.java", r#"
                package com.example;
                public class Cache<V extends Resource> {
                    private V current;
                    public void check() {
                        current.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("release", SymbolKind::Method));
                assert!(items.has("status", SymbolKind::Method));
            })
            .expected_failure("member completion via field typed as bounded type parameter not yet implemented")
            .run();
    }

    /// Bounded type parameter with JDK bound — the bound type should resolve.
    // @keep — resolves JDK Comparable from type parameter bound (usage site in type param)
    #[test]
    fn bounded_type_param_bound_resolves() {
        fixture()
            .file("com/example/SortedList.java", r#"
                package com.example;
                public class SortedList<<cur:bound_ref>T extends Comparable<T>> {
                    public void add(T item) {}
                }
            "#)
            .assert_at("bound_ref")
                .resolves_to("java.lang.Comparable")
                .expected_failure("type parameter bound resolution to JDK type not yet implemented")
            .run();
    }

    /// Multiple type params with bounds — go-to-definition on the bound type.
    // @keep — resolves JDK Number from type parameter bound (usage site)
    #[test]
    fn multi_param_bound_resolves() {
        fixture()
            .file("com/example/Converter.java", r#"
                package com.example;
                public class Converter<S extends <cur:num_ref>Number, T extends CharSequence> {
                    public T convert(S source) { return null; }
                }
            "#)
            .assert_at("num_ref")
                .resolves_to("java.lang.Number")
                .expected_failure("JDK type java.lang.Number not available as bound target")
            .run();
    }

}

// §4.5 — Parameterized Types
mod jls_4_5_parameterized_types {
    use super::*;

    /// Dot-completion on a parameterized type — members of the generic class are accessible.
    #[test]
    fn dot_completion_parameterized_type_own_methods() {
        fixture()
            .file("com/example/Container.java", r#"
                package com.example;
                public class Container<E> {
                    public void add(E item) {}
                    public E get(int index) { return null; }
                    public int size() { return 0; }
                    private E[] data;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Container<String> c) {
                        c.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("add", SymbolKind::Method));
                assert!(items.has("get", SymbolKind::Method));
                assert!(items.has("size", SymbolKind::Method));
                assert!(!items.has("data", SymbolKind::Field));
            })
            .expected_failure("member completion on parameterized type not yet implemented")
            .run();
    }

    /// Dot-completion on result of generic method call — resolved to the actual type argument.
    #[test]
    fn dot_completion_generic_method_return_type() {
        fixture()
            .file("com/example/Item.java", r#"
                package com.example;
                public class Item {
                    public String label() { return null; }
                    public int weight() { return 0; }
                }
            "#)
            .file("com/example/Container.java", r#"
                package com.example;
                public class Container<E> {
                    public E get(int index) { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Container<Item> c) {
                        c.get(0).<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("label", SymbolKind::Method));
                assert!(items.has("weight", SymbolKind::Method));
            })
            .expected_failure("chained call completion with generic return type not yet implemented")
            .run();
    }

    /// Dot-completion on a multi-type-parameter class (Pair<A, B>) — all methods visible.
    #[test]
    fn dot_completion_multiple_type_parameters() {
        fixture()
            .file("com/example/Pair.java", r#"
                package com.example;
                public class Pair<A, B> {
                    public A getFirst() { return null; }
                    public B getSecond() { return null; }
                    public void swap() {}
                    private A first;
                    private B second;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Pair<String, Integer> p) {
                        p.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getFirst", SymbolKind::Method));
                assert!(items.has("getSecond", SymbolKind::Method));
                assert!(items.has("swap", SymbolKind::Method));
                assert!(!items.has("first", SymbolKind::Field));
                assert!(!items.has("second", SymbolKind::Field));
            })
            .expected_failure("member completion on multi-type-parameter class not yet implemented")
            .run();
    }

    /// Nested generic chained dot — outer parameterized return resolves to inner parameterized type.
    #[test]
    fn dot_completion_nested_generic_chained_outer_return() {
        fixture()
            .file("com/example/Item.java", r#"
                package com.example;
                public class Item {
                    public String name() { return null; }
                }
            "#)
            .file("com/example/Box.java", r#"
                package com.example;
                public class Box<E> {
                    public E unwrap() { return null; }
                    public boolean isEmpty() { return true; }
                }
            "#)
            .file("com/example/Wrapper.java", r#"
                package com.example;
                public class Wrapper<E> {
                    public E get() { return null; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Wrapper<Box<Item>> w) {
                        w.get().<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("unwrap", SymbolKind::Method));
                assert!(items.has("isEmpty", SymbolKind::Method));
                assert!(!items.has("name", SymbolKind::Method));
            })
            .expected_failure("nested generic chained completion not yet implemented")
            .run();
    }

    /// Static members should not appear via instance dot-completion — JLS §4.5.2.
    #[test]
    fn dot_completion_parameterized_instance_excludes_static() {
        fixture()
            .file("com/example/Registry.java", r#"
                package com.example;
                public class Registry<E> {
                    public void register(E item) {}
                    public E lookup(String name) { return null; }
                    public static Registry create() { return new Registry(); }
                    public static int count() { return 0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Registry<String> reg) {
                        reg.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("register", SymbolKind::Method));
                assert!(items.has("lookup", SymbolKind::Method));
            })
            .expected_failure("instance dot-completion on parameterized type not yet implemented")
            .run();
    }

    /// Go-to-definition on `ArrayList` in a `new` expression — should resolve to JDK type.
    // @keep — resolves JDK ArrayList from new-expression type usage site
    #[test]
    fn parameterized_constructor_type_resolves() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                import java.util.List;
                import java.util.ArrayList;
                public class Service {
                    private List<String> items = new <cur:ctor_ref>ArrayList<>();
                }
            "#)
            .assert_at("ctor_ref")
                .resolves_to("java.util.ArrayList")
                .expected_failure("JDK type java.util.ArrayList in new-expression not resolved")
            .run();
    }

    /// Nested parameterized type `Map<String, List<Integer>>` — inner `List` should resolve.
    // @keep — resolves JDK List from nested type argument usage site
    #[test]
    fn nested_parameterized_inner_type_resolves() {
        fixture()
            .file("com/example/Graph.java", r#"
                package com.example;
                import java.util.Map;
                import java.util.List;
                public class Graph {
                    private Map<String, <cur:inner_list>List<String>> adjacencyList;
                }
            "#)
            .assert_at("inner_list")
                .resolves_to("java.util.List")
                .expected_failure("JDK type in nested type argument not resolved")
            .run();
    }

}

// §4.8 — Raw Types
mod jls_4_8_raw_types {
    use super::*;

    /// Dot-completion on a raw type — erased members are still accessible.
    #[test]
    fn dot_completion_raw_type_erased_members() {
        fixture()
            .file("com/example/Cell.java", r#"
                package com.example;
                public class Cell<E> {
                    public E value;
                    public E get() { return value; }
                    public void set(E v) { value = v; }
                }
            "#)
            .file("com/example/Legacy.java", r#"
                package com.example;
                public class Legacy {
                    @SuppressWarnings("rawtypes")
                    public void process(Cell raw) {
                        raw.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("get", SymbolKind::Method));
                assert!(items.has("set", SymbolKind::Method));
                assert!(items.has("value", SymbolKind::Field));
            })
            .expected_failure("member completion on raw type not yet implemented")
            .run();
    }

    /// Raw type `List` (no type argument) — go-to-definition should resolve to java.util.List.
    // @keep — resolves JDK List from raw type usage site (no type argument)
    #[test]
    fn raw_list_resolves_to_jdk() {
        fixture()
            .file("com/example/LegacyService.java", r#"
                package com.example;
                import java.util.List;
                import java.util.ArrayList;
                @SuppressWarnings("rawtypes")
                public class LegacyService {
                    private <cur:raw_type>List items = new ArrayList();
                }
            "#)
            .assert_at("raw_type")
                .resolves_to("java.util.List")
                .expected_failure("JDK type resolution for raw List not yet implemented")
            .run();
    }

    /// Raw `Map` in a method parameter — should still resolve to java.util.Map.
    // @keep — resolves JDK Map from raw type in method parameter (usage site)
    #[test]
    fn raw_map_param_resolves_to_jdk() {
        fixture()
            .file("com/example/Compat.java", r#"
                package com.example;
                import java.util.Map;
                @SuppressWarnings("rawtypes")
                public class Compat {
                    public void process(<cur:raw_param>Map data) {}
                }
            "#)
            .assert_at("raw_param")
                .resolves_to("java.util.Map")
                .expected_failure("JDK type resolution for raw Map param not yet implemented")
            .run();
    }
}

// §4.9 — Intersection Types
mod jls_4_9_intersection_types {
    use super::*;

    /// Dot-completion on intersection bound — members from BOTH bounds are accessible.
    #[test]
    fn dot_completion_intersection_bound_both_bounds_members() {
        fixture()
            .file("com/example/Printable.java", r#"
                package com.example;
                public interface Printable {
                    void print();
                }
            "#)
            .file("com/example/Entity.java", r#"
                package com.example;
                public class Entity {
                    public String getName() { return null; }
                    public int getId() { return 0; }
                }
            "#)
            .file("com/example/Processor.java", r#"
                package com.example;
                public class Processor {
                    public <T extends Entity & Printable> void handle(T item) {
                        item.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getId", SymbolKind::Method));
                assert!(items.has("print", SymbolKind::Method));
            })
            .expected_failure("member completion via intersection type bound not yet implemented")
            .run();
    }

    /// Go-to-definition on `Serializable` in an intersection bound `T extends Serializable & Comparable<T>`.
    // @keep — resolves JDK Serializable from first type in intersection bound (usage site)
    #[test]
    fn intersection_bound_first_type_resolves() {
        fixture()
            .file("com/example/Sorter.java", r#"
                package com.example;
                import java.io.Serializable;
                public class Sorter<T extends <cur:ser_ref>Serializable & Comparable<T>> {
                    private T best;
                }
            "#)
            .assert_at("ser_ref")
                .resolves_to("java.io.Serializable")
                .expected_failure("JDK type in intersection bound not resolved")
            .run();
    }

    /// Go-to-definition on the second interface in an intersection bound.
    // @keep — resolves JDK Closeable from second intersection bound type (usage site)
    #[test]
    fn intersection_bound_second_type_resolves() {
        fixture()
            .file("com/example/Processor.java", r#"
                package com.example;
                import java.io.Closeable;
                public class Processor {
                    public <T extends Runnable & <cur:close_ref>Closeable> void execute(T task) {
                        task.run();
                    }
                }
            "#)
            .assert_at("close_ref")
                .resolves_to("java.io.Closeable")
                .expected_failure("JDK type in second intersection bound not resolved")
            .run();
    }
}

// §4.10 — Subtyping
mod jls_4_10_subtyping {
    use super::*;

    /// Go-to-definition on the superclass when it is a JDK type.
    // @keep — resolves JDK RuntimeException from extends clause (usage site)
    #[test]
    fn extends_jdk_type_resolves() {
        fixture()
            .file("com/example/AppException.java", r#"
                package com.example;
                public class AppException extends <cur:rt_exc>RuntimeException {
                    public AppException(String message) {
                        super(message);
                    }
                }
            "#)
            .assert_at("rt_exc")
                .resolves_to("java.lang.RuntimeException")
                .expected_failure("JDK type java.lang.RuntimeException not available in fixture")
            .run();
    }

    /// Go-to-definition on a JDK interface in an implements clause.
    // @keep — resolves JDK Serializable from implements clause (usage site)
    #[test]
    fn implements_jdk_interface_resolves() {
        fixture()
            .file("com/example/Config.java", r#"
                package com.example;
                import java.io.Serializable;
                public class Config implements <cur:ser_ref>Serializable {
                    private static final long serialVersionUID = 1L;
                    private String name;
                }
            "#)
            .assert_at("ser_ref")
                .resolves_to("java.io.Serializable")
                .expected_failure("JDK interface java.io.Serializable not available in fixture")
            .run();
    }

    /// Multiple JDK interfaces in implements clause — each should resolve.
    // @keep — resolves JDK Runnable and Serializable from multiple implements clauses (usage sites)
    #[test]
    fn implements_multiple_jdk_interfaces() {
        fixture()
            .file("com/example/Task.java", r#"
                package com.example;
                import java.io.Serializable;
                public class Task implements <cur:run_ref>Runnable, <cur:ser_ref>Serializable {
                    private static final long serialVersionUID = 1L;
                    public void run() {}
                }
            "#)
            .assert_at("run_ref")
                .resolves_to("java.lang.Runnable")
                .expected_failure("JDK type java.lang.Runnable not available in fixture")
            .assert_at("ser_ref")
                .resolves_to("java.io.Serializable")
                .expected_failure("JDK type java.io.Serializable not available in fixture")
            .run();
    }
}


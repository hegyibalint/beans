use beans::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §9.1 — Interface Declarations
mod jls_9_1_interface_declarations {
    use super::*;

    // @evolve — add dot-completion test: class implementing Service, cursor after svc., expect start() and stop()
    #[test]
    fn basic_interface_children() {
        fixture()
            .file(
                "com/example/Service.java",
                r#"
                package com.example;
                public interface <cur:svc>Service {
                    void start();
                    void stop();
                }
            "#,
            )
            .assert_at("svc")
            .kind(SymbolKind::Interface)
            .fqn("com.example.Service")
            .children_include(&["start", "stop"])
            .children_count(2)
            .run();
    }

    #[test]
    fn dot_completion_on_service() {
        fixture()
            .file(
                "com/example/Service.java",
                r#"
                package com.example;
                public interface Service {
                    void start();
                    void stop();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(Service svc) {
                        svc.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("start", SymbolKind::Method));
                assert!(items.has("stop", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: class implementing Channel, cursor after channel., expect isOpen() and inherited close()
    #[test]
    fn interface_extends_single() {
        fixture()
            .file(
                "com/example/Closeable.java",
                r#"
                package com.example;
                public interface Closeable {
                    void close();
                }
            "#,
            )
            .file(
                "com/example/Channel.java",
                r#"
                package com.example;
                public interface <cur:channel>Channel extends Closeable {
                    boolean <cur:isopen>isOpen();
                }
            "#,
            )
            .assert_at("channel")
            .kind(SymbolKind::Interface)
            .fqn("com.example.Channel")
            .assert_at("isopen")
            .kind(SymbolKind::Method)
            .fqn("com.example.Channel.isOpen")
            .signature_return("boolean")
            .run();
    }

    #[test]
    fn dot_completion_on_channel() {
        fixture()
            .file(
                "com/example/Closeable.java",
                r#"
                package com.example;
                public interface Closeable {
                    void close();
                }
            "#,
            )
            .file(
                "com/example/Channel.java",
                r#"
                package com.example;
                public interface Channel extends Closeable {
                    boolean isOpen();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(Channel channel) {
                        channel.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("isOpen", SymbolKind::Method));
                assert!(items.has("close", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: class implementing ReadWriteChannel, cursor after channel., expect read(), write(), and flush()
    #[test]
    fn interface_extends_multiple() {
        fixture()
            .file(
                "com/example/Readable.java",
                r#"
                package com.example;
                public interface Readable { String read(); }
            "#,
            )
            .file(
                "com/example/Writable.java",
                r#"
                package com.example;
                public interface Writable { void write(String data); }
            "#,
            )
            .file(
                "com/example/ReadWriteChannel.java",
                r#"
                package com.example;
                public interface <cur:rw>ReadWriteChannel extends Readable, Writable {
                    void <cur:flush>flush();
                }
            "#,
            )
            .assert_at("rw")
            .kind(SymbolKind::Interface)
            .fqn("com.example.ReadWriteChannel")
            .assert_at("flush")
            .kind(SymbolKind::Method)
            .fqn("com.example.ReadWriteChannel.flush")
            .signature_return("void")
            .run();
    }

    #[test]
    fn dot_completion_on_read_write_channel() {
        fixture()
            .file(
                "com/example/Readable.java",
                r#"
                package com.example;
                public interface Readable { String read(); }
            "#,
            )
            .file(
                "com/example/Writable.java",
                r#"
                package com.example;
                public interface Writable { void write(String data); }
            "#,
            )
            .file(
                "com/example/ReadWriteChannel.java",
                r#"
                package com.example;
                public interface ReadWriteChannel extends Readable, Writable {
                    void flush();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void process(ReadWriteChannel channel) {
                        channel.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("read", SymbolKind::Method));
                assert!(items.has("write", SymbolKind::Method));
                assert!(items.has("flush", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: AreaCalc.java with Shape parameter, cursor after shape., expect area(); multi-file setup with Circle/Rectangle is ideal
    #[test]
    fn sealed_interface() {
        fixture()
            .file(
                "com/example/Shape.java",
                r#"
                package com.example;
                public sealed interface <cur:shape>Shape permits Circle, Rectangle {
                    double area();
                }
            "#,
            )
            .file(
                "com/example/Circle.java",
                r#"
                package com.example;
                public record Circle(double radius) implements Shape {
                    public double area() { return Math.PI * radius * radius; }
                }
            "#,
            )
            .file(
                "com/example/Rectangle.java",
                r#"
                package com.example;
                public record Rectangle(double width, double height) implements Shape {
                    public double area() { return width * height; }
                }
            "#,
            )
            .assert_at("shape")
            .kind(SymbolKind::Interface)
            .fqn("com.example.Shape")
            .modifiers(vec![Modifier::Sealed])
            .run();
    }

    #[test]
    fn dot_completion_on_shape() {
        fixture()
            .file(
                "com/example/Shape.java",
                r#"
                package com.example;
                public sealed interface Shape permits Circle, Rectangle {
                    double area();
                }
            "#,
            )
            .file(
                "com/example/Circle.java",
                r#"
                package com.example;
                public record Circle(double radius) implements Shape {
                    public double area() { return Math.PI * radius * radius; }
                }
            "#,
            )
            .file(
                "com/example/Rectangle.java",
                r#"
                package com.example;
                public record Rectangle(double width, double height) implements Shape {
                    public double area() { return width * height; }
                }
            "#,
            )
            .file(
                "com/example/AreaCalc.java",
                r#"
                package com.example;
                public class AreaCalc {
                    public double calculate(Shape shape) {
                        return shape.<cur>area();
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("area", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // §9.1.4 — non-sealed interface in sealed hierarchy
    #[test]
    fn dot_completion_on_non_sealed_interface() {
        fixture()
            .file(
                "com/example/Shape.java",
                r#"
                package com.example;
                public sealed interface Shape permits Circle, Polygon {}
            "#,
            )
            .file(
                "com/example/Polygon.java",
                r#"
                package com.example;
                public non-sealed interface Polygon extends Shape {
                    int sides();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(Polygon p) {
                        p.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("sides", SymbolKind::Method));
            })
            .expected_failure("non-sealed interface member completion not yet implemented")
            .run();
    }
}

// §9.2 — Interface Members
mod jls_9_2_interface_members {
    use super::*;

    // @evolve — add dot-completion test: App.java with ClickListener instance, cursor after listener., expect onClick() and inherited onEvent()
    #[test]
    fn own_method_declaration() {
        fixture()
            .file(
                "com/example/EventListener.java",
                r#"
                package com.example;
                public interface EventListener {
                    void onEvent(String event);
                }
            "#,
            )
            .file(
                "com/example/ClickListener.java",
                r#"
                package com.example;
                public interface <cur:click>ClickListener extends EventListener {
                    void <cur:onclick>onClick(int x, int y);
                }
            "#,
            )
            .assert_at("click")
            .kind(SymbolKind::Interface)
            .fqn("com.example.ClickListener")
            .assert_at("onclick")
            .kind(SymbolKind::Method)
            .fqn("com.example.ClickListener.onClick")
            .signature_params(&[("x", "int"), ("y", "int")])
            .run();
    }

    #[test]
    fn dot_completion_on_click_listener() {
        fixture()
            .file(
                "com/example/EventListener.java",
                r#"
                package com.example;
                public interface EventListener {
                    void onEvent(String event);
                }
            "#,
            )
            .file(
                "com/example/ClickListener.java",
                r#"
                package com.example;
                public interface ClickListener extends EventListener {
                    void onClick(int x, int y);
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void register(ClickListener listener) {
                        listener.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("onClick", SymbolKind::Method));
                assert!(items.has("onEvent", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: App.java with UserProfile instance, cursor after profile., expect getDisplayName(), getName(), getEmail(), getId() (all inherited)
    #[test]
    fn diamond_interface_inheritance() {
        fixture()
            .file(
                "com/example/HasId.java",
                r#"
                package com.example;
                public interface HasId { long getId(); }
            "#,
            )
            .file(
                "com/example/HasName.java",
                r#"
                package com.example;
                public interface HasName extends HasId { String getName(); }
            "#,
            )
            .file(
                "com/example/HasEmail.java",
                r#"
                package com.example;
                public interface HasEmail extends HasId { String getEmail(); }
            "#,
            )
            .file(
                "com/example/UserProfile.java",
                r#"
                package com.example;
                public interface <cur:profile>UserProfile extends HasName, HasEmail {
                    String <cur:display>getDisplayName();
                }
            "#,
            )
            .assert_at("profile")
            .kind(SymbolKind::Interface)
            .fqn("com.example.UserProfile")
            .assert_at("display")
            .kind(SymbolKind::Method)
            .fqn("com.example.UserProfile.getDisplayName")
            .signature_return("String")
            .run();
    }

    #[test]
    fn dot_completion_on_user_profile() {
        fixture()
            .file(
                "com/example/HasId.java",
                r#"
                package com.example;
                public interface HasId { long getId(); }
            "#,
            )
            .file(
                "com/example/HasName.java",
                r#"
                package com.example;
                public interface HasName extends HasId { String getName(); }
            "#,
            )
            .file(
                "com/example/HasEmail.java",
                r#"
                package com.example;
                public interface HasEmail extends HasId { String getEmail(); }
            "#,
            )
            .file(
                "com/example/UserProfile.java",
                r#"
                package com.example;
                public interface UserProfile extends HasName, HasEmail {
                    String getDisplayName();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void display(UserProfile profile) {
                        profile.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("getDisplayName", SymbolKind::Method));
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getEmail", SymbolKind::Method));
                assert!(items.has("getId", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // §9.2 — Interface with empty body inherits all members from superinterfaces
    #[test]
    fn dot_completion_empty_body_interface_shows_inherited_members() {
        fixture()
            .file(
                "com/example/Serializable.java",
                r#"
                package com.example;
                public interface Serializable {}
            "#,
            )
            .file(
                "com/example/Persistable.java",
                r#"
                package com.example;
                public interface Persistable {
                    void save();
                    void delete();
                }
            "#,
            )
            .file(
                "com/example/Entity.java",
                r#"
                package com.example;
                public interface Entity extends Serializable, Persistable {}
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(Entity e) {
                        e.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("save", SymbolKind::Method));
                assert!(items.has("delete", SymbolKind::Method));
            })
            .expected_failure(
                "empty-body interface inherited member completion not yet implemented",
            )
            .run();
    }
}

// §9.3 — Field (Constant) Declarations
mod jls_9_3_interface_constants {
    use super::*;

    #[test]
    fn dot_completion_on_instance_shows_constants() {
        fixture()
            .file(
                "com/example/HttpStatus.java",
                r#"
                package com.example;
                public interface HttpStatus {
                    int OK = 200;
                    int NOT_FOUND = 404;
                    String getMessage();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void handle(HttpStatus status) {
                        status.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("getMessage", SymbolKind::Method));
                assert!(items.has("OK", SymbolKind::Field));
                assert!(items.has("NOT_FOUND", SymbolKind::Field));
            })
            .expected_failure("interface constants not yet included in completion")
            .run();
    }

    #[test]
    fn dot_completion_on_interface_type_shows_static_constants() {
        fixture()
            .file(
                "com/example/Colors.java",
                r#"
                package com.example;
                public interface Colors {
                    int RED = 1;
                    int GREEN = 2;
                    int BLUE = 4;
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void paint() {
                        Colors.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("RED", SymbolKind::Field));
                assert!(items.has("GREEN", SymbolKind::Field));
                assert!(items.has("BLUE", SymbolKind::Field));
            })
            .expected_failure("interface constants not yet included in static completion")
            .run();
    }

    #[test]
    fn dot_completion_shows_inherited_constants() {
        fixture()
            .file(
                "com/example/Base.java",
                r#"
                package com.example;
                public interface Base {
                    int VERSION = 1;
                }
            "#,
            )
            .file(
                "com/example/Extended.java",
                r#"
                package com.example;
                public interface Extended extends Base {
                    int REVISION = 2;
                    String name();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(Extended ext) {
                        ext.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("name", SymbolKind::Method));
                assert!(items.has("REVISION", SymbolKind::Field));
                assert!(items.has("VERSION", SymbolKind::Field));
            })
            .expected_failure("inherited interface constants not yet included in completion")
            .run();
    }

    #[test]
    fn dot_completion_with_ambiguous_inherited_constants() {
        fixture()
            .file(
                "com/example/A.java",
                r#"
                package com.example;
                public interface A { int X = 1; }
            "#,
            )
            .file(
                "com/example/B.java",
                r#"
                package com.example;
                public interface B { int X = 2; }
            "#,
            )
            .file(
                "com/example/C.java",
                r#"
                package com.example;
                public interface C extends A, B {
                    void doWork();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(C c) {
                        c.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("doWork", SymbolKind::Method));
                assert!(items.has("X", SymbolKind::Field));
            })
            .expected_failure(
                "ambiguous inherited interface constants not yet handled in completion",
            )
            .run();
    }

    // §9.3 — Subinterface redefines same-name constant (hiding, not ambiguity)
    #[test]
    fn dot_completion_constant_hiding_shows_most_specific() {
        fixture()
            .file(
                "com/example/Parent.java",
                r#"
                package com.example;
                public interface Parent {
                    int LIMIT = 100;
                    String label();
                }
            "#,
            )
            .file(
                "com/example/Child.java",
                r#"
                package com.example;
                public interface Child extends Parent {
                    int LIMIT = 200;
                    void process();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(Child c) {
                        c.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("label", SymbolKind::Method));
                assert!(items.has("process", SymbolKind::Method));
                assert!(items.has("LIMIT", SymbolKind::Field));
                assert_eq!(items.count(SymbolKind::Field), 1);
            })
            .expected_failure("interface constant hiding not yet implemented")
            .run();
    }

    // §9.3 + §9.4 — Constants and methods from interface accessible on implementing class
    #[test]
    fn dot_completion_implementing_class_exposes_interface_constants_and_methods() {
        fixture()
            .file(
                "com/example/Config.java",
                r#"
                package com.example;
                public interface Config {
                    String DEFAULT_HOST = "localhost";
                    int DEFAULT_PORT = 8080;
                    String getHost();
                    int getPort();
                }
            "#,
            )
            .file(
                "com/example/AppConfig.java",
                r#"
                package com.example;
                public class AppConfig implements Config {
                    public String getHost() { return DEFAULT_HOST; }
                    public int getPort() { return DEFAULT_PORT; }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(AppConfig cfg) {
                        cfg.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("getHost", SymbolKind::Method));
                assert!(items.has("getPort", SymbolKind::Method));
                assert!(items.has("DEFAULT_HOST", SymbolKind::Field));
                assert!(items.has("DEFAULT_PORT", SymbolKind::Field));
            })
            .expected_failure(
                "interface constants not yet inherited by implementing class in completion",
            )
            .run();
    }
}

// §9.4 — Method Declarations
mod jls_9_4_method_declarations {
    use super::*;

    // @evolve — add dot-completion test: class implementing Logger, cursor after logger., expect info() and warn() but not private log()
    #[test]
    fn private_method_java9() {
        fixture()
            .file(
                "com/example/Logger.java",
                r#"
                package com.example;
                public interface Logger {
                    default void <cur:info>info(String msg) {
                        log("INFO", msg);
                    }
                    default void warn(String msg) {
                        log("WARN", msg);
                    }
                    private void <cur:log>log(String level, String msg) {
                        System.out.println("[" + level + "] " + msg);
                    }
                }
            "#,
            )
            .assert_at("info")
            .kind(SymbolKind::Method)
            .fqn("com.example.Logger.info")
            .signature_return("void")
            .signature_params(&[("msg", "String")])
            .assert_at("log")
            .kind(SymbolKind::Method)
            .fqn("com.example.Logger.log")
            .modifiers(vec![Modifier::Private])
            .run();
    }

    #[test]
    fn dot_completion_on_logger() {
        fixture()
            .file(
                "com/example/Logger.java",
                r#"
                package com.example;
                public interface Logger {
                    default void info(String msg) {
                        log("INFO", msg);
                    }
                    default void warn(String msg) {
                        log("WARN", msg);
                    }
                    private void log(String level, String msg) {
                        System.out.println("[" + level + "] " + msg);
                    }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(Logger logger) {
                        logger.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("info", SymbolKind::Method));
                assert!(items.has("warn", SymbolKind::Method));
                assert!(!items.has("log", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: App.java with Collection<E> instance, cursor after coll., expect size(), add(), isEmpty() but not static emptyCollection()
    #[test]
    fn multiple_method_kinds_children() {
        fixture()
            .file(
                "com/example/Collection.java",
                r#"
                package com.example;
                public interface <cur:coll>Collection<E> {
                    int size();
                    boolean add(E element);
                    default boolean isEmpty() { return size() == 0; }
                    static <E> Collection<E> emptyCollection() { return null; }
                }
            "#,
            )
            .assert_at("coll")
            .kind(SymbolKind::Interface)
            .fqn("com.example.Collection")
            .children_include(&["size", "add", "isEmpty", "emptyCollection"])
            .children_count(4)
            .run();
    }

    #[test]
    fn dot_completion_on_collection() {
        fixture()
            .file(
                "com/example/Collection.java",
                r#"
                package com.example;
                public interface Collection<E> {
                    int size();
                    boolean add(E element);
                    default boolean isEmpty() { return size() == 0; }
                    static <E> Collection<E> emptyCollection() { return null; }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void process(Collection<String> coll) {
                        coll.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("size", SymbolKind::Method));
                assert!(items.has("add", SymbolKind::Method));
                assert!(items.has("isEmpty", SymbolKind::Method));
                assert!(!items.has("emptyCollection", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // §9.4 — Static method access via interface type name
    #[test]
    fn dot_completion_static_method_via_type_name() {
        fixture()
            .file(
                "com/example/Collection.java",
                r#"
                package com.example;
                public interface Collection<E> {
                    int size();
                    default boolean isEmpty() { return size() == 0; }
                    static <E> Collection<E> emptyCollection() { return null; }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run() {
                        Collection.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("emptyCollection", SymbolKind::Method));
                assert!(!items.has("size", SymbolKind::Method));
                assert!(!items.has("isEmpty", SymbolKind::Method));
            })
            .expected_failure(
                "static method completion via interface type name not yet implemented",
            )
            .run();
    }

    // §9.4.1 — Default method overriding in subinterface
    #[test]
    fn dot_completion_default_method_overridden_in_subinterface() {
        fixture()
            .file(
                "com/example/Base.java",
                r#"
                package com.example;
                public interface Base {
                    default String describe() { return "base"; }
                }
            "#,
            )
            .file(
                "com/example/Child.java",
                r#"
                package com.example;
                public interface Child extends Base {
                    default String describe() { return "child"; }
                    void extra();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(Child c) {
                        c.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("describe", SymbolKind::Method));
                assert!(items.has("extra", SymbolKind::Method));
                assert_eq!(items.count(SymbolKind::Method), 2);
            })
            .expected_failure("default method override deduplication not yet implemented")
            .run();
    }

    // §9.4.2 — Overloaded methods in interface completion
    #[test]
    fn dot_completion_overloaded_methods_in_interface() {
        fixture()
            .file(
                "com/example/PointInterface.java",
                r#"
                package com.example;
                public interface PointInterface {
                    void move(int dx, int dy);
                }
            "#,
            )
            .file(
                "com/example/RealPointInterface.java",
                r#"
                package com.example;
                public interface RealPointInterface extends PointInterface {
                    void move(float dx, float dy);
                    void move(double dx, double dy);
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(RealPointInterface p) {
                        p.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("move", SymbolKind::Method));
            })
            .expected_failure("interface method overload completion not yet implemented")
            .run();
    }

    // §9.4 — Class implementing interface inherits default methods
    #[test]
    fn dot_completion_implementing_class_inherits_default_methods() {
        fixture()
            .file(
                "com/example/Greeting.java",
                r#"
                package com.example;
                public interface Greeting {
                    default String hello() { return "Hi"; }
                    void greet();
                }
            "#,
            )
            .file(
                "com/example/FriendlyBot.java",
                r#"
                package com.example;
                public class FriendlyBot implements Greeting {
                    public void greet() {}
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(FriendlyBot bot) {
                        bot.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("greet", SymbolKind::Method));
                assert!(items.has("hello", SymbolKind::Method));
            })
            .expected_failure("class inheriting interface default methods not yet in completion")
            .run();
    }

    // §9.4.1 — Abstract method re-declared in subinterface (not duplicated in completion)
    #[test]
    fn dot_completion_abstract_redeclared_method_not_duplicated() {
        fixture()
            .file(
                "com/example/Base.java",
                r#"
                package com.example;
                public interface Base {
                    void execute();
                }
            "#,
            )
            .file(
                "com/example/Worker.java",
                r#"
                package com.example;
                public interface Worker extends Base {
                    void execute();
                    void prepare();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(Worker w) {
                        w.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("execute", SymbolKind::Method));
                assert!(items.has("prepare", SymbolKind::Method));
                assert_eq!(items.count(SymbolKind::Method), 2);
            })
            .expected_failure("abstract method re-declaration deduplication not yet implemented")
            .run();
    }

    // §9.4.1.3 — Conflicting defaults from two superinterfaces resolved by Bottom's override
    #[test]
    fn dot_completion_conflicting_defaults_resolved_by_override() {
        fixture()
            .file(
                "com/example/Left.java",
                r#"
                package com.example;
                public interface Left {
                    default String name() { return "left"; }
                }
            "#,
            )
            .file(
                "com/example/Right.java",
                r#"
                package com.example;
                public interface Right {
                    default String name() { return "right"; }
                }
            "#,
            )
            .file(
                "com/example/Bottom.java",
                r#"
                package com.example;
                public interface Bottom extends Left, Right {
                    default String name() { return "bottom"; }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(Bottom b) {
                        b.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("name", SymbolKind::Method));
                assert_eq!(items.count(SymbolKind::Method), 1);
            })
            .expected_failure("conflicting default method deduplication not yet implemented")
            .run();
    }

    // §9.4 — Mixed default and abstract in 3-level hierarchy
    #[test]
    fn dot_completion_mixed_default_abstract_three_levels() {
        fixture()
            .file(
                "com/example/Animal.java",
                r#"
                package com.example;
                public interface Animal {
                    String sound();
                    default String describe() { return "animal"; }
                }
            "#,
            )
            .file(
                "com/example/Pet.java",
                r#"
                package com.example;
                public interface Pet extends Animal {
                    default String sound() { return "?"; }
                    String owner();
                }
            "#,
            )
            .file(
                "com/example/DomesticCat.java",
                r#"
                package com.example;
                public interface DomesticCat extends Pet {
                    void purr();
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(DomesticCat cat) {
                        cat.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("purr", SymbolKind::Method));
                assert!(items.has("sound", SymbolKind::Method));
                assert!(items.has("describe", SymbolKind::Method));
                assert!(items.has("owner", SymbolKind::Method));
                assert_eq!(items.count(SymbolKind::Method), 4);
            })
            .expected_failure("multi-level interface method completion not yet implemented")
            .run();
    }

    // §9.4 — private static method excluded from type-level completion
    #[test]
    fn dot_completion_private_static_excluded_from_type_level() {
        fixture()
            .file("com/example/Validator.java", r#"
                package com.example;
                public interface Validator {
                    boolean validate(String input);
                    private static boolean isBlank(String s) { return s == null || s.trim().isEmpty(); }
                    static Validator nonBlank() { return s -> !isBlank(s); }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void setup() {
                        Validator.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("nonBlank", SymbolKind::Method));
                assert!(!items.has("isBlank", SymbolKind::Method));
                assert!(!items.has("validate", SymbolKind::Method));
            })
            .expected_failure("private static method exclusion from type-level completion not yet implemented")
            .run();
    }
}

// §9.7 — Annotations
mod jls_9_7_annotations {
    use super::*;

    // @evolve — add dot-completion test: App.java with LegacyService instance, cursor after svc., expect process() (deprecated methods still appear in completions)
    #[test]
    fn deprecated_annotation_hover() {
        fixture()
            .file(
                "com/example/LegacyService.java",
                r#"
                package com.example;
                @Deprecated
                public class <cur:legacy>LegacyService {
                    public void process() {}
                }
            "#,
            )
            .assert_at("legacy")
            .kind(SymbolKind::Class)
            .fqn("com.example.LegacyService")
            .hover_contains("Deprecated")
            .expected_failure("deprecated annotation not reflected in hover text")
            .run();
    }

    #[test]
    fn dot_completion_on_legacy_service() {
        fixture()
            .file(
                "com/example/LegacyService.java",
                r#"
                package com.example;
                @Deprecated
                public class LegacyService {
                    public void process() {}
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(LegacyService svc) {
                        svc.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("process", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: App.java with OrderService instance, cursor after service., expect findOrder() (annotation should not suppress method in completions)
    #[test]
    fn method_with_custom_annotation() {
        fixture()
            .file(
                "com/example/Transactional.java",
                r#"
                package com.example;
                public @interface Transactional {
                    boolean readOnly() default false;
                }
            "#,
            )
            .file(
                "com/example/OrderService.java",
                r#"
                package com.example;
                public class OrderService {
                    @Transactional(readOnly = true)
                    public Object <cur:find>findOrder(long id) {
                        return null;
                    }
                }
            "#,
            )
            .assert_at("find")
            .kind(SymbolKind::Method)
            .fqn("com.example.OrderService.findOrder")
            .signature_return("Object")
            .run();
    }

    #[test]
    fn dot_completion_on_order_service() {
        fixture()
            .file(
                "com/example/Transactional.java",
                r#"
                package com.example;
                public @interface Transactional {
                    boolean readOnly() default false;
                }
            "#,
            )
            .file(
                "com/example/OrderService.java",
                r#"
                package com.example;
                public class OrderService {
                    @Transactional(readOnly = true)
                    public Object findOrder(long id) {
                        return null;
                    }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(OrderService service) {
                        service.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("findOrder", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §9.5 — Member Type Declarations
mod jls_9_5_member_types {
    use super::*;

    #[test]
    fn dot_completion_on_nested_interface_instance() {
        fixture()
            .file(
                "com/example/Map.java",
                r#"
                package com.example;
                public interface Map<K, V> {
                    V get(K key);
                    interface Entry<K, V> {
                        K getKey();
                        V getValue();
                    }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use(Map.Entry<String, String> entry) {
                        entry.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("getKey", SymbolKind::Method));
                assert!(items.has("getValue", SymbolKind::Method));
                assert!(!items.has("get", SymbolKind::Method));
            })
            .expected_failure("nested interface member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_type_level_access_to_nested_interface() {
        fixture()
            .file(
                "com/example/Map.java",
                r#"
                package com.example;
                public interface Map<K, V> {
                    V get(K key);
                    interface Entry<K, V> {
                        K getKey();
                        V getValue();
                    }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void use() {
                        Map.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("Entry", SymbolKind::Interface));
            })
            .expected_failure("type-level access to nested interface not yet implemented")
            .run();
    }

    // §9.5 — Nested enum inside an interface (member type)
    #[test]
    fn dot_completion_type_level_access_to_nested_enum() {
        fixture()
            .file(
                "com/example/Logger.java",
                r#"
                package com.example;
                public interface Logger {
                    void log(Level level, String msg);
                    enum Level { DEBUG, INFO, WARN, ERROR }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void setup() {
                        Logger.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("Level", SymbolKind::Enum));
                assert!(!items.has("log", SymbolKind::Method));
            })
            .expected_failure("nested enum type-level access not yet implemented")
            .run();
    }
}

// §9.8 — Functional Interfaces
mod jls_9_8_functional_interfaces {
    use super::*;

    // @evolve — add dot-completion test: App.java with Function<String,String> instance, cursor after fn., expect apply() and compose()
    #[test]
    fn functional_interface_with_default_methods() {
        fixture()
            .file(
                "com/example/Function.java",
                r#"
                package com.example;
                @FunctionalInterface
                public interface <cur:func>Function<T, R> {
                    R apply(T input);
                    default <V> Function<V, R> compose(Function<V, T> before) {
                        return v -> apply(before.apply(v));
                    }
                }
            "#,
            )
            .assert_at("func")
            .kind(SymbolKind::Interface)
            .fqn("com.example.Function")
            .children_include(&["apply", "compose"])
            .run();
    }

    #[test]
    fn dot_completion_on_function() {
        fixture()
            .file(
                "com/example/Function.java",
                r#"
                package com.example;
                @FunctionalInterface
                public interface Function<T, R> {
                    R apply(T input);
                    default <V> Function<V, R> compose(Function<V, T> before) {
                        return v -> apply(before.apply(v));
                    }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void run(Function<String, String> fn) {
                        fn.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("apply", SymbolKind::Method));
                assert!(items.has("compose", SymbolKind::Method));
            })
            .expected_failure("interface member completion not yet implemented")
            .run();
    }

    // §9.8 + §9.2 — equals(Object) declared in functional interface still appears in completion
    #[test]
    fn dot_completion_on_comparator_includes_equals() {
        fixture()
            .file(
                "com/example/Comparator.java",
                r#"
                package com.example;
                @FunctionalInterface
                public interface Comparator<T> {
                    int compare(T o1, T o2);
                    boolean equals(Object obj);
                    default Comparator<T> reversed() { return null; }
                }
            "#,
            )
            .file(
                "com/example/App.java",
                r#"
                package com.example;
                public class App {
                    public void sort(Comparator<String> cmp) {
                        cmp.<cur>
                    }
                }
            "#,
            )
            .complete_default(|items| {
                assert!(items.has("compare", SymbolKind::Method));
                assert!(items.has("reversed", SymbolKind::Method));
                assert!(items.has("equals", SymbolKind::Method));
            })
            .expected_failure("functional interface member completion not yet implemented")
            .run();
    }
}

use beans_core::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §8.1 — Class Declarations
mod jls_8_1_class_declarations {
    use super::*;

    // @evolve — add dot-completion test: App.java takes Dog parameter, cursor after dog., expect getName() and getAge() but not name/age (private)
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

    #[test]
    fn dot_completion_on_instance() {
        fixture()
            .file("com/example/Dog.java", r#"
                package com.example;
                public class Dog {
                    private String name;
                    private int age;
                    public String getName() { return name; }
                    public int getAge() { return age; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Dog dog) {
                        dog.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getAge", SymbolKind::Method));
                assert!(!items.has("name", SymbolKind::Field));
                assert!(!items.has("age", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn this_dot_completion_inside_own_class() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    private String name;
                    public int port;
                    public void start() {}
                    private void init() {}
                    public void run() {
                        this.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("name", SymbolKind::Field));
                assert!(items.has("port", SymbolKind::Field));
                assert!(items.has("start", SymbolKind::Method));
                assert!(items.has("init", SymbolKind::Method));
            })
            .expected_failure("this-dot completion not yet implemented")
            .run();
    }
}

// §8.1.1 — Class Modifiers
mod jls_8_1_1_class_modifiers {
    use super::*;

    // @evolve — add dot-completion test: concrete subclass of Shape, cursor after shape., expect describe() (non-abstract inherited method) and area()
    #[test]
    fn abstract_class() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public abstract class <cur:cls>Shape {
                    public abstract double <cur:area>area();
                    public String describe() { return "a shape"; }
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Shape")
                .modifiers(vec![Modifier::Public, Modifier::Abstract])
            .assert_at("area")
                .kind(SymbolKind::Method)
                .modifiers(vec![Modifier::Public, Modifier::Abstract])
                .signature_return("double")
            .run();
    }

    #[test]
    fn dot_completion_on_shape() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public abstract class Shape {
                    public abstract double area();
                    public String describe() { return "a shape"; }
                }
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public class Circle extends Shape {
                    private double radius;
                    public Circle(double radius) { this.radius = radius; }
                    public double area() { return Math.PI * radius * radius; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Shape shape) {
                        shape.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("area", SymbolKind::Method));
                assert!(items.has("describe", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.1.2 — Generic Classes and Type Parameters
mod jls_8_1_2_generic_classes {
    use super::*;

    // @evolve — add dot-completion test: App.java with Box<String> instance, cursor after box., expect getValue() and setValue()
    #[test]
    fn simple_generic_class() {
        fixture()
            .file("com/example/Box.java", r#"
                package com.example;
                public class <cur:cls>Box<T> {
                    private T <cur:value>value;
                    public T getValue() { return value; }
                    public void setValue(T value) { this.value = value; }
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Box")
                .name("Box")
            .assert_at("value")
                .kind(SymbolKind::Field)
                .fqn("com.example.Box.value")
            .run();
    }

    #[test]
    fn dot_completion_on_box() {
        fixture()
            .file("com/example/Box.java", r#"
                package com.example;
                public class Box<T> {
                    private T value;
                    public T getValue() { return value; }
                    public void setValue(T value) { this.value = value; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Box<String> box) {
                        box.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getValue", SymbolKind::Method));
                assert!(items.has("setValue", SymbolKind::Method));
                assert!(!items.has("value", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: App.java with Pair<String,Integer> instance, cursor after pair., expect getKey() and getValue()
    #[test]
    fn multi_type_parameter_class() {
        fixture()
            .file("com/example/Pair.java", r#"
                package com.example;
                public class <cur:cls>Pair<K, V> {
                    private K <cur:key>key;
                    private V <cur:val>value;
                    public Pair(K key, V value) {
                        this.key = key;
                        this.value = value;
                    }
                    public K getKey() { return key; }
                    public V getValue() { return value; }
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Pair")
            .assert_at("key")
                .kind(SymbolKind::Field)
                .fqn("com.example.Pair.key")
            .run();
    }

    #[test]
    fn dot_completion_on_pair() {
        fixture()
            .file("com/example/Pair.java", r#"
                package com.example;
                public class Pair<K, V> {
                    private K key;
                    private V value;
                    public Pair(K key, V value) {
                        this.key = key;
                        this.value = value;
                    }
                    public K getKey() { return key; }
                    public V getValue() { return value; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Pair<String, Integer> pair) {
                        pair.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getKey", SymbolKind::Method));
                assert!(items.has("getValue", SymbolKind::Method));
                assert!(!items.has("key", SymbolKind::Field));
                assert!(!items.has("value", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.1.3 — Inner Classes and Enclosing Instances
mod jls_8_1_3_inner_classes {
    use super::*;

    // @evolve — add dot-completion test: App.java creates Outer.Inner instance, cursor after inner., expect getOuterX()
    #[test]
    fn non_static_inner_class() {
        fixture()
            .file("com/example/Outer.java", r#"
                package com.example;
                public class <cur:outer>Outer {
                    private int x;
                    public class <cur:inner>Inner {
                        public int getOuterX() { return x; }
                    }
                }
            "#)
            .assert_at("outer")
                .kind(SymbolKind::Class)
                .fqn("com.example.Outer")
                .children_include(&["x", "Inner"])
            .assert_at("inner")
                .kind(SymbolKind::Class)
                .fqn("com.example.Outer.Inner")
                .parent_fqn("com.example.Outer")
            .run();
    }

    #[test]
    fn dot_completion_on_inner() {
        fixture()
            .file("com/example/Outer.java", r#"
                package com.example;
                public class Outer {
                    private int x;
                    public class Inner {
                        public int getOuterX() { return x; }
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Outer.Inner inner) {
                        inner.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getOuterX", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn outer_this_dot_completion_inside_inner_class() {
        fixture()
            .file("com/example/Outer.java", r#"
                package com.example;
                public class Outer {
                    private String name;
                    public void outerMethod() {}

                    public class Inner {
                        public void innerMethod() {}
                        public void run() {
                            Outer.this.<cur>
                        }
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("name", SymbolKind::Field));
                assert!(items.has("outerMethod", SymbolKind::Method));
                assert!(!items.has("innerMethod", SymbolKind::Method));
            })
            .expected_failure("Outer.this-dot completion not yet implemented")
            .run();
    }
}

// §8.1.2 (continued) — Bounded type parameter
mod jls_8_1_2_bounded_generics {
    use super::*;

    #[test]
    fn dot_completion_on_repository_with_bounded_type_param() {
        fixture()
            .file("com/example/Entity.java", r#"
                package com.example;
                public class Entity {
                    public long getId() { return 0; }
                }
            "#)
            .file("com/example/User.java", r#"
                package com.example;
                public class User extends Entity {}
            "#)
            .file("com/example/Repository.java", r#"
                package com.example;
                public class Repository<T extends Entity> {
                    public T find() { return null; }
                    public void save(T item) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Repository<User> repo) {
                        repo.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("find", SymbolKind::Method));
                assert!(items.has("save", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.1.4 — Superclasses and Subclasses
mod jls_8_1_4_superclasses {
    use super::*;

    // @keep — cross-file resolves_to for superclass reference in extends clause
    #[test]
    fn simple_extends() {
        fixture()
            .file("com/example/Animal.java", r#"
                package com.example;
                public class Animal {
                    public String name;
                    public void speak() {}
                }
            "#)
            .file("com/example/Dog.java", r#"
                package com.example;
                public class <cur:cls>Dog extends <cur:super_ref>Animal {
                    public void fetch() {}
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Dog")
            .assert_at("super_ref")
                .resolves_to("com.example.Animal")
            .run();
    }

    #[test]
    fn dot_completion_on_dog_inherits_animal() {
        fixture()
            .file("com/example/Animal.java", r#"
                package com.example;
                public class Animal {
                    public String name;
                    private int secret;
                    public void speak() {}
                }
            "#)
            .file("com/example/Dog.java", r#"
                package com.example;
                public class Dog extends Animal {
                    public void fetch() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Dog dog) {
                        dog.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("fetch", SymbolKind::Method));
                assert!(items.has("speak", SymbolKind::Method));
                assert!(items.has("name", SymbolKind::Field));
                assert!(!items.has("secret", SymbolKind::Field));
            })
            .expected_failure("inherited member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_sportscar_transitive_inheritance() {
        fixture()
            .file("com/example/Vehicle.java", r#"
                package com.example;
                public class Vehicle {
                    public void start() {}
                }
            "#)
            .file("com/example/Car.java", r#"
                package com.example;
                public class Car extends Vehicle {
                    public void drive() {}
                }
            "#)
            .file("com/example/SportsCar.java", r#"
                package com.example;
                public class SportsCar extends Car {
                    public void boost() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(SportsCar sportsCar) {
                        sportsCar.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("boost", SymbolKind::Method));
                assert!(items.has("drive", SymbolKind::Method));
                assert!(items.has("start", SymbolKind::Method));
            })
            .expected_failure("transitive inherited member completion not yet implemented")
            .run();
    }

    // @keep — cross-file resolves_to for parent class in multi-level inheritance chain
    #[test]
    fn multi_level_inheritance() {
        fixture()
            .file("com/example/Vehicle.java", r#"
                package com.example;
                public class Vehicle {
                    public int speed;
                }
            "#)
            .file("com/example/Car.java", r#"
                package com.example;
                public class Car extends Vehicle {
                    public int doors;
                }
            "#)
            .file("com/example/SportsCar.java", r#"
                package com.example;
                public class <cur:cls>SportsCar extends <cur:parent>Car {
                    public boolean turbo;
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.SportsCar")
            .assert_at("parent")
                .resolves_to("com.example.Car")
            .run();
    }

    #[test]
    fn super_dot_completion_inside_subclass_method() {
        fixture()
            .file("com/example/Animal.java", r#"
                package com.example;
                public class Animal {
                    public void speak() {}
                    protected String name = "animal";
                    private int id = 0;
                }
            "#)
            .file("com/example/Dog.java", r#"
                package com.example;
                public class Dog extends Animal {
                    public void fetch() {}
                    public void run() {
                        super.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("speak", SymbolKind::Method));
                assert!(items.has("name", SymbolKind::Field));
                assert!(!items.has("id", SymbolKind::Field));
                assert!(!items.has("fetch", SymbolKind::Method));
            })
            .expected_failure("super-dot completion not yet implemented")
            .run();
    }
}

// §8.1.5 — Superinterfaces
mod jls_8_1_5_superinterfaces {
    use super::*;

    // @keep — cross-file resolves_to for interface in implements clause
    #[test]
    fn single_interface_implementation() {
        fixture()
            .file("com/example/Printable.java", r#"
                package com.example;
                public interface Printable {
                    void print();
                }
            "#)
            .file("com/example/Document.java", r#"
                package com.example;
                public class <cur:cls>Document implements <cur:iface>Printable {
                    public void print() {}
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Document")
            .assert_at("iface")
                .resolves_to("com.example.Printable")
            .run();
    }

    #[test]
    fn dot_completion_on_resource_shows_interface_methods() {
        fixture()
            .file("com/example/Closeable.java", r#"
                package com.example;
                public interface Closeable {
                    void close();
                }
            "#)
            .file("com/example/Resource.java", r#"
                package com.example;
                public class Resource implements Closeable {
                    public void use() {}
                    public void close() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Resource resource) {
                        resource.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("use", SymbolKind::Method));
                assert!(items.has("close", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    // @keep — cross-file resolves_to for both interfaces in multiple implements clause
    #[test]
    fn multiple_interface_implementation() {
        fixture()
            .file("com/example/Readable.java", r#"
                package com.example;
                public interface Readable {
                    String read();
                }
            "#)
            .file("com/example/Writable.java", r#"
                package com.example;
                public interface Writable {
                    void write(String data);
                }
            "#)
            .file("com/example/FileStream.java", r#"
                package com.example;
                public class <cur:cls>FileStream implements <cur:r>Readable, <cur:w>Writable {
                    public String read() { return ""; }
                    public void write(String data) {}
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.FileStream")
            .assert_at("r")
                .resolves_to("com.example.Readable")
            .assert_at("w")
                .resolves_to("com.example.Writable")
            .run();
    }

    #[test]
    fn dot_completion_multiple_interfaces_deduplicates_close() {
        fixture()
            .file("com/example/Readable.java", r#"
                package com.example;
                public interface Readable {
                    void close();
                }
            "#)
            .file("com/example/Writable.java", r#"
                package com.example;
                public interface Writable {
                    void close();
                    void flush();
                }
            "#)
            .file("com/example/Stream.java", r#"
                package com.example;
                public class Stream implements Readable, Writable {
                    public void close() {}
                    public void flush() {}
                    public void open() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Stream stream) {
                        stream.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("close", SymbolKind::Method));
                assert!(items.has("flush", SymbolKind::Method));
                assert!(items.has("open", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.2 — Class Members (cross-package access control)
mod jls_8_2_class_members {
    use super::*;

    #[test]
    fn dot_completion_cross_package_visibility() {
        fixture()
            .file("com/pkg1/Base.java", r#"
                package com.pkg1;
                public class Base {
                    public void pub() {}
                    protected void prot() {}
                    void pkg() {}
                    private void priv() {}
                }
            "#)
            .file("com/pkg2/App.java", r#"
                package com.pkg2;
                import com.pkg1.Base;
                public class App {
                    public void run(Base base) {
                        base.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("pub", SymbolKind::Method));
                assert!(!items.has("prot", SymbolKind::Method));
                assert!(!items.has("pkg", SymbolKind::Method));
                assert!(!items.has("priv", SymbolKind::Method));
            })
            .expected_failure("cross-package visibility filtering not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_protected_visible_in_subclass_different_package() {
        fixture()
            .file("com/base/Animal.java", r#"
                package com.base;
                public class Animal {
                    public void eat() {}
                    protected void breathe() {}
                    private void digest() {}
                }
            "#)
            .file("com/pets/Dog.java", r#"
                package com.pets;
                import com.base.Animal;
                public class Dog extends Animal {
                    public void bark() {}
                }
            "#)
            .file("com/pets/App.java", r#"
                package com.pets;
                public class App {
                    public void run(Dog dog) {
                        dog.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("bark", SymbolKind::Method));
                assert!(items.has("eat", SymbolKind::Method));
                assert!(items.has("breathe", SymbolKind::Method));
                assert!(!items.has("digest", SymbolKind::Method));
            })
            .expected_failure("protected subclass visibility not yet implemented")
            .run();
    }
}

// §8.1.6 — Permitted Direct Subclasses
mod jls_8_1_6_sealed_classes {
    use super::*;

    // @evolve — add dot-completion test: method taking Shape parameter, cursor after shape., expect area(); also cursor after Circle/Square instance to verify their area() method
    #[test]
    fn sealed_class_with_permits() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public sealed class <cur:sealed>Shape permits Circle, Square {
                    public abstract double area();
                }
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public final class <cur:circle>Circle extends Shape {
                    private double radius;
                    public double area() { return Math.PI * radius * radius; }
                }
            "#)
            .file("com/example/Square.java", r#"
                package com.example;
                public final class <cur:square>Square extends Shape {
                    private double side;
                    public double area() { return side * side; }
                }
            "#)
            .assert_at("sealed")
                .kind(SymbolKind::Class)
                .fqn("com.example.Shape")
                .modifiers(vec![Modifier::Public, Modifier::Sealed])
            .assert_at("circle")
                .kind(SymbolKind::Class)
                .fqn("com.example.Circle")
                .modifiers(vec![Modifier::Public, Modifier::Final])
            .assert_at("square")
                .kind(SymbolKind::Class)
                .fqn("com.example.Square")
            .run();
    }

    #[test]
    fn dot_completion_on_sealed_shape() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public sealed class Shape permits Circle, Square {
                    public abstract double area();
                }
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public final class Circle extends Shape {
                    private double radius;
                    public double area() { return Math.PI * radius * radius; }
                }
            "#)
            .file("com/example/Square.java", r#"
                package com.example;
                public final class Square extends Shape {
                    private double side;
                    public double area() { return side * side; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Shape shape, Circle circle, Square square) {
                        shape.<cur:shape_dot>
                        circle.<cur:circle_dot>
                        square.<cur:square_dot>
                    }
                }
            "#)
            .complete("shape_dot", |items| {
                assert!(items.has("area", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .complete("circle_dot", |items| {
                assert!(items.has("area", SymbolKind::Method));
                assert!(!items.has("radius", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .complete("square_dot", |items| {
                assert!(items.has("area", SymbolKind::Method));
                assert!(!items.has("side", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_through_sealed_non_sealed_chain() {
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public sealed class Shape permits Circle, Polygon {
                    public String describe() { return "shape"; }
                }
            "#)
            .file("com/example/Circle.java", r#"
                package com.example;
                public final class Circle extends Shape {
                    public double radius() { return 0; }
                }
            "#)
            .file("com/example/Polygon.java", r#"
                package com.example;
                public non-sealed class Polygon extends Shape {
                    public int sides() { return 0; }
                }
            "#)
            .file("com/example/Triangle.java", r#"
                package com.example;
                public class Triangle extends Polygon {
                    public double area() { return 0; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Triangle triangle) {
                        triangle.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("area", SymbolKind::Method));
                assert!(items.has("sides", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.3 — Field Declarations
mod jls_8_3_field_declarations {
    use super::*;

    // @evolve — add dot-completion test: App.java in same package with Config instance, cursor after config., expect NAME (public static), debug (protected), but not port (private)
    #[test]
    fn various_field_types_and_modifiers() {
        fixture()
            .file("com/example/Config.java", r#"
                package com.example;
                public class <cur:cls>Config {
                    public static final String <cur:name>NAME = "app";
                    private int <cur:port>port;
                    protected boolean <cur:debug>debug;
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Config")
                .children_include(&["NAME", "port", "debug"])
            .assert_at("name")
                .kind(SymbolKind::Field)
                .fqn("com.example.Config.NAME")
                .modifiers(vec![Modifier::Public, Modifier::Static, Modifier::Final])
                .parent_fqn("com.example.Config")
            .assert_at("port")
                .kind(SymbolKind::Field)
                .fqn("com.example.Config.port")
                .modifiers(vec![Modifier::Private])
            .assert_at("debug")
                .kind(SymbolKind::Field)
                .fqn("com.example.Config.debug")
                .modifiers(vec![Modifier::Protected])
            .run();
    }

    #[test]
    fn dot_completion_on_config() {
        fixture()
            .file("com/example/Config.java", r#"
                package com.example;
                public class Config {
                    public static final String NAME = "app";
                    private int port;
                    protected boolean debug;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Config config) {
                        config.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("NAME", SymbolKind::Field));
                assert!(items.has("debug", SymbolKind::Field));
                assert!(!items.has("port", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn static_completion_on_constants_class() {
        fixture()
            .file("com/example/Constants.java", r#"
                package com.example;
                public class Constants {
                    public static final int MAX = 100;
                    public static final String NAME = "app";
                    private int counter;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Constants.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("MAX", SymbolKind::Field));
                assert!(items.has("NAME", SymbolKind::Field));
                assert!(!items.has("counter", SymbolKind::Field));
            })
            .expected_failure("static member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_field_hiding_in_subclass() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public int value = 1;
                }
            "#)
            .file("com/example/Sub.java", r#"
                package com.example;
                public class Sub extends Base {
                    public String value = "hello";
                    public int baseOnly = 42;
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Sub sub) {
                        sub.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("value", SymbolKind::Field));
                assert!(items.has("baseOnly", SymbolKind::Field));
            })
            .expected_failure("field hiding completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_field_and_method_same_name() {
        fixture()
            .file("com/example/Widget.java", r#"
                package com.example;
                public class Widget {
                    public int size;
                    public int size() { return size; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Widget widget) {
                        widget.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("size", SymbolKind::Field));
                assert!(items.has("size", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.4.3 — Static Methods — static vs instance filtering on class name
mod jls_8_4_3_static_methods {
    use super::*;

    #[test]
    fn static_completion_excludes_instance_methods() {
        fixture()
            .file("com/example/MathUtils.java", r#"
                package com.example;
                public class MathUtils {
                    public static int add(int a, int b) { return a + b; }
                    public static int multiply(int a, int b) { return a * b; }
                    public String describe() { return "MathUtils"; }
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
                assert!(items.has("add", SymbolKind::Method));
                assert!(items.has("multiply", SymbolKind::Method));
                assert!(!items.has("describe", SymbolKind::Method));
            })
            .expected_failure("static member completion not yet implemented")
            .run();
    }
}

// §8.4.3.1 — abstract Methods
mod jls_8_4_3_1_abstract_methods {
    use super::*;

    #[test]
    fn abstract_method_with_body_is_a_compile_error() {
        // JLS §8.4.3.1: a method declared `abstract` may not be defined
        // with a body. Rule `abstract-method-with-body` fires once for
        // every such declaration; concrete methods and bodyless
        // abstract methods are unaffected.
        fixture()
            .file("com/example/Shape.java", r#"
                package com.example;
                public abstract class Shape {
                    public abstract double area();
                    public abstract double bad() { return 0; }
                    public String describe() { return ""; }
                }
            "#)
            .diagnostics("com/example/Shape.java", |findings| {
                assert_eq!(
                    findings.count_code("abstract-method-with-body"),
                    1,
                    "expected exactly one abstract-method-with-body diagnostic; \
                     got {} total: {:#?}",
                    findings.count(),
                    findings.iter().collect::<Vec<_>>()
                );
            })
            .run();
    }

    #[test]
    fn three_abstract_methods_with_bodies_emit_three_diagnostics() {
        // Multi-fire sanity: three offending methods, three diagnostics.
        fixture()
            .file("com/example/Bad.java", r#"
                package com.example;
                public abstract class Bad {
                    public abstract int a() { return 1; }
                    public abstract int b() { return 2; }
                    public abstract int c() { return 3; }
                }
            "#)
            .diagnostics("com/example/Bad.java", |findings| {
                assert_eq!(findings.count_code("abstract-method-with-body"), 3);
            })
            .run();
    }
}

// §8.4.8 — Inheritance, Overriding, and Hiding
mod jls_8_4_8_inheritance_overriding {
    use super::*;

    // @evolve — add dot-completion test: App.java with Derived instance, cursor after derived., expect describe(); verify overridden method appears in completion
    #[test]
    fn method_override() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public String describe() { return "base"; }
                }
            "#)
            .file("com/example/Derived.java", r#"
                package com.example;
                public class Derived extends Base {
                    public String <cur:ovr>describe() { return "derived"; }
                }
            "#)
            .assert_at("ovr")
                .kind(SymbolKind::Method)
                .fqn("com.example.Derived.describe")
                .signature_return("String")
                .expected_failure("cannot resolve method in subclass that overrides parent")
            .run();
    }

    #[test]
    fn dot_completion_on_derived() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public String describe() { return "base"; }
                }
            "#)
            .file("com/example/Derived.java", r#"
                package com.example;
                public class Derived extends Base {
                    public String describe() { return "derived"; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Derived derived) {
                        derived.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("describe", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    /// When a subclass overrides a method, it should appear only once in
    /// completion (not once for the override and once for the inherited version).
    #[test]
    fn dot_completion_override_not_duplicated() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public String toString() { return "base"; }
                    public void baseOnly() {}
                }
            "#)
            .file("com/example/Sub.java", r#"
                package com.example;
                public class Sub extends Base {
                    public String toString() { return "sub"; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Sub sub) {
                        sub.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("toString", SymbolKind::Method));
                assert!(items.has("baseOnly", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    /// Covariant return: Cat.copy() returns Cat instead of Animal.
    /// The base method cursor resolves via the method name in Animal's scope,
    /// but the override in Cat returns a narrower type.
    // @evolve — add dot-completion test: cursor after Animal instance expect copy(); cursor after Cat instance expect copy() (both should show copy())
    #[test]
    fn covariant_return_type() {
        fixture()
            .file("com/example/Animal.java", r#"
                package com.example;
                public class Animal {
                    public Animal <cur:base_copy>copy() { return new Animal(); }
                }
            "#)
            .file("com/example/Cat.java", r#"
                package com.example;
                public class Cat extends Animal {
                    public Cat <cur:covariant>copy() { return new Cat(); }
                }
            "#)
            .assert_at("base_copy")
                .kind(SymbolKind::Method)
                .fqn("com.example.Animal.copy")
                .signature_return("Animal")
                .expected_failure("cannot resolve method 'copy' — name conflicts with constructor-like heuristic")
            .assert_at("covariant")
                .kind(SymbolKind::Method)
                .fqn("com.example.Cat.copy")
                .signature_return("Cat")
                .expected_failure("cannot resolve method in subclass with covariant return")
            .run();
    }

    #[test]
    fn dot_completion_on_animal_and_cat() {
        fixture()
            .file("com/example/Animal.java", r#"
                package com.example;
                public class Animal {
                    public Animal copy() { return new Animal(); }
                }
            "#)
            .file("com/example/Cat.java", r#"
                package com.example;
                public class Cat extends Animal {
                    public Cat copy() { return new Cat(); }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Animal animal, Cat cat) {
                        animal.<cur:animal_dot>
                        cat.<cur:cat_dot>
                    }
                }
            "#)
            .complete("animal_dot", |items| {
                assert!(items.has("copy", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .complete("cat_dot", |items| {
                assert!(items.has("copy", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn static_completion_on_child_shows_hiding_method() {
        fixture()
            .file("com/example/Parent.java", r#"
                package com.example;
                public class Parent {
                    public static void greet() {}
                    public static void parentOnly() {}
                }
            "#)
            .file("com/example/Child.java", r#"
                package com.example;
                public class Child extends Parent {
                    public static void greet() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Child.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("greet", SymbolKind::Method));
                assert!(items.has("parentOnly", SymbolKind::Method));
            })
            .expected_failure("static method hiding completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_includes_final_inherited_method() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public final void lock() {}
                    public void process() {}
                }
            "#)
            .file("com/example/Sub.java", r#"
                package com.example;
                public class Sub extends Base {
                    public void extra() {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Sub sub) {
                        sub.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("lock", SymbolKind::Method));
                assert!(items.has("process", SymbolKind::Method));
                assert!(items.has("extra", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.4.9 — Overloading
mod jls_8_4_9_overloading {
    use super::*;

    #[test]
    fn dot_completion_on_printer_shows_overloaded_method() {
        fixture()
            .file("com/example/Document.java", r#"
                package com.example;
                public class Document {}
            "#)
            .file("com/example/Printer.java", r#"
                package com.example;
                public class Printer {
                    public void print(String text) {}
                    public void print(String text, int copies) {}
                    public void print(Document doc) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Printer printer) {
                        printer.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("print", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_mixed_inherited_and_own_overloads() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public void process(String s) {}
                }
            "#)
            .file("com/example/Sub.java", r#"
                package com.example;
                public class Sub extends Base {
                    public void process(int n) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Sub sub) {
                        sub.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("process", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.4.4 — Generic Methods
mod jls_8_4_4_generic_methods {
    use super::*;

    #[test]
    fn static_completion_on_generic_method_class() {
        fixture()
            .file("com/example/Collections.java", r#"
                package com.example;
                public class Collections {
                    public static <T> Object singletonList(T item) { return null; }
                    public static <T extends Comparable<T>> void sort(Object list) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Collections.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("singletonList", SymbolKind::Method));
                assert!(items.has("sort", SymbolKind::Method));
            })
            .expected_failure("static member completion not yet implemented")
            .run();
    }
}

// §8.5 — Member Class and Interface Declarations
mod jls_8_5_member_classes {
    use super::*;

    // @evolve — add dot-completion test: App.java creates LinkedList.Node, cursor after node., expect data and next fields
    #[test]
    fn static_nested_class_with_members() {
        fixture()
            .file("com/example/LinkedList.java", r#"
                package com.example;
                public class <cur:list>LinkedList {
                    private Node head;

                    private static class <cur:node>Node {
                        Object <cur:data>data;
                        Node next;
                    }
                }
            "#)
            .assert_at("list")
                .kind(SymbolKind::Class)
                .fqn("com.example.LinkedList")
                .children_include(&["head", "Node"])
            .assert_at("node")
                .kind(SymbolKind::Class)
                .fqn("com.example.LinkedList.Node")
                .parent_fqn("com.example.LinkedList")
                .modifiers(vec![Modifier::Private, Modifier::Static])
            .assert_at("data")
                .kind(SymbolKind::Field)
                .fqn("com.example.LinkedList.Node.data")
                .parent_fqn("com.example.LinkedList.Node")
            .run();
    }

    #[test]
    fn dot_completion_on_linked_list_node() {
        fixture()
            .file("com/example/LinkedList.java", r#"
                package com.example;
                public class LinkedList {
                    public static class Node {
                        public Object data;
                        public Node next;
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(LinkedList.Node node) {
                        node.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("data", SymbolKind::Field));
                assert!(items.has("next", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    // @evolve — add dot-completion test: App.java with Outer.Inner instance, cursor after inner., expect getX(); with Outer.Nested instance, expect helper()
    #[test]
    fn inner_class_vs_static_nested() {
        fixture()
            .file("com/example/Outer.java", r#"
                package com.example;
                public class Outer {
                    private int x;

                    public class <cur:inner>Inner {
                        public int getX() { return x; }
                    }

                    public static class <cur:nested>Nested {
                        public static void helper() {}
                    }
                }
            "#)
            .assert_at("inner")
                .kind(SymbolKind::Class)
                .fqn("com.example.Outer.Inner")
                .modifiers(vec![Modifier::Public])
                .parent_fqn("com.example.Outer")
            .assert_at("nested")
                .kind(SymbolKind::Class)
                .fqn("com.example.Outer.Nested")
                .modifiers(vec![Modifier::Public, Modifier::Static])
                .parent_fqn("com.example.Outer")
            .run();
    }

    #[test]
    fn dot_completion_on_outer_inner_and_nested() {
        fixture()
            .file("com/example/Outer.java", r#"
                package com.example;
                public class Outer {
                    private int x;

                    public class Inner {
                        public int getX() { return x; }
                    }

                    public static class Nested {
                        public static void helper() {}
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Outer.Inner inner, Outer.Nested nested) {
                        inner.<cur:inner_dot>
                        nested.<cur:nested_dot>
                    }
                }
            "#)
            .complete("inner_dot", |items| {
                assert!(items.has("getX", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .complete("nested_dot", |items| {
                assert!(items.has("helper", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn static_completion_on_outer_shows_nested_type() {
        fixture()
            .file("com/example/Outer.java", r#"
                package com.example;
                public class Outer {
                    public static class Builder {
                        public Outer build() { return new Outer(); }
                    }
                    public class Inner {
                        public void act() {}
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Outer.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("Builder", SymbolKind::Class));
            })
            .expected_failure("nested type completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_inherited_nested_type_via_subclass() {
        fixture()
            .file("com/example/Base.java", r#"
                package com.example;
                public class Base {
                    public static class Config {
                        public String setting;
                    }
                }
            "#)
            .file("com/example/Sub.java", r#"
                package com.example;
                public class Sub extends Base {}
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Sub.Config config) {
                        config.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("setting", SymbolKind::Field));
            })
            .expected_failure("inherited nested type completion not yet implemented")
            .run();
    }
}

// §8.8 — Constructor Declarations
mod jls_8_8_constructors {
    use super::*;

    /// Constructor cursors resolve to the class name token. The LSP currently
    /// reports them as SymbolKind::Class; detecting Constructor kind requires
    /// distinguishing constructor declarations from class declarations.
    // @evolve — add dot-completion test: App.java with Point instance, cursor after point., expect x and y fields
    #[test]
    fn default_and_parameterized_constructors() {
        fixture()
            .file("com/example/Point.java", r#"
                package com.example;
                public class <cur:cls>Point {
                    private int x;
                    private int y;

                    public <cur:default_ctor>Point() {
                        this.x = 0;
                        this.y = 0;
                    }

                    public <cur:param_ctor>Point(int x, int y) {
                        this.x = x;
                        this.y = y;
                    }
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.Point")
                .children_include(&["x", "y", "Point"])
            .assert_at("default_ctor")
                .kind(SymbolKind::Constructor)
                .fqn("com.example.Point.Point")
                .signature_params(&[])
                .parent_fqn("com.example.Point")
                .expected_failure("constructor kind detection not implemented — reports Class")
            .assert_at("param_ctor")
                .kind(SymbolKind::Constructor)
                .fqn("com.example.Point.Point")
                .signature_params(&[("x", "int"), ("y", "int")])
                .parent_fqn("com.example.Point")
                .expected_failure("constructor kind detection not implemented — reports Class")
            .run();
    }

    #[test]
    fn dot_completion_on_point() {
        fixture()
            .file("com/example/Point.java", r#"
                package com.example;
                public class Point {
                    private int x;
                    private int y;

                    public Point() { this.x = 0; this.y = 0; }
                    public Point(int x, int y) { this.x = x; this.y = y; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Point point) {
                        point.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(!items.has("x", SymbolKind::Field));
                assert!(!items.has("y", SymbolKind::Field));
            })
            .run();
    }

    /// Private constructor (singleton pattern). Same limitation: kind is Class not Constructor.
    // @evolve — add dot-completion test: App.java accessing Singleton., cursor after Singleton., expect getInstance() (only public static method visible)
    #[test]
    fn private_constructor() {
        fixture()
            .file("com/example/Singleton.java", r#"
                package com.example;
                public class Singleton {
                    private static Singleton instance;

                    private <cur:ctor>Singleton() {}

                    public static Singleton getInstance() {
                        if (instance == null) instance = new Singleton();
                        return instance;
                    }
                }
            "#)
            .assert_at("ctor")
                .kind(SymbolKind::Constructor)
                .fqn("com.example.Singleton.Singleton")
                .modifiers(vec![Modifier::Private])
                .expected_failure("constructor kind detection not implemented — reports Class")
            .run();
    }

    #[test]
    fn static_completion_on_singleton() {
        fixture()
            .file("com/example/Singleton.java", r#"
                package com.example;
                public class Singleton {
                    private static Singleton instance;

                    private Singleton() {}

                    public static Singleton getInstance() {
                        if (instance == null) instance = new Singleton();
                        return instance;
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Singleton.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getInstance", SymbolKind::Method));
                assert!(!items.has("instance", SymbolKind::Field));
            })
            .expected_failure("static member completion not yet implemented")
            .run();
    }

    #[test]
    fn constructor_parameter_completion_shows_overloads() {
        fixture()
            .file("com/example/Service.java", r#"
                package com.example;
                public class Service {
                    public Service(String host, int port) {}
                    public Service(String host) {}
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        new Service(<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("Service", SymbolKind::Constructor));
            })
            .expected_failure("constructor parameter completion not yet implemented")
            .run();
    }
}

// §8.9 — Enum Classes
mod jls_8_9_enums {
    use super::*;

    // @evolve — add dot-completion test: App.java with Planet instance, cursor after planet., expect getMass() and accessible fields
    #[test]
    fn enum_with_fields_and_methods() {
        fixture()
            .file("com/example/Planet.java", r#"
                package com.example;
                public enum <cur:enm>Planet {
                    MERCURY(3.303e+23, 2.4397e6),
                    EARTH(5.976e+24, 6.37814e6);

                    private final double <cur:mass>mass;
                    private final double radius;

                    Planet(double mass, double radius) {
                        this.mass = mass;
                        this.radius = radius;
                    }

                    public double <cur:get_mass>getMass() { return mass; }
                }
            "#)
            .assert_at("enm")
                .kind(SymbolKind::Enum)
                .fqn("com.example.Planet")
                .children_include(&["mass", "radius", "getMass"])
            .assert_at("mass")
                .kind(SymbolKind::Field)
                .fqn("com.example.Planet.mass")
                .parent_fqn("com.example.Planet")
            .assert_at("get_mass")
                .kind(SymbolKind::Method)
                .fqn("com.example.Planet.getMass")
                .signature_return("double")
                .parent_fqn("com.example.Planet")
            .run();
    }

    #[test]
    fn dot_completion_on_planet() {
        fixture()
            .file("com/example/Planet.java", r#"
                package com.example;
                public enum Planet {
                    MERCURY(3.303e+23, 2.4397e6),
                    EARTH(5.976e+24, 6.37814e6);

                    private final double mass;
                    private final double radius;

                    Planet(double mass, double radius) {
                        this.mass = mass;
                        this.radius = radius;
                    }

                    public double getMass() { return mass; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Planet planet) {
                        planet.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getMass", SymbolKind::Method));
                assert!(!items.has("mass", SymbolKind::Field));
                assert!(!items.has("radius", SymbolKind::Field));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn static_completion_on_season_enum_constants() {
        fixture()
            .file("com/example/Season.java", r#"
                package com.example;
                public enum Season {
                    SPRING, SUMMER, FALL, WINTER;
                    public String displayName() { return name().toLowerCase(); }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run() {
                        Season.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("SPRING", SymbolKind::Field));
                assert!(items.has("SUMMER", SymbolKind::Field));
                assert!(items.has("FALL", SymbolKind::Field));
                assert!(items.has("WINTER", SymbolKind::Field));
                assert!(!items.has("displayName", SymbolKind::Method));
            })
            .expected_failure("static enum member completion not yet implemented")
            .run();
    }

    // @keep — cross-file resolves_to for interface in enum implements clause
    #[test]
    fn enum_implementing_interface() {
        fixture()
            .file("com/example/Printable.java", r#"
                package com.example;
                public interface Printable {
                    String display();
                }
            "#)
            .file("com/example/Color.java", r#"
                package com.example;
                public enum <cur:enm>Color implements <cur:iface>Printable {
                    RED, GREEN, BLUE;

                    public String <cur:display>display() { return name().toLowerCase(); }
                }
            "#)
            .assert_at("enm")
                .kind(SymbolKind::Enum)
                .fqn("com.example.Color")
            .assert_at("iface")
                .resolves_to("com.example.Printable")
            .assert_at("display")
                .kind(SymbolKind::Method)
                .fqn("com.example.Color.display")
                .signature_return("String")
                .expected_failure("method 'display' inside enum body not resolved")
            .run();
    }

    #[test]
    fn dot_completion_on_enum_with_abstract_method() {
        fixture()
            .file("com/example/Operation.java", r#"
                package com.example;
                public enum Operation {
                    PLUS {
                        public double apply(double x, double y) { return x + y; }
                    },
                    MINUS {
                        public double apply(double x, double y) { return x - y; }
                    };
                    public abstract double apply(double x, double y);
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Operation op) {
                        op.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("apply", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_enum_with_interface_and_builtin_methods() {
        fixture()
            .file("com/example/Describable.java", r#"
                package com.example;
                public interface Describable {
                    String describe();
                }
            "#)
            .file("com/example/Priority.java", r#"
                package com.example;
                public enum Priority implements Describable {
                    HIGH, MEDIUM, LOW;
                    public String describe() { return name().toLowerCase(); }
                    public int level() { return ordinal(); }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Priority priority) {
                        priority.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("describe", SymbolKind::Method));
                assert!(items.has("level", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }
}

// §8.10 — Record Classes
mod jls_8_10_records {
    use super::*;

    // @evolve — add dot-completion test: App.java with Range instance, cursor after range., expect size() and record accessor methods lo()/hi()
    #[test]
    fn record_with_custom_method() {
        fixture()
            .file("com/example/Range.java", r#"
                package com.example;
                public record <cur:rec>Range(int lo, int hi) {
                    public int <cur:size>size() { return hi - lo; }
                }
            "#)
            .assert_at("rec")
                .kind(SymbolKind::Record)
                .fqn("com.example.Range")
                .children_include(&["size"])
            .assert_at("size")
                .kind(SymbolKind::Method)
                .fqn("com.example.Range.size")
                .signature_return("int")
                .parent_fqn("com.example.Range")
            .run();
    }

    #[test]
    fn dot_completion_on_range() {
        fixture()
            .file("com/example/Range.java", r#"
                package com.example;
                public record Range(int lo, int hi) {
                    public int size() { return hi - lo; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Range range) {
                        range.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("size", SymbolKind::Method));
                assert!(items.has("lo", SymbolKind::Method));
                assert!(items.has("hi", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_generic_pair_record_accessors() {
        fixture()
            .file("com/example/Pair.java", r#"
                package com.example;
                public record Pair<A, B>(A first, B second) {}
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Pair<String, Integer> pair) {
                        pair.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("first", SymbolKind::Method));
                assert!(items.has("second", SymbolKind::Method));
            })
            .expected_failure("record accessor completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_record_with_interface_impl() {
        fixture()
            .file("com/example/Measurable.java", r#"
                package com.example;
                public interface Measurable {
                    double measure();
                }
            "#)
            .file("com/example/Box.java", r#"
                package com.example;
                public record Box(double width, double height) implements Measurable {
                    public double measure() { return width * height; }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Box box) {
                        box.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("width", SymbolKind::Method));
                assert!(items.has("height", SymbolKind::Method));
                assert!(items.has("measure", SymbolKind::Method));
            })
            .expected_failure("member completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_on_record_with_compact_canonical_constructor() {
        fixture()
            .file("com/example/Email.java", r#"
                package com.example;
                public record Email(String address) {
                    public Email {
                        if (!address.contains("@")) throw new IllegalArgumentException();
                    }
                    public String domain() {
                        return address.substring(address.indexOf("@") + 1);
                    }
                }
            "#)
            .file("com/example/App.java", r#"
                package com.example;
                public class App {
                    public void run(Email email) {
                        email.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("address", SymbolKind::Method));
                assert!(items.has("domain", SymbolKind::Method));
            })
            .expected_failure("record accessor completion not yet implemented")
            .run();
    }
}

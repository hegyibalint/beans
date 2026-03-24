use beans_core::{Modifier, SymbolKind};

fn fixture() -> beans_test_harness::fixture::Fixture {
    crate::prelude::fixture()
}

// §10.1 — Array Types
mod jls_10_1_array_types {
    use super::*;

    // @keep — hover_contains("int[]") verifies primitive array type preserved in field hover
    #[test]
    fn primitive_array_field() {
        fixture()
            .file("com/example/Scores.java", r#"
                package com.example;
                public class Scores {
                    private int[] <cur:scores>scores;
                }
            "#)
            .assert_at("scores")
                .kind(SymbolKind::Field)
                .fqn("com.example.Scores.scores")
                .name("scores")
                .parent_fqn("com.example.Scores")
                .modifiers(vec![Modifier::Private])
                .hover_contains("int[]")
            .run();
    }

    // @keep — hover_contains("String[]") verifies object array type preserved in field hover
    #[test]
    fn string_array_field() {
        fixture()
            .file("com/example/Names.java", r#"
                package com.example;
                public class Names {
                    private String[] <cur:names>names;
                }
            "#)
            .assert_at("names")
                .kind(SymbolKind::Field)
                .fqn("com.example.Names.names")
                .name("names")
                .modifiers(vec![Modifier::Private])
                .hover_contains("String[]")
            .run();
    }

    /// Multidimensional arrays should preserve all bracket dimensions in hover.
    /// Known issue: `int[][]` renders as `int[]` (dimensions are lost).
    // @keep — multidimensional array brackets currently lost in hover (int[][] shows as int[]); documents regression
    #[test]
    fn multidimensional_array_hover() {
        fixture()
            .file("com/example/Matrix.java", r#"
                package com.example;
                public class Matrix {
                    private int[][] <cur:grid>grid;
                    private String[][][] <cur:cube>cube;
                }
            "#)
            .assert_at("grid")
                .kind(SymbolKind::Field)
                .fqn("com.example.Matrix.grid")
                .hover_contains("int[][]")
                .expected_failure("multidimensional array hover loses bracket dimensions")
            .assert_at("cube")
                .kind(SymbolKind::Field)
                .fqn("com.example.Matrix.cube")
                .hover_contains("String[][][]")
                .expected_failure("multidimensional array hover loses bracket dimensions")
            .run();
    }

    // @keep — hover_contains("Object[]") verifies object array type preserved in field hover
    #[test]
    fn object_array_field() {
        fixture()
            .file("com/example/Container.java", r#"
                package com.example;
                public class Container {
                    private Object[] <cur:items>items;
                }
            "#)
            .assert_at("items")
                .kind(SymbolKind::Field)
                .fqn("com.example.Container.items")
                .name("items")
                .hover_contains("Object[]")
            .run();
    }

    // @keep — signature_params and hover_contains verify int[] preserved in method parameter type
    #[test]
    fn array_as_method_parameter() {
        fixture()
            .file("com/example/Sorter.java", r#"
                package com.example;
                public class Sorter {
                    public void <cur:sort>sort(int[] numbers) {}
                }
            "#)
            .assert_at("sort")
                .kind(SymbolKind::Method)
                .fqn("com.example.Sorter.sort")
                .signature_return("void")
                .signature_params(&[("numbers", "int[]")])
                .hover_contains("int[]")
            .run();
    }

    // @keep — signature_return and hover_contains verify String[] preserved in method return type
    #[test]
    fn array_as_return_type() {
        fixture()
            .file("com/example/DataSource.java", r#"
                package com.example;
                public class DataSource {
                    public String[] <cur:fetch>fetchAll() { return null; }
                }
            "#)
            .assert_at("fetch")
                .kind(SymbolKind::Method)
                .fqn("com.example.DataSource.fetchAll")
                .signature_return("String[]")
                .hover_contains("String[]")
            .run();
    }

    // @keep — static method with array param and array return: both types preserved in signature
    #[test]
    fn array_param_and_return_combined() {
        fixture()
            .file("com/example/ArrayUtils.java", r#"
                package com.example;
                public class ArrayUtils {
                    public static int[] <cur:filter>filter(int[] source, int threshold) {
                        return null;
                    }
                }
            "#)
            .assert_at("filter")
                .kind(SymbolKind::Method)
                .fqn("com.example.ArrayUtils.filter")
                .modifiers(vec![Modifier::Public, Modifier::Static])
                .signature_return("int[]")
                .signature_params(&[("source", "int[]"), ("threshold", "int")])
            .run();
    }

    /// C-style array declaration: `int nums[]` instead of `int[] nums` (§10.2).
    /// Known issue: hover shows `int nums` instead of `int[] nums` — the brackets
    /// after the identifier are not folded into the type.
    // @keep — C-style `int nums[]` array: brackets not yet folded into type in hover; expected_failure
    #[test]
    fn c_style_array_declaration() {
        fixture()
            .file("com/example/Legacy.java", r#"
                package com.example;
                public class Legacy {
                    private int <cur:nums>nums[];
                }
            "#)
            .assert_at("nums")
                .kind(SymbolKind::Field)
                .fqn("com.example.Legacy.nums")
                .name("nums")
                .hover_contains("int[]")
                .expected_failure("C-style array brackets not folded into type in hover")
            .run();
    }
}

// §10.7 — Array Members
mod jls_10_7_array_members {
    use super::*;

    /// Every array has a `public final int length` field (JLS §10.7).
    /// The LSP should resolve `data.length` to this synthetic member.
    // @keep — array .length synthetic member not yet resolved; documents missing JLS §10.7 feature
    #[test]
    fn array_length_field() {
        fixture()
            .file("com/example/LengthCheck.java", r#"
                package com.example;
                public class LengthCheck {
                    public int getSize(int[] data) {
                        return data.<cur:len>length;
                    }
                }
            "#)
            .assert_at("len")
                .kind(SymbolKind::Field)
                .name("length")
                .expected_failure("array .length synthetic member not yet resolved")
            .run();
    }

    /// Every array type has a `public T[] clone()` method (JLS §10.7).
    /// The LSP should resolve `original.clone()` to this synthetic member.
    // @keep — array .clone() synthetic member not yet resolved; documents missing JLS §10.7 feature
    #[test]
    fn array_clone_method() {
        fixture()
            .file("com/example/Cloner.java", r#"
                package com.example;
                public class Cloner {
                    public int[] copy(int[] original) {
                        return original.<cur:clone_call>clone();
                    }
                }
            "#)
            .assert_at("clone_call")
                .kind(SymbolKind::Method)
                .name("clone")
                .expected_failure("array .clone() synthetic member not yet resolved")
            .run();
    }

    /// Arrays are assignable to Object (JLS §10.7, §4.10.3).
    /// Method signature should correctly represent String[] parameter type.
    // @keep — verifies String[] param correctly captured in signature when method accepts Object
    #[test]
    fn array_assignable_to_object() {
        fixture()
            .file("com/example/Boxing.java", r#"
                package com.example;
                public class Boxing {
                    public Object <cur:wrap>wrap(String[] items) {
                        return items;
                    }
                }
            "#)
            .assert_at("wrap")
                .kind(SymbolKind::Method)
                .fqn("com.example.Boxing.wrap")
                .signature_return("Object")
                .signature_params(&[("items", "String[]")])
            .run();
    }

    #[test]
    fn dot_completion_array_length_and_clone() {
        fixture()
            .file("com/example/Processor.java", r#"
                package com.example;
                public class Processor {
                    private int[] data;
                    public void run() {
                        data.<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("length", SymbolKind::Field));
                assert!(items.has("clone", SymbolKind::Method));
            })
            .expected_failure("array synthetic member completion not yet implemented")
            .run();
    }

    /// A class with multiple array-typed fields of varying element types
    /// and dimensions. Verifies children indexing and parent relationships.
    // @keep — DataRecord with 4 array fields of varying element types; verifies children and hover for each
    #[test]
    fn class_with_multiple_array_fields() {
        fixture()
            .file("com/example/DataRecord.java", r#"
                package com.example;
                public class <cur:cls>DataRecord {
                    private int[] <cur:ids>ids;
                    private String[] <cur:labels>labels;
                    private double[][] <cur:matrix>matrix;
                    private Object[] <cur:extras>extras;
                }
            "#)
            .assert_at("cls")
                .kind(SymbolKind::Class)
                .fqn("com.example.DataRecord")
                .children_include(&["ids", "labels", "matrix", "extras"])
                .children_count(4)
            .assert_at("ids")
                .kind(SymbolKind::Field)
                .fqn("com.example.DataRecord.ids")
                .parent_fqn("com.example.DataRecord")
                .hover_contains("int[]")
            .assert_at("labels")
                .kind(SymbolKind::Field)
                .fqn("com.example.DataRecord.labels")
                .parent_fqn("com.example.DataRecord")
                .hover_contains("String[]")
            .assert_at("matrix")
                .kind(SymbolKind::Field)
                .fqn("com.example.DataRecord.matrix")
                .parent_fqn("com.example.DataRecord")
                .hover_contains("double[][]")
                .expected_failure("multidimensional array hover loses bracket dimensions")
            .assert_at("extras")
                .kind(SymbolKind::Field)
                .fqn("com.example.DataRecord.extras")
                .parent_fqn("com.example.DataRecord")
                .hover_contains("Object[]")
            .run();
    }
}

// §10.4 — Array Access
mod jls_10_4_array_access {
    use super::*;

    #[test]
    fn dot_completion_string_array_element() {
        fixture()
            .file("com/example/StringsApp.java", r#"
                package com.example;
                public class StringsApp {
                    private String[] names;
                    public void run() {
                        names[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("length", SymbolKind::Method));
                assert!(items.has("charAt", SymbolKind::Method));
                assert!(items.has("substring", SymbolKind::Method));
            })
            .expected_failure("array element type inference for completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_custom_type_array_element() {
        fixture()
            .file("com/example/User.java", r#"
                package com.example;
                public class User {
                    public String getName() { return null; }
                    public int getAge() { return 0; }
                }
            "#)
            .file("com/example/Registry.java", r#"
                package com.example;
                public class Registry {
                    public void process(User[] users) {
                        users[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getAge", SymbolKind::Method));
            })
            .expected_failure("array element type inference for completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_multidimensional_inner_array() {
        fixture()
            .file("com/example/Grid.java", r#"
                package com.example;
                public class Grid {
                    private int[][] matrix;
                    public void run() {
                        matrix[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("length", SymbolKind::Field));
                assert!(items.has("clone", SymbolKind::Method));
            })
            .expected_failure("multidimensional array element type inference not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_array_param_element() {
        fixture()
            .file("com/example/TextProcessor.java", r#"
                package com.example;
                public class TextProcessor {
                    public void process(String[] items) {
                        items[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("length", SymbolKind::Method));
                assert!(items.has("toUpperCase", SymbolKind::Method));
            })
            .expected_failure("array element type inference for method param not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_interface_array_element() {
        fixture()
            .file("com/example/Printable.java", r#"
                package com.example;
                public interface Printable {
                    void print();
                    String format();
                }
            "#)
            .file("com/example/Renderer.java", r#"
                package com.example;
                public class Renderer {
                    public void renderAll(Printable[] items) {
                        items[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("print", SymbolKind::Method));
                assert!(items.has("format", SymbolKind::Method));
            })
            .expected_failure("interface array element type inference for completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_method_returning_array_chained_access() {
        fixture()
            .file("com/example/User.java", r#"
                package com.example;
                public class User {
                    public String getName() { return null; }
                    public int getAge() { return 0; }
                }
            "#)
            .file("com/example/DataSource.java", r#"
                package com.example;
                public class DataSource {
                    public User[] getUsers() { return null; }
                    public void run(DataSource ds) {
                        ds.getUsers()[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getName", SymbolKind::Method));
                assert!(items.has("getAge", SymbolKind::Method));
            })
            .expected_failure("chained method-return-array element type completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_varargs_param_as_array_element() {
        fixture()
            .file("com/example/Processor.java", r#"
                package com.example;
                public class Processor {
                    public void process(String... args) {
                        args[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("length", SymbolKind::Method));
                assert!(items.has("charAt", SymbolKind::Method));
            })
            .expected_failure("varargs parameter treated as array for element completion not yet implemented")
            .run();
    }

    #[test]
    fn dot_completion_imported_type_array_element() {
        fixture()
            .file("com/model/Product.java", r#"
                package com.model;
                public class Product {
                    public String getTitle() { return null; }
                    public double getPrice() { return 0.0; }
                }
            "#)
            .file("com/shop/Catalog.java", r#"
                package com.shop;
                import com.model.Product;
                public class Catalog {
                    public void display(Product[] catalog) {
                        catalog[0].<cur>
                    }
                }
            "#)
            .complete_default(|items| {
                assert!(items.has("getTitle", SymbolKind::Method));
                assert!(items.has("getPrice", SymbolKind::Method));
            })
            .expected_failure("cross-package array element type inference for completion not yet implemented")
            .run();
    }
}

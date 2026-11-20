use beans_core::ranges::ByteRange;
use beans_platform_jvm::{
    Class, ElementType, FieldType, JvmPlatform, query, storage, virtual_file,
};
use beans_testing::{
    TempDir, database,
    support::{class_bytes, jar_bytes},
};

const INITIALIZER: [u8; 3] = [0x00, 0x00, 0xb1]; // nop; nop; return

#[test]
fn stored_declarations_render_after_the_original_artifact_disappears() {
    let files = TempDir::new();
    let bytes = class_with_members(INITIALIZER);
    let path = files.write(
        "library.jar",
        jar_bytes(&[("example/Outer$Étage.class", &bytes)]),
    );
    let processed = JvmPlatform::new().process(&path).unwrap();
    assert!(processed.failures.is_empty());
    let [(location, original)] = processed.classes.as_slice() else {
        panic!("expected one class");
    };
    let mut connection = database();
    let transaction = connection.transaction().unwrap();
    storage::store(original, location, &transaction).unwrap();
    transaction.commit().unwrap();
    std::fs::remove_file(path).unwrap();

    let file = query::unique_class_by_name(&original.name, &connection)
        .unwrap()
        .unwrap();
    assert_eq!(&file, original);
    assert_eq!(
        file.fingerprint,
        Class::read(bytes.as_slice()).unwrap().unwrap().fingerprint
    );
    assert_eq!(file.name, "example/Outer$Étage");
    assert_eq!(file.superclass.as_deref(), Some("example/Base"));
    assert_eq!(file.interfaces, ["example/Api"]);
    assert_eq!(
        file.fields[0].field_type,
        FieldType {
            dimensions: 2,
            element: ElementType::Reference("example/Étage".into()),
        },
    );
    let [method, bridge, initializer] = file.methods.as_slice() else {
        panic!("expected two methods and a class initializer");
    };
    assert_eq!(initializer.name, "<clinit>");
    assert_eq!(method.name, bridge.name);
    assert_eq!(method.parameters, bridge.parameters);
    assert_ne!(method.return_type, bridge.return_type);

    let rendered = virtual_file::generate(&file).unwrap();
    assert_eq!(
        rendered.text(),
        concat!(
            "package example;\n\n",
            "public class Outer$Étage extends example.Base implements example.Api {\n",
            "    private example.Étage[][] \"total\\nbins\";\n\n",
            "    public native java.lang.String read(int[], example.Arg);\n",
            "    public native bridge synthetic java.lang.Object read(int[], example.Arg);\n",
            "    static void <clinit>();\n",
            "}\n",
        ),
    );
    let slice = |range: ByteRange| &rendered.text()[range.start()..range.end()];
    assert_eq!(slice(rendered.class.name), "Outer$Étage");
    assert_eq!(slice(rendered.fields[0].name), "\"total\\nbins\"");
    assert_ne!(rendered.methods[0].name, rendered.methods[1].name);
    for method in &rendered.methods[..2] {
        assert_eq!(slice(method.name), "read");
        assert!(method.declaration.contains(method.name.start()));
        assert!(slice(method.declaration).ends_with(" read(int[], example.Arg);"));
    }
    assert_eq!(slice(rendered.methods[2].name), "<clinit>");
    assert_eq!(
        slice(rendered.methods[2].declaration),
        "static void <clinit>();"
    );
    assert!(slice(rendered.class.declaration).starts_with("public class "));
    assert!(slice(rendered.class.declaration).ends_with('}'));
}

#[test]
fn rendered_type_names_are_references_to_jvm_classes() {
    let class = Class::read(class_with_members(INITIALIZER).as_slice())
        .unwrap()
        .unwrap();
    let rendered = virtual_file::generate(&class).unwrap();
    let text = rendered.text();

    for (name, internal_name) in [
        ("example.Base", "example/Base"),
        ("example.Api", "example/Api"),
        ("example.Étage", "example/Étage"),
        ("java.lang.String", "java/lang/String"),
        ("java.lang.Object", "java/lang/Object"),
        ("example.Arg", "example/Arg"),
    ] {
        assert!(text.contains(name));
        for (start, _) in text.match_indices(name) {
            let end = start + name.len();
            let hit = rendered.type_reference_at(start).unwrap();
            assert_eq!(hit.value(), internal_name);
            assert_eq!(hit.range(), ByteRange::new(start, end));
            assert_eq!(
                rendered.type_reference_at(end - 1).unwrap().range(),
                hit.range()
            );
            assert!(rendered.type_reference_at(end).is_none());
        }
    }

    let package = text.find("example;").unwrap();
    let primitive = text.find("int[]").unwrap();
    for offset in [
        package,
        primitive,
        rendered.class.name.start(),
        rendered.methods[0].name.start(),
    ] {
        assert!(rendered.type_reference_at(offset).is_none());
    }
}

#[test]
fn a_bytecode_only_change_affects_the_fingerprint_but_not_the_listing() {
    let read = |code| {
        Class::read(class_with_members(code).as_slice())
            .unwrap()
            .unwrap()
    };
    let original = read(INITIALIZER);
    let mut changed = read([0x03, 0x57, 0xb1]); // iconst_0; pop; return

    assert_ne!(original.fingerprint, changed.fingerprint);
    assert_eq!(
        virtual_file::generate(&original).unwrap().text(),
        virtual_file::generate(&changed).unwrap().text(),
    );
    changed.fingerprint = original.fingerprint;
    assert_eq!(original, changed);
}

fn class_with_members(initializer: [u8; 3]) -> Vec<u8> {
    let mut bytes = class_bytes("example/Outer$Étage");
    // Replace the original class body with one that uses additional pool entries.
    bytes.truncate(bytes.len() - 14);
    bytes[8..10].copy_from_slice(&17u16.to_be_bytes());
    for value in [
        "example/Api",                         // #5
        "total\nbins",                         // #6
        "[[Lexample/Étage;",                   // #7
        "read",                                // #8
        "([ILexample/Arg;)Ljava/lang/String;", // #9
        "([ILexample/Arg;)Ljava/lang/Object;", // #10
        "example/Base",                        // #11
        "<clinit>",                            // #12
        "()V",                                 // #13
        "Code",                                // #14
    ] {
        bytes.push(1);
        bytes.extend((value.len() as u16).to_be_bytes());
        bytes.extend(value.as_bytes());
    }
    bytes.extend([7, 0, 5]); // #15: example/Api
    bytes.extend([7, 0, 11]); // #16: example/Base

    for value in [
        0x0021u16, 2, 16, 1, 15, // class header and interface
        1, 0x0002, 6, 7, 0, // private field
        3, 0x0101, 8, 9, 0, // public native method
        0x1141, 8, 10, 0, // public native bridge synthetic method
        0x0008, 12, 13, 1, // static initializer with one attribute
        14, 0, 15, 1, 0, 0, 3, // Code: length 15, one stack slot, no locals, three code bytes
    ] {
        bytes.extend(value.to_be_bytes());
    }
    bytes.extend(initializer);
    bytes.extend([0; 6]); // empty exception table, Code attributes and class attributes
    bytes
}

use std::path::Path;

use beans_core::{SymbolKind, SymbolTable};
use beans_lang_java::parse_java_file;

const ANIMAL_SRC: &str = include_str!("../../tests/fixtures/java/com/example/Animal.java");
const DOG_SRC: &str = include_str!("../../tests/fixtures/java/com/example/Dog.java");
const KENNEL_SRC: &str = include_str!("../../tests/fixtures/java/com/example/Kennel.java");

fn build_table() -> SymbolTable {
    let mut table = SymbolTable::new();

    let animal_path = Path::new("com/example/Animal.java");
    let dog_path = Path::new("com/example/Dog.java");
    let kennel_path = Path::new("com/example/Kennel.java");

    // Parse files sequentially. For the first file, offset is 0 (local IDs match global IDs).
    let animal_symbols = parse_java_file(animal_path, ANIMAL_SRC);
    let animal_count = animal_symbols.len();
    for sym in animal_symbols {
        table.insert(sym);
    }

    let dog_symbols = parse_java_file(dog_path, DOG_SRC);
    let dog_count = dog_symbols.len();
    for mut sym in dog_symbols {
        if let Some(ref mut parent) = sym.parent {
            parent.0 += animal_count;
        }
        table.insert(sym);
    }

    let kennel_symbols = parse_java_file(kennel_path, KENNEL_SRC);
    for mut sym in kennel_symbols {
        if let Some(ref mut parent) = sym.parent {
            parent.0 += animal_count + dog_count;
        }
        table.insert(sym);
    }

    table
}

#[test]
fn test_animal_interface_by_fqn() {
    let table = build_table();

    let animal_id = table.lookup_by_fqn("com.example.Animal");
    assert!(animal_id.is_some(), "Should find Animal by FQN");

    let animal = table.get(animal_id.unwrap()).unwrap();
    assert_eq!(animal.name, "Animal");
    assert_eq!(animal.kind, SymbolKind::Interface);
}

#[test]
fn test_animal_has_methods() {
    let table = build_table();

    let animal_id = table.lookup_by_fqn("com.example.Animal").unwrap();
    let children = table.lookup_children(animal_id);
    assert_eq!(
        children.len(),
        2,
        "Animal should have 2 methods (getName, makeSound)"
    );

    let child_names: Vec<&str> = children
        .iter()
        .filter_map(|id| table.get(*id))
        .map(|s| s.name.as_str())
        .collect();
    assert!(
        child_names.contains(&"getName"),
        "Animal should have getName method"
    );
    assert!(
        child_names.contains(&"makeSound"),
        "Animal should have makeSound method"
    );
}

#[test]
fn test_dog_class_by_fqn() {
    let table = build_table();

    let dog_id = table.lookup_by_fqn("com.example.Dog");
    assert!(dog_id.is_some(), "Should find Dog by FQN");

    let dog = table.get(dog_id.unwrap()).unwrap();
    assert_eq!(dog.name, "Dog");
    assert_eq!(dog.kind, SymbolKind::Class);
}

#[test]
fn test_dog_has_all_members() {
    let table = build_table();

    let dog_id = table.lookup_by_fqn("com.example.Dog").unwrap();
    let children = table.lookup_children(dog_id);

    // Dog should have: 2 fields (name, age) + 1 constructor + 3 methods (getName, makeSound, getAge)
    assert!(
        children.len() >= 6,
        "Dog should have at least 6 members (2 fields + 1 constructor + 3 methods), got {}",
        children.len()
    );

    let child_names: Vec<&str> = children
        .iter()
        .filter_map(|id| table.get(*id))
        .map(|s| s.name.as_str())
        .collect();

    assert!(
        child_names.contains(&"name"),
        "Dog should have 'name' field"
    );
    assert!(child_names.contains(&"age"), "Dog should have 'age' field");
    assert!(
        child_names.contains(&"getName"),
        "Dog should have getName method"
    );
    assert!(
        child_names.contains(&"makeSound"),
        "Dog should have makeSound method"
    );
    assert!(
        child_names.contains(&"getAge"),
        "Dog should have getAge method"
    );
}

#[test]
fn test_dog_field_types() {
    let table = build_table();

    let name_id = table.lookup_by_fqn("com.example.Dog.name").unwrap();
    let name_sym = table.get(name_id).unwrap();
    assert_eq!(name_sym.kind, SymbolKind::Field);
    if let Some(beans_core::Signature::Field { ref field_type }) = name_sym.signature {
        assert_eq!(field_type, "String");
    } else {
        panic!("Expected Field signature for Dog.name");
    }

    let age_id = table.lookup_by_fqn("com.example.Dog.age").unwrap();
    let age_sym = table.get(age_id).unwrap();
    assert_eq!(age_sym.kind, SymbolKind::Field);
    if let Some(beans_core::Signature::Field { ref field_type }) = age_sym.signature {
        assert_eq!(field_type, "int");
    } else {
        panic!("Expected Field signature for Dog.age");
    }
}

#[test]
fn test_dog_constructor() {
    let table = build_table();

    let dog_id = table.lookup_by_fqn("com.example.Dog").unwrap();
    let children = table.lookup_children(dog_id);

    let constructors: Vec<_> = children
        .iter()
        .filter_map(|id| table.get(*id))
        .filter(|s| s.kind == SymbolKind::Constructor)
        .collect();

    assert_eq!(
        constructors.len(),
        1,
        "Dog should have exactly 1 constructor"
    );
    let ctor = constructors[0];
    assert_eq!(ctor.fqn, "com.example.Dog.Dog");

    if let Some(beans_core::Signature::Method {
        ref parameters, ..
    }) = ctor.signature
    {
        assert_eq!(
            parameters.len(),
            2,
            "Dog constructor should have 2 parameters"
        );
        assert_eq!(parameters[0].name, "name");
        assert_eq!(parameters[0].param_type, "String");
        assert_eq!(parameters[1].name, "age");
        assert_eq!(parameters[1].param_type, "int");
    } else {
        panic!("Expected Method signature on constructor");
    }
}

#[test]
fn test_kennel_by_fqn() {
    let table = build_table();

    let kennel_id = table.lookup_by_fqn("com.example.Kennel");
    assert!(kennel_id.is_some(), "Should find Kennel by FQN");

    let kennel = table.get(kennel_id.unwrap()).unwrap();
    assert_eq!(kennel.name, "Kennel");
    assert_eq!(kennel.kind, SymbolKind::Class);
}

#[test]
fn test_kennel_has_fields_and_methods() {
    let table = build_table();

    let kennel_id = table.lookup_by_fqn("com.example.Kennel").unwrap();
    let children = table.lookup_children(kennel_id);

    let child_names: Vec<&str> = children
        .iter()
        .filter_map(|id| table.get(*id))
        .map(|s| s.name.as_str())
        .collect();

    assert!(
        child_names.contains(&"dogs"),
        "Kennel should have 'dogs' field"
    );
    assert!(
        child_names.contains(&"addDog"),
        "Kennel should have addDog method"
    );
    assert!(
        child_names.contains(&"findDog"),
        "Kennel should have findDog method"
    );
}

#[test]
fn test_lookup_by_simple_name() {
    let table = build_table();

    let animals = table.lookup_by_name("Animal");
    assert!(!animals.is_empty(), "Should find Animal by simple name");
    let animal = table.get(animals[0]).unwrap();
    assert_eq!(animal.fqn, "com.example.Animal");

    let dogs = table.lookup_by_name("Dog");
    assert!(!dogs.is_empty(), "Should find Dog by simple name");
}

#[test]
fn test_lookup_by_package() {
    let table = build_table();

    let pkg_symbols = table.lookup_by_package("com.example");
    // Should find at least the 3 top-level types + their members
    assert!(
        pkg_symbols.len() >= 3,
        "Should find at least 3 symbols in com.example, got {}",
        pkg_symbols.len()
    );

    let fqns: Vec<&str> = pkg_symbols
        .iter()
        .filter_map(|id| table.get(*id))
        .map(|s| s.fqn.as_str())
        .collect();
    assert!(
        fqns.contains(&"com.example.Animal"),
        "Package should contain Animal"
    );
    assert!(
        fqns.contains(&"com.example.Dog"),
        "Package should contain Dog"
    );
    assert!(
        fqns.contains(&"com.example.Kennel"),
        "Package should contain Kennel"
    );
}

#[test]
fn test_lookup_by_kind_interface() {
    let table = build_table();

    let interfaces = table.lookup_by_kind(SymbolKind::Interface);
    assert!(!interfaces.is_empty(), "Should find at least one interface");

    let interface_names: Vec<&str> = interfaces
        .iter()
        .filter_map(|id| table.get(*id))
        .map(|s| s.name.as_str())
        .collect();
    assert!(
        interface_names.contains(&"Animal"),
        "Animal should be an interface"
    );
}

#[test]
fn test_lookup_by_kind_class() {
    let table = build_table();

    let classes = table.lookup_by_kind(SymbolKind::Class);
    assert!(
        classes.len() >= 2,
        "Should find at least 2 classes (Dog, Kennel)"
    );

    let class_names: Vec<&str> = classes
        .iter()
        .filter_map(|id| table.get(*id))
        .map(|s| s.name.as_str())
        .collect();
    assert!(class_names.contains(&"Dog"));
    assert!(class_names.contains(&"Kennel"));
}

#[test]
fn test_lookup_by_file() {
    let table = build_table();

    let dog_syms = table.lookup_by_file(Path::new("com/example/Dog.java"));
    assert!(
        dog_syms.len() >= 6,
        "Dog.java should have at least 6 symbols, got {}",
        dog_syms.len()
    );

    let animal_syms = table.lookup_by_file(Path::new("com/example/Animal.java"));
    assert!(
        animal_syms.len() >= 3,
        "Animal.java should have at least 3 symbols (interface + 2 methods), got {}",
        animal_syms.len()
    );
}

#[test]
fn test_cross_file_symbol_coexistence() {
    let table = build_table();

    // All three types should coexist in the same symbol table
    assert!(table.lookup_by_fqn("com.example.Animal").is_some());
    assert!(table.lookup_by_fqn("com.example.Dog").is_some());
    assert!(table.lookup_by_fqn("com.example.Kennel").is_some());

    // Members from different files should all be accessible
    assert!(table.lookup_by_fqn("com.example.Animal.getName").is_some());
    assert!(table.lookup_by_fqn("com.example.Dog.getName").is_some());
    assert!(table.lookup_by_fqn("com.example.Kennel.addDog").is_some());
}

#[test]
fn test_method_return_types() {
    let table = build_table();

    let get_name_id = table.lookup_by_fqn("com.example.Dog.getName").unwrap();
    let get_name = table.get(get_name_id).unwrap();
    if let Some(beans_core::Signature::Method {
        ref return_type, ..
    }) = get_name.signature
    {
        assert_eq!(return_type, "String");
    } else {
        panic!("Expected Method signature for Dog.getName");
    }

    let get_age_id = table.lookup_by_fqn("com.example.Dog.getAge").unwrap();
    let get_age = table.get(get_age_id).unwrap();
    if let Some(beans_core::Signature::Method {
        ref return_type, ..
    }) = get_age.signature
    {
        assert_eq!(return_type, "int");
    } else {
        panic!("Expected Method signature for Dog.getAge");
    }
}

#[test]
fn test_method_parameters() {
    let table = build_table();

    let add_dog_id = table.lookup_by_fqn("com.example.Kennel.addDog").unwrap();
    let add_dog = table.get(add_dog_id).unwrap();
    if let Some(beans_core::Signature::Method {
        ref parameters, ..
    }) = add_dog.signature
    {
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].name, "dog");
        assert_eq!(parameters[0].param_type, "Dog");
    } else {
        panic!("Expected Method signature for Kennel.addDog");
    }

    let find_dog_id = table.lookup_by_fqn("com.example.Kennel.findDog").unwrap();
    let find_dog = table.get(find_dog_id).unwrap();
    if let Some(beans_core::Signature::Method {
        ref return_type,
        ref parameters,
        ..
    }) = find_dog.signature
    {
        assert_eq!(return_type, "Dog");
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].name, "name");
        assert_eq!(parameters[0].param_type, "String");
    } else {
        panic!("Expected Method signature for Kennel.findDog");
    }
}

#[test]
fn test_incremental_reindex() {
    let mut table = SymbolTable::new();
    let dog_path = Path::new("com/example/Dog.java");

    // Initial parse (first file, no offset needed)
    let symbols = parse_java_file(dog_path, DOG_SRC);
    for sym in symbols {
        table.insert(sym);
    }
    assert!(table.lookup_by_fqn("com.example.Dog").is_some());
    assert!(table.lookup_by_fqn("com.example.Dog.getAge").is_some());

    // Remove and re-parse with modified content (removed getAge)
    table.remove_by_file(dog_path);
    assert!(table.lookup_by_fqn("com.example.Dog").is_none());

    let modified_src = r#"package com.example;

public class Dog {
    private String name;

    public String getName() {
        return name;
    }
}
"#;
    let symbols = parse_java_file(dog_path, modified_src);
    for sym in symbols {
        table.insert(sym);
    }
    assert!(table.lookup_by_fqn("com.example.Dog").is_some());
    assert!(table.lookup_by_fqn("com.example.Dog.getName").is_some());
    assert!(
        table.lookup_by_fqn("com.example.Dog.getAge").is_none(),
        "getAge should be gone after reindex"
    );
}

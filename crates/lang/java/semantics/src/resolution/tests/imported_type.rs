use super::{ResolutionResult, lookup_field};

#[test]
fn single_import_lookup_ignores_on_demand_imports() {
    for import in ["import p.*;", "import static p.Container.*;"] {
        let source = format!("{import} class Use {{ Outer.Inner target; }}");
        let result = lookup_field(&source, |instance, _| {
            instance.find_single_imported_type(instance.type_ref)
        });

        assert!(matches!(result, ResolutionResult::NotFound));
    }
}

#[test]
fn single_imports_must_match_the_first_reference_component() {
    for import in ["import p.Inner;", "import static p.Container.Inner;"] {
        let source = format!("{import} class Use {{ Outer.Inner target; }}");
        let result = lookup_field(&source, |instance, _| {
            instance.find_single_imported_type(instance.type_ref)
        });

        assert!(matches!(result, ResolutionResult::NotFound));
    }
}

#[test]
fn single_import_lookup_leaves_non_named_references_to_other_resolution() {
    for field_type in ["int", "Outer[]"] {
        let source = format!("import p.Outer; import p.*; class Use {{ {field_type} target; }}");
        lookup_field(&source, |instance, _| {
            assert!(matches!(
                instance.find_single_imported_type(instance.type_ref),
                ResolutionResult::NotFound
            ));
            ResolutionResult::NotFound
        });
    }
}

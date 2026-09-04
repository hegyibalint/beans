use crate::lower_into;
use beans_lang_java_model::{
    imports::{Import, ImportType},
    references::NameRef,
};

fn qualified_name(components: &[&str]) -> NameRef {
    NameRef::Qualified(
        components
            .iter()
            .map(|component| (*component).to_owned())
            .collect(),
    )
}

#[test]
fn one_import_is_preserved() {
    let file = lower_into("import example.Item;");

    assert_eq!(
        file.imports,
        [Import::new(
            qualified_name(&["example", "Item"]),
            ImportType::SingleType,
        )]
    );
}

#[test]
fn two_imports_preserve_source_order() {
    let file = lower_into("import first.One; import second.Two;");

    assert_eq!(
        file.imports,
        [
            Import::new(qualified_name(&["first", "One"]), ImportType::SingleType,),
            Import::new(qualified_name(&["second", "Two"]), ImportType::SingleType,),
        ]
    );
}

#[test]
fn supported_import_kinds_are_distinguished() {
    let file = lower_into(
        "import types.Single;
         import types.*;
         import static members.Owner.VALUE;
         import static members.Owner.*;",
    );

    assert_eq!(
        file.imports,
        [
            Import::new(qualified_name(&["types", "Single"]), ImportType::SingleType,),
            Import::new(
                NameRef::Simple("types".to_owned()),
                ImportType::OnDemandType,
            ),
            Import::new(
                qualified_name(&["members", "Owner", "VALUE"]),
                ImportType::SingleStaticType,
            ),
            Import::new(
                qualified_name(&["members", "Owner"]),
                ImportType::OnDemandStaticType,
            ),
        ]
    );
}

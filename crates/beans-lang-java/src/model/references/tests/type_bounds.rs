use crate::model::references::{BoundKind, TypeBound};

#[test]
fn unbounded_has_no_types() {
    let bound = TypeBound::<u8>::new_unbounded();

    assert_eq!(bound.kind(), BoundKind::Unbounded);
    assert_eq!(bound.primary(), None);
    assert!(bound.additional().is_empty());
    assert!(bound.types().is_empty());
}

#[test]
fn exact_has_only_a_primary_type() {
    let bound = TypeBound::new_exact(1);

    assert_eq!(bound.kind(), BoundKind::Exact);
    assert_eq!(bound.primary(), Some(&1));
    assert!(bound.additional().is_empty());
    assert_eq!(bound.types(), &[1]);
}

#[test]
fn super_has_only_a_primary_type() {
    let bound = TypeBound::new_super(1);

    assert_eq!(bound.kind(), BoundKind::Super);
    assert_eq!(bound.primary(), Some(&1));
    assert!(bound.additional().is_empty());
    assert_eq!(bound.types(), &[1]);
}

#[test]
fn extends_preserves_primary_and_zero_one_or_two_additional_types() {
    for additional in [vec![], vec![2], vec![2, 3]] {
        let bound = TypeBound::new_extends(1, additional.clone());

        assert_eq!(bound.kind(), BoundKind::Extends);
        assert_eq!(bound.primary(), Some(&1));
        assert_eq!(bound.additional(), additional);
        assert_eq!(bound.types().len(), additional.len() + 1);
        assert_eq!(bound.types()[0], 1);
        assert_eq!(&bound.types()[1..], additional);
    }
}

#[test]
fn mapping_preserves_kind_and_visits_every_type_once_in_order() {
    for bound in [
        TypeBound::new_unbounded(),
        TypeBound::new_exact(1),
        TypeBound::new_super(1),
        TypeBound::new_extends(1, vec![]),
        TypeBound::new_extends(1, vec![2]),
        TypeBound::new_extends(1, vec![2, 3]),
    ] {
        let mut visited = Vec::new();
        let mapped = bound.map_ref(|value| {
            visited.push(*value);
            value.to_string()
        });

        assert_eq!(visited.as_slice(), bound.types());
        assert_eq!(mapped.kind(), bound.kind());
        assert_eq!(mapped.types().len(), bound.types().len());
        for (original, transformed) in bound.types().iter().zip(mapped.types()) {
            assert_eq!(transformed, &original.to_string());
        }
    }
}

#[test]
fn mapping_unbounded_does_not_call_the_transformation() {
    let mapped: TypeBound<String> = TypeBound::<u8>::new_unbounded()
        .map_ref(|_| panic!("an unbounded wildcard has no reference to transform"));

    assert_eq!(mapped.kind(), BoundKind::Unbounded);
    assert!(mapped.types().is_empty());
}

#[test]
fn mapping_borrows_without_requiring_clone_or_consuming_the_source() {
    struct Value(u8);
    let bound = TypeBound::new_extends(Value(1), vec![Value(2)]);

    let mapped = bound.map_ref(|value| value.0);

    assert_eq!(mapped.types(), &[1, 2]);
    assert_eq!(bound.primary().unwrap().0, 1);
    assert_eq!(bound.additional()[0].0, 2);
}

#[test]
fn mapping_to_collections_does_not_flatten_the_payloads() {
    let bound = TypeBound::new_extends(1, vec![2, 3]);

    let mapped = bound.map_ref(|value| vec![*value, value + 10]);

    assert_eq!(mapped.primary(), Some(&vec![1, 11]));
    assert_eq!(mapped.additional(), &[vec![2, 12], vec![3, 13]]);
    assert_eq!(mapped.types().len(), 3);
}

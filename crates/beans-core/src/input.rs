use crate::location::Location;

#[derive(Debug, Clone)]
pub struct Input<'a> {
    pub resource: Location,
    pub contents: &'a [u8],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::LocationEntry;

    #[test]
    fn different_contents_can_belong_to_the_same_resource() {
        let resource = Location::new(LocationEntry::File("/src/Example.java".into()));
        let before = Input {
            resource: resource.clone(),
            contents: b"class Example {}",
        };
        let after = Input {
            resource,
            contents: b"class Example { int value; }",
        };
        assert_eq!(before.resource, after.resource);
        assert_ne!(before.contents, after.contents);
    }

    #[test]
    fn nested_resources_can_carry_binary_contents() {
        let contents = [0xca, 0xfe, 0xba, 0xbe];
        let input = Input {
            resource: Location::new(LocationEntry::File("/library.jar".into()))
                .entry(LocationEntry::Container("Example.class".into())),
            contents: &contents,
        };
        assert_eq!(input.contents.as_ptr(), contents.as_ptr());
    }
}

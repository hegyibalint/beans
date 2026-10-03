use std::hash::{Hash, Hasher};

/// A half-open range of byte offsets in source content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    start: usize,
    end: usize,
}

impl ByteRange {
    pub const fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "a byte range cannot end before it starts");
        Self { start, end }
    }

    pub const fn start(self) -> usize {
        self.start
    }

    pub const fn end(self) -> usize {
        self.end
    }

    pub const fn contains(self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }
}

/// A semantic value annotated with its source range.
///
/// Equality and hashing use only the value; compare `range()` explicitly when
/// source position is part of the question.
#[derive(Debug, Clone, Copy)]
pub struct Spanned<T> {
    value: T,
    range: ByteRange,
}

impl<T> Spanned<T> {
    pub const fn new(value: T, range: ByteRange) -> Self {
        Self { value, range }
    }

    pub const fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub const fn range(&self) -> ByteRange {
        self.range
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for Spanned<T> {}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteRange, Spanned};

    #[test]
    fn ranges_include_their_start_and_exclude_their_end() {
        let range = ByteRange::new(3, 7);

        assert!(!range.contains(2));
        assert!(range.contains(3));
        assert!(range.contains(6));
        assert!(!range.contains(7));
        assert_eq!(range.start(), 3);
        assert_eq!(range.end(), 7);
    }

    #[test]
    fn empty_ranges_contain_no_offsets() {
        let range = ByteRange::new(5, 5);

        assert!(!range.contains(4));
        assert!(!range.contains(5));
        assert!(!range.contains(6));
    }

    #[test]
    #[should_panic(expected = "a byte range cannot end before it starts")]
    fn ranges_reject_reversed_boundaries() {
        ByteRange::new(8, 7);
    }

    #[test]
    fn spanned_values_expose_their_value_and_range() {
        let value = Spanned::new("name", ByteRange::new(3, 7));

        assert_eq!(value.value(), &"name");
        assert_eq!(value.range(), ByteRange::new(3, 7));
    }

    #[test]
    fn spanned_value_equality_ignores_source_position() {
        let first = Spanned::new("name", ByteRange::new(3, 7));
        let second = Spanned::new("name", ByteRange::new(20, 24));

        assert_eq!(first, second);
        assert_ne!(first.range(), second.range());
    }
}

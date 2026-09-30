use super::source::Source;

/// Determines which stored sources are visible to a query.
pub trait Classpath {
    fn contains(&self, source: &Source) -> bool;
}

/// Unscoped queries see every stored source.
pub struct Unrestricted;

impl Classpath for Unrestricted {
    fn contains(&self, _source: &Source) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrestricted_includes_source_files_virtual_documents_and_binary_origins() {
        let classpath = Unrestricted;

        assert!(classpath.contains(&Source::uri("file:///src/Example.java")));
        assert!(classpath.contains(&Source::uri("untitled:Example.java")));
        assert!(classpath.contains(&Source::jar_entry("library.jar", "p/Example.class")));
    }
}

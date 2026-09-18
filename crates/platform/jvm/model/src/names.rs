use std::fmt;

/// A class or interface binary name in the external form defined by JLS §13.1.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BinaryName(String);

impl BinaryName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BinaryName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::BinaryName;

    #[test]
    fn binary_names_use_the_jls_external_form() {
        for name in ["Example", "java.lang.String", "example.Outer$Member"] {
            let binary_name = BinaryName::new(name);

            assert_eq!(binary_name.as_str(), name);
            assert_eq!(binary_name.to_string(), name);
        }
    }
}

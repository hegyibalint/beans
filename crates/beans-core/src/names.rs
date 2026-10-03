use std::fmt;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Name {
    components: Vec<String>,
}

impl Name {
    pub fn new(components: Vec<String>) -> Self {
        Self { components }
    }

    pub fn as_slice(&self) -> &[String] {
        &self.components
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Checks for nonempty components, not identifier syntax or target validity.
    pub fn is_valid(&self) -> bool {
        self.components
            .iter()
            .all(|component| !component.is_empty())
    }
}

impl FromIterator<String> for Name {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, component) in self.components.iter().enumerate() {
            if index > 0 {
                formatter.write_str(".")?;
            }
            formatter.write_str(component)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Name;

    #[test]
    fn an_empty_name_is_valid_and_has_no_components() {
        let name = Name::default();

        assert!(name.is_valid());
        assert!(name.is_empty());
        assert_eq!(name.len(), 0);
        assert_eq!(name.to_string(), "");
    }

    #[test]
    fn nonempty_components_form_valid_names() {
        for components in [vec!["p"], vec!["p", "q"], vec!["p", "Outer", "Member"]] {
            let name: Name = components
                .iter()
                .map(|component| (*component).to_owned())
                .collect();

            assert!(name.is_valid());
            assert!(!name.is_empty());
            assert_eq!(name.len(), components.len());
            assert_eq!(name.as_slice(), components);
        }
    }

    #[test]
    fn empty_components_are_invalid_but_preserved() {
        for components in [vec![""], vec!["", "p"], vec!["p", "", "q"], vec!["p", ""]] {
            let name = Name::new(
                components
                    .iter()
                    .map(|component| (*component).to_owned())
                    .collect(),
            );

            assert!(!name.is_valid());
            assert_eq!(name.as_slice(), components);
        }
    }

    #[test]
    fn display_separates_components_without_sanitizing_them() {
        for text in ["p", "p.Outer.Member", ".p", "p..Outer", "p."] {
            let name: Name = text.split('.').map(str::to_owned).collect();

            assert_eq!(name.to_string(), text);
        }
    }
}

use crate::{analysis::FileAnalysis, model::Offset, model::OffsetSpan, storage::Revision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationTarget<Source> {
    pub source: Source,
    pub span: OffsetSpan,
}

pub trait LanguageProcessing<Source, Platform> {
    fn accepts(&self, source: &Source) -> bool;

    fn process(
        &mut self,
        source: Source,
        revision: Revision,
        platform: &mut Platform,
        contents: &str,
    );
}

pub trait Language<Source, Platform>: LanguageProcessing<Source, Platform> {
    fn analyze(
        &self,
        _source: &Source,
        _revision: Revision,
        _platform: &Platform,
    ) -> Option<FileAnalysis> {
        None
    }

    fn find_declarations_for(
        &self,
        _source: &Source,
        _offset: Offset,
        _revision: Revision,
        _platform: &Platform,
    ) -> Option<Vec<NavigationTarget<Source>>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestLanguage;

    impl LanguageProcessing<String, ()> for TestLanguage {
        fn accepts(&self, source: &String) -> bool {
            source.ends_with(".test")
        }

        fn process(
            &mut self,
            _source: String,
            _revision: Revision,
            _platform: &mut (),
            _contents: &str,
        ) {
        }
    }

    impl Language<String, ()> for TestLanguage {
        fn find_declarations_for(
            &self,
            source: &String,
            offset: Offset,
            _revision: Revision,
            _platform: &(),
        ) -> Option<Vec<NavigationTarget<String>>> {
            Some(vec![NavigationTarget {
                source: source.clone(),
                span: OffsetSpan {
                    start: offset,
                    end: offset,
                },
            }])
        }
    }

    #[test]
    fn languages_override_optional_operations() {
        let languages: Vec<Box<dyn Language<String, ()>>> = vec![Box::new(TestLanguage)];
        let language = &languages[0];

        let source = "example.test".to_string();
        assert!(language.accepts(&source));
        assert!(
            language
                .analyze(&source, Revision::default(), &())
                .is_none()
        );
        assert_eq!(
            language
                .find_declarations_for(&source, Offset(4), Revision::default(), &())
                .unwrap()
                .len(),
            1
        );
    }
}

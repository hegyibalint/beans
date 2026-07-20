use crate::{analysis::FileAnalysis, model::Span, storage::Revision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationTarget<Source> {
    pub source: Source,
    pub span: Span,
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

pub trait LanguageAnalysis<Source, Platform> {
    fn analyze(
        &self,
        source: &Source,
        revision: Revision,
        platform: &Platform,
    ) -> Option<FileAnalysis>;
}

pub trait LanguageNavigation<Source, Platform> {
    fn find_declaration_for(
        &self,
        source: &Source,
        offset: usize,
        revision: Revision,
        platform: &Platform,
    ) -> Option<NavigationTarget<Source>>;
}

pub trait LanguageCompletion<Source, Platform> {}

pub trait LanguageSymbols<Source, Platform> {}

pub trait LanguageRefactoring<Source, Platform> {}

pub trait LanguageFormatting<Source, Platform> {}

pub trait Language<Source, Platform>: LanguageProcessing<Source, Platform> {
    fn analysis(&self) -> Option<&dyn LanguageAnalysis<Source, Platform>> {
        None
    }

    fn navigation(&self) -> Option<&dyn LanguageNavigation<Source, Platform>> {
        None
    }

    fn completion(&self) -> Option<&dyn LanguageCompletion<Source, Platform>> {
        None
    }

    fn symbols(&self) -> Option<&dyn LanguageSymbols<Source, Platform>> {
        None
    }

    fn refactoring(&self) -> Option<&dyn LanguageRefactoring<Source, Platform>> {
        None
    }

    fn formatting(&self) -> Option<&dyn LanguageFormatting<Source, Platform>> {
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

    impl LanguageAnalysis<String, ()> for TestLanguage {
        fn analyze(
            &self,
            _source: &String,
            _revision: Revision,
            _platform: &(),
        ) -> Option<FileAnalysis> {
            None
        }
    }

    impl LanguageNavigation<String, ()> for TestLanguage {
        fn find_declaration_for(
            &self,
            source: &String,
            offset: usize,
            _revision: Revision,
            _platform: &(),
        ) -> Option<NavigationTarget<String>> {
            Some(NavigationTarget {
                source: source.clone(),
                span: Span {
                    start: offset,
                    end: offset,
                },
            })
        }
    }

    impl Language<String, ()> for TestLanguage {
        fn analysis(&self) -> Option<&dyn LanguageAnalysis<String, ()>> {
            Some(self)
        }

        fn navigation(&self) -> Option<&dyn LanguageNavigation<String, ()>> {
            Some(self)
        }
    }

    #[test]
    fn languages_expose_supported_capabilities() {
        let languages: Vec<Box<dyn Language<String, ()>>> = vec![Box::new(TestLanguage)];
        let language = &languages[0];

        assert!(language.accepts(&"example.test".to_string()));
        assert!(language.analysis().is_some());
        assert!(language.navigation().is_some());
        assert!(language.completion().is_none());
    }
}

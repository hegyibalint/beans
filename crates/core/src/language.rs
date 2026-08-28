use crate::{analysis::FileAnalysis, model::Offset, model::OffsetSpan, storage::Revision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationTarget<Source> {
    pub source: Source,
    pub span: OffsetSpan,
}

/// One row a client may offer the user. Everything a list needs to render is
/// here; the handle is for the second half of the exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem<Source> {
    /// What the row reads as. Not what accepting it writes: a Java method is
    /// shown with its parameters, `describe(int factor)`, and inserts a bare
    /// name.
    pub label: String,
    /// What accepting the row writes over `replace`. Separate from the label
    /// because the two diverge as soon as a row says more than it inserts.
    pub insert: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    /// What accepting this row overwrites.
    pub replace: OffsetSpan,
    pub handle: Option<Handle<Source>>,
}

impl<Source> CompletionItem<Source> {
    /// A row that reads exactly as it writes, which is every namespace but
    /// methods.
    pub fn plain(
        name: String,
        kind: CompletionItemKind,
        detail: Option<String>,
        replace: OffsetSpan,
        handle: Option<Handle<Source>>,
    ) -> CompletionItem<Source> {
        CompletionItem {
            insert: name.clone(),
            label: name,
            kind,
            detail,
            replace,
            handle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionItemKind {
    Class,
    Interface,
    Enum,
    Record,
    AnnotationInterface,
    Method,
    Field,
    Variable,
    Parameter,
    TypeParameter,
}

/// A note a language vertical writes to itself, handed out with an item and
/// given back untouched when the client asks for more about that one row.
///
/// The payload is opaque here on purpose. The JVM is the only vocabulary its
/// languages share, and above that line identity does not unify: a Clojure
/// `deftype` has a binary name, while a `defn` has a var with no type and no
/// member to hang it on. So only the vertical that minted a payload ever reads
/// it, and this crate carries it without looking inside.
///
/// The two fields that are not opaque are the two questions `core` already
/// answers. `source` routes, the way `LanguageProcessing::accepts` routes
/// everything else. `revision` pins: `RevisionedStorage` keeps history, so a
/// handle never goes stale — it is either valid, or invalid because the source
/// did not exist then, was deleted by then, or names nothing in that model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handle<Source> {
    pub source: Source,
    pub revision: Revision,
    pub payload: String,
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

    /// The names in scope at `offset`. Takes the text because a caret is a
    /// lexical position before it is a semantic one: the prefix behind it and
    /// the character in front of it are both facts about characters, and no
    /// model holds them.
    fn complete_at(
        &self,
        _source: &Source,
        _offset: Offset,
        _revision: Revision,
        _platform: &Platform,
        _contents: &str,
    ) -> Option<Vec<CompletionItem<Source>>> {
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

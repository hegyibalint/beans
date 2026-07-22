use beans_core::{
    analysis::FileAnalysis,
    file::TextFile,
    language::{Language, LanguageProcessing, NavigationTarget},
    model::{LineColumnPosition, LineColumnSpan, Offset, OffsetSpan},
    storage::{Revision, RevisionedStorage},
};
use beans_lang_java::LanguageJava;
use beans_platform_jvm::{PlatformJvm, model::JvmSource};

pub struct Beans {
    revision: Revision,
    platform_jvm: PlatformJvm,
    lang_java: LanguageJava,
    /// Text of record for every processed source, independent of any parse.
    /// The sole substrate for byte-offset ↔ line/column translation.
    text_files: RevisionedStorage<JvmSource, TextFile>,
}

impl Beans {
    pub fn new() -> Beans {
        Beans {
            revision: Revision::default(),
            platform_jvm: PlatformJvm::new(),
            lang_java: LanguageJava::new(),
            text_files: RevisionedStorage::new(),
        }
    }
}

impl Beans {
    pub fn process(&mut self, source: JvmSource, contents: &str) {
        let revision = self.revision.bump();

        // Text is language-agnostic: store it for every source so coordinates
        // resolve even for files no language claims.
        self.text_files
            .put(revision, source.clone(), TextFile::new(contents));

        if self.lang_java.accepts(&source) {
            self.lang_java
                .process(source, revision, &mut self.platform_jvm, contents);
        }
    }

    /// `None` when no language claims the source; the editor sends us
    /// all kinds of files, and skipping them is not an error.
    pub fn analyze(&self, source: &JvmSource) -> Option<FileAnalysis> {
        if self.lang_java.accepts(source) {
            return self
                .lang_java
                .analyze(source, self.revision, &self.platform_jvm);
        }

        None
    }

    pub fn find_declarations_for(
        &self,
        source: &JvmSource,
        offset: Offset,
    ) -> Option<Vec<NavigationTarget<JvmSource>>> {
        if self.lang_java.accepts(source) {
            return self.lang_java.find_declarations_for(
                source,
                offset,
                self.revision,
                &self.platform_jvm,
            );
        }

        None
    }

    /// A display name for the declaration whose name sits at `span`,
    /// e.g. `p.Outer.Inner` for a member type.
    pub fn declaration_label(&self, source: &JvmSource, span: OffsetSpan) -> Option<String> {
        if self.lang_java.accepts(source) {
            return self
                .lang_java
                .declaration_label(source, span, self.revision);
        }

        None
    }

    /// Ingress: the line/column an editor sends us becomes a byte offset.
    /// `None` if the file is unknown or the position lands outside it.
    pub fn offset_at(&self, source: &JvmSource, position: LineColumnPosition) -> Option<Offset> {
        self.text_files.get(source, self.revision)?.offset(position)
    }

    /// Egress: a byte span becomes line/column. The file need not be open —
    /// the range comes from that file's stored text, so a navigation target
    /// in an unopened file still ranges correctly.
    pub fn text_range(&self, source: &JvmSource, span: OffsetSpan) -> Option<LineColumnSpan> {
        Some(
            self.text_files
                .get(source, self.revision)?
                .line_column_span(span),
        )
    }
}

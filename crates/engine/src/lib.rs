use beans_core::{
    analysis::FileAnalysis,
    file::TextFile,
    language::{Language, LanguageProcessing, NavigationTarget},
    model::{LineColumnPosition, LineColumnSpan, Offset, OffsetSpan},
    storage::{Revision, RevisionedStorage},
};
use beans_lang_java::LanguageJava;
use beans_platform_jvm::{PlatformJvm, model::JvmSource};
use beans_workspace::model::Workspace;
use std::fs;
use std::path::Path;

use crate::workspace::java_sources;

mod workspace;

pub use beans_workspace_beans::LoadError;

pub struct Beans {
    revision: Revision,
    /// What the project looks like: which units exist, what each one owns, and
    /// which others it depends on. Recorded and not yet consulted; resolution
    /// still runs unscoped, so every source sees the whole lake.
    workspace: Option<Workspace>,
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
            workspace: None,
            platform_jvm: PlatformJvm::new(),
            lang_java: LanguageJava::new(),
            text_files: RevisionedStorage::new(),
        }
    }
}

impl Beans {
    pub fn process(&mut self, source: JvmSource, contents: &str) {
        let revision = self.revision.bump();
        self.process_at(revision, source, contents);
    }

    /// Record what the project looks like, without reading a byte of it.
    ///
    /// Separate from `open_workspace` because knowing the structure and holding
    /// the sources are two different things: an editor hands us text for files
    /// it has open, a test holds its files in memory, and only a descriptor on
    /// disk needs both halves.
    pub fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = Some(workspace);
    }

    /// `None` until someone declares one. Absence is a project we know nothing
    /// about, which is every editor session without a descriptor.
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    /// Load everything `beans.toml` at `root` declares, if there is one, and
    /// answer how many sources that was. Without a descriptor we know nothing
    /// about the project and wait for an editor to hand us files, which is the
    /// behaviour this replaces.
    ///
    /// The whole batch shares one revision, so no query sees a project that is
    /// half loaded, and one load is one change rather than one per file.
    pub fn open_workspace(&mut self, root: &Path) -> Result<usize, LoadError> {
        let Some(workspace) = beans_workspace_beans::load(root)? else {
            return Ok(0);
        };

        // Listed before the workspace is handed over, so the borrow ends and
        // nothing has to be cloned.
        let paths = java_sources(&workspace);
        self.set_workspace(workspace);

        let revision = self.revision.bump();
        let mut loaded = 0;
        for path in paths {
            // A file that vanished between listing and reading is not worth
            // failing an import over.
            let Ok(contents) = fs::read_to_string(&path) else {
                continue;
            };
            self.process_at(revision, JvmSource::SourceFile { path }, &contents);
            loaded += 1;
        }

        Ok(loaded)
    }

    fn process_at(&mut self, revision: Revision, source: JvmSource, contents: &str) {
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

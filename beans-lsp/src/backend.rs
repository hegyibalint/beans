use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use beans_core::{SymbolId, SymbolKind, SymbolTable};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::resolve::{self, Import};
use crate::workspace;

pub struct ServerState {
    pub symbol_table: SymbolTable,
    pub file_imports: HashMap<PathBuf, Vec<Import>>,
    pub file_packages: HashMap<PathBuf, String>,
    /// Stores the latest content of open files (for getting text at cursor)
    pub open_files: HashMap<Url, String>,
    pub workspace_root: Option<PathBuf>,
}

impl ServerState {
    /// Re-index a file from in-memory content.
    pub fn reindex_content(&mut self, path: &std::path::Path, source: &str) {
        workspace::index_file_with_content(
            path,
            source,
            &mut self.symbol_table,
            &mut self.file_imports,
        );
        let pkg = workspace::extract_package(source);
        if !pkg.is_empty() {
            self.file_packages.insert(path.to_path_buf(), pkg);
        }
    }

    /// Re-index a file from disk.
    pub fn reindex_from_disk(&mut self, path: &std::path::Path) {
        workspace::index_file(path, &mut self.symbol_table, &mut self.file_imports);
        if let Ok(source) = std::fs::read_to_string(path) {
            let pkg = workspace::extract_package(&source);
            if !pkg.is_empty() {
                self.file_packages.insert(path.to_path_buf(), pkg);
            }
        }
    }
}

pub struct BeanBackend {
    client: Client,
    state: Arc<RwLock<ServerState>>,
}

impl BeanBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ServerState {
                symbol_table: SymbolTable::new(),
                file_imports: HashMap::new(),
                file_packages: HashMap::new(),
                open_files: HashMap::new(),
                workspace_root: None,
            })),
        }
    }
}

/// Get the word at a given position in a text document.
fn word_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    let target_line = text.lines().nth(line as usize)?;
    let col = character as usize;

    if col > target_line.len() {
        return None;
    }

    let bytes = target_line.as_bytes();

    // Find start of word
    let mut start = col;
    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }

    // Find end of word
    let mut end = col;
    while end < bytes.len() && is_identifier_char(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(target_line[start..end].to_string())
}

/// Get the compound expression at a position (e.g., "Type.method").
fn compound_at_position(text: &str, line: u32, character: u32) -> Option<String> {
    let target_line = text.lines().nth(line as usize)?;
    let col = character as usize;

    if col > target_line.len() {
        return None;
    }

    let bytes = target_line.as_bytes();

    // Find start of compound expression (identifiers + dots)
    let mut start = col;
    while start > 0 && (is_identifier_char(bytes[start - 1]) || bytes[start - 1] == b'.') {
        start -= 1;
    }

    // Find end of compound expression
    let mut end = col;
    while end < bytes.len() && (is_identifier_char(bytes[end]) || bytes[end] == b'.') {
        end += 1;
    }

    if start == end {
        return None;
    }

    let text = target_line[start..end].trim_matches('.');
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn is_identifier_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

fn uri_to_path(uri: &Url) -> Option<PathBuf> {
    uri.to_file_path().ok()
}

fn location_to_lsp(loc: &beans_core::Location) -> Option<Location> {
    let uri = Url::from_file_path(&loc.file).ok()?;
    Some(Location {
        uri,
        range: Range {
            start: Position {
                line: loc.start_line,
                character: loc.start_col,
            },
            end: Position {
                line: loc.end_line,
                character: loc.end_col,
            },
        },
    })
}

fn symbol_kind_to_lsp(kind: SymbolKind) -> SymbolKind2 {
    match kind {
        SymbolKind::Class | SymbolKind::DataClass | SymbolKind::SealedClass => {
            SymbolKind2::CLASS
        }
        SymbolKind::Interface | SymbolKind::Trait | SymbolKind::Protocol => {
            SymbolKind2::INTERFACE
        }
        SymbolKind::Enum | SymbolKind::CaseClass | SymbolKind::CaseObject => SymbolKind2::ENUM,
        SymbolKind::Record => SymbolKind2::STRUCT,
        SymbolKind::Annotation => SymbolKind2::CLASS,
        SymbolKind::Method | SymbolKind::Function | SymbolKind::Multimethod => {
            SymbolKind2::METHOD
        }
        SymbolKind::Constructor => SymbolKind2::CONSTRUCTOR,
        SymbolKind::Field | SymbolKind::EnumConstant => SymbolKind2::FIELD,
        SymbolKind::Parameter => SymbolKind2::VARIABLE,
        SymbolKind::Package | SymbolKind::Namespace => SymbolKind2::NAMESPACE,
        SymbolKind::Object | SymbolKind::CompanionObject => SymbolKind2::OBJECT,
        SymbolKind::Defrecord | SymbolKind::Deftype => SymbolKind2::CLASS,
    }
}

// Alias for the LSP SymbolKind to avoid confusion with beans_core::SymbolKind
use tower_lsp::lsp_types::SymbolKind as SymbolKind2;

#[tower_lsp::async_trait]
impl LanguageServer for BeanBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                self.state.write().await.workspace_root = Some(path);
            }
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Beans LSP server initialized");

        // Index workspace
        self.client
            .log_message(MessageType::INFO, "Beans: Indexing workspace...")
            .await;

        let state = self.state.clone();
        let client = self.client.clone();

        // Perform indexing (could be done in a spawn for large workspaces)
        let mut state = state.write().await;
        let root = state.workspace_root.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let file_imports = workspace::index_workspace(&root, &mut state.symbol_table);
        state.file_imports = file_imports;

        // Extract packages for each indexed file
        let files = workspace::scan_workspace(&root);
        for file in &files {
            if let Ok(source) = std::fs::read_to_string(file) {
                let pkg = workspace::extract_package(&source);
                if !pkg.is_empty() {
                    state.file_packages.insert(file.clone(), pkg);
                }
            }
        }

        let count = state.symbol_table.lookup_by_kind(SymbolKind::Class).len()
            + state.symbol_table.lookup_by_kind(SymbolKind::Interface).len()
            + state.symbol_table.lookup_by_kind(SymbolKind::Enum).len();

        drop(state);

        client
            .log_message(
                MessageType::INFO,
                format!("Beans: Indexed {} types from {} files", count, files.len()),
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        if let Some(path) = uri_to_path(&uri) {
            let mut state = self.state.write().await;
            state.open_files.insert(uri, text.clone());
            state.reindex_content(&path, &text);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // With TextDocumentSyncKind::FULL, we get the full text
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            if let Some(path) = uri_to_path(&uri) {
                let mut state = self.state.write().await;
                state.open_files.insert(uri, text.clone());
                state.reindex_content(&path, &text);
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(path) = uri_to_path(&uri) {
            let mut state = self.state.write().await;
            state.reindex_from_disk(&path);
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let state = self.state.read().await;

        let text = match state.open_files.get(uri) {
            Some(t) => t.clone(),
            None => {
                if let Some(path) = uri_to_path(uri) {
                    std::fs::read_to_string(path).unwrap_or_default()
                } else {
                    return Ok(None);
                }
            }
        };

        let file_path = match uri_to_path(uri) {
            Some(p) => p,
            None => return Ok(None),
        };

        // Try compound name first (Type.method), then simple name
        let symbol_id = compound_at_position(&text, pos.line, pos.character)
            .and_then(|compound| {
                resolve::resolve_compound_name(
                    &compound,
                    &file_path,
                    &state.file_imports,
                    &state.file_packages,
                    &state.symbol_table,
                )
            })
            .or_else(|| {
                word_at_position(&text, pos.line, pos.character).and_then(|word| {
                    resolve::resolve_name(
                        &word,
                        &file_path,
                        &state.file_imports,
                        &state.file_packages,
                        &state.symbol_table,
                    )
                })
            });

        if let Some(id) = symbol_id {
            if let Some(symbol) = state.symbol_table.get(id) {
                if let Some(ref loc) = symbol.location {
                    if let Some(lsp_loc) = location_to_lsp(loc) {
                        return Ok(Some(GotoDefinitionResponse::Scalar(lsp_loc)));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let state = self.state.read().await;

        let text = match state.open_files.get(uri) {
            Some(t) => t.clone(),
            None => {
                if let Some(path) = uri_to_path(uri) {
                    std::fs::read_to_string(path).unwrap_or_default()
                } else {
                    return Ok(None);
                }
            }
        };

        let word = match word_at_position(&text, pos.line, pos.character) {
            Some(w) => w,
            None => return Ok(None),
        };

        let refs = resolve::find_references_by_name(&word, &state.symbol_table);

        let locations: Vec<Location> = refs
            .iter()
            .filter_map(|id| {
                let sym = state.symbol_table.get(*id)?;
                let loc = sym.location.as_ref()?;
                location_to_lsp(loc)
            })
            .collect();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let state = self.state.read().await;

        let text = match state.open_files.get(uri) {
            Some(t) => t.clone(),
            None => {
                if let Some(path) = uri_to_path(uri) {
                    std::fs::read_to_string(path).unwrap_or_default()
                } else {
                    return Ok(None);
                }
            }
        };

        let file_path = match uri_to_path(uri) {
            Some(p) => p,
            None => return Ok(None),
        };

        let symbol_id = compound_at_position(&text, pos.line, pos.character)
            .and_then(|compound| {
                resolve::resolve_compound_name(
                    &compound,
                    &file_path,
                    &state.file_imports,
                    &state.file_packages,
                    &state.symbol_table,
                )
            })
            .or_else(|| {
                word_at_position(&text, pos.line, pos.character).and_then(|word| {
                    resolve::resolve_name(
                        &word,
                        &file_path,
                        &state.file_imports,
                        &state.file_packages,
                        &state.symbol_table,
                    )
                })
            });

        if let Some(id) = symbol_id {
            if let Some(symbol) = state.symbol_table.get(id) {
                let text = resolve::build_hover_text(symbol);
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: text,
                    }),
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let file_path = match uri_to_path(uri) {
            Some(p) => p,
            None => return Ok(None),
        };

        let state = self.state.read().await;
        let file_symbols = state.symbol_table.lookup_by_file(&file_path);

        if file_symbols.is_empty() {
            return Ok(None);
        }

        // Build document symbols: top-level symbols with children nested
        let mut result: Vec<DocumentSymbol> = Vec::new();

        // Collect top-level symbols (no parent, or parent not in this file)
        for &id in &file_symbols {
            let sym = match state.symbol_table.get(id) {
                Some(s) => s,
                None => continue,
            };

            // Only include top-level symbols (parent is None or parent not in file)
            if sym.parent.is_some() {
                continue;
            }

            let doc_sym = build_document_symbol(sym, &state.symbol_table, &file_symbols);
            result.push(doc_sym);
        }

        Ok(Some(DocumentSymbolResponse::Nested(result)))
    }
}

#[allow(deprecated)] // DocumentSymbol::deprecated field
fn build_document_symbol(
    sym: &beans_core::Symbol,
    table: &SymbolTable,
    file_symbols: &[SymbolId],
) -> DocumentSymbol {
    let range = sym
        .location
        .as_ref()
        .map(|loc| Range {
            start: Position {
                line: loc.start_line,
                character: loc.start_col,
            },
            end: Position {
                line: loc.end_line,
                character: loc.end_col,
            },
        })
        .unwrap_or_default();

    let children: Vec<DocumentSymbol> = table
        .lookup_children(sym.id)
        .iter()
        .filter(|child_id| file_symbols.contains(child_id))
        .filter_map(|child_id| table.get(*child_id))
        .map(|child| build_document_symbol(child, table, file_symbols))
        .collect();

    let detail = match &sym.signature {
        Some(beans_core::Signature::Method {
            return_type,
            parameters,
            ..
        }) => {
            let params: Vec<String> = parameters.iter().map(|p| p.param_type.to_string()).collect();
            Some(format!("({}) -> {}", params.join(", "), return_type))
        }
        Some(beans_core::Signature::Field { field_type, .. }) => Some(field_type.to_string()),
        _ => None,
    };

    DocumentSymbol {
        name: sym.name.clone(),
        detail,
        kind: symbol_kind_to_lsp(sym.kind),
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_at_position() {
        let text = "public class MyClass extends Base {\n    private String name;\n}";
        assert_eq!(
            word_at_position(text, 0, 15),
            Some("MyClass".to_string())
        );
        assert_eq!(
            word_at_position(text, 0, 29),
            Some("Base".to_string())
        );
        assert_eq!(
            word_at_position(text, 1, 19),
            Some("name".to_string())
        );
    }

    #[test]
    fn test_word_at_position_edge_cases() {
        let text = "int x;";
        assert_eq!(word_at_position(text, 0, 0), Some("int".to_string()));
        assert_eq!(word_at_position(text, 0, 4), Some("x".to_string()));
        // On the semicolon — cursor is just past 'x', still resolves to 'x'
        assert_eq!(word_at_position(text, 0, 5), Some("x".to_string()));
        // Beyond the line
        assert_eq!(word_at_position(text, 0, 100), None);
    }

    #[test]
    fn test_compound_at_position() {
        let text = "MyClass.doWork()";
        assert_eq!(
            compound_at_position(text, 0, 10),
            Some("MyClass.doWork".to_string())
        );
        assert_eq!(
            compound_at_position(text, 0, 3),
            Some("MyClass.doWork".to_string())
        );
    }

    #[test]
    fn test_symbol_kind_to_lsp() {
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Class), SymbolKind2::CLASS);
        assert_eq!(
            symbol_kind_to_lsp(SymbolKind::Interface),
            SymbolKind2::INTERFACE
        );
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Method), SymbolKind2::METHOD);
        assert_eq!(symbol_kind_to_lsp(SymbolKind::Field), SymbolKind2::FIELD);
    }

    #[test]
    fn test_document_symbol_outline() {
        // Parse a Java file, populate the symbol table, and verify document_symbol output
        use beans_lang_java::parse_java_file;
        use std::path::Path;

        let source = r#"
package com.example;

public class Dog {
    private String name;

    public Dog(String name) {
        this.name = name;
    }

    public String getName() {
        return name;
    }
}
"#;

        let path = Path::new("src/Dog.java");
        let symbols = parse_java_file(path, source);

        let mut table = SymbolTable::new();
        for sym in symbols {
            table.insert(sym);
        }

        let file_symbols = table.lookup_by_file(path);
        assert!(!file_symbols.is_empty());

        // Build document symbols for top-level entries
        let mut doc_symbols: Vec<DocumentSymbol> = Vec::new();
        for &id in &file_symbols {
            let sym = table.get(id).unwrap();
            if sym.parent.is_some() {
                continue;
            }
            doc_symbols.push(build_document_symbol(sym, &table, &file_symbols));
        }

        // Should have one top-level symbol: Dog
        assert_eq!(doc_symbols.len(), 1);
        let dog = &doc_symbols[0];
        assert_eq!(dog.name, "Dog");
        assert_eq!(dog.kind, SymbolKind2::CLASS);

        // Dog should have 3 children: name (field), Dog (constructor), getName (method)
        let children = dog.children.as_ref().expect("Dog should have children");
        assert_eq!(children.len(), 3);

        let child_names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
        assert!(child_names.contains(&"name"), "missing field 'name'");
        assert!(child_names.contains(&"Dog"), "missing constructor 'Dog'");
        assert!(child_names.contains(&"getName"), "missing method 'getName'");

        // Verify kinds
        let name_field = children.iter().find(|c| c.name == "name").unwrap();
        assert_eq!(name_field.kind, SymbolKind2::FIELD);

        let ctor = children.iter().find(|c| c.name == "Dog").unwrap();
        assert_eq!(ctor.kind, SymbolKind2::CONSTRUCTOR);

        let getter = children.iter().find(|c| c.name == "getName").unwrap();
        assert_eq!(getter.kind, SymbolKind2::METHOD);
        // Verify detail shows return type
        assert_eq!(getter.detail.as_deref(), Some("() -> String"));
    }

    /// Full end-to-end integration test: parse Java files, populate symbol table,
    /// resolve names across files using imports, and verify goto-definition works.
    #[test]
    fn test_end_to_end_cross_file_navigation() {
        use crate::resolve;
        use std::path::Path;

        let service_source = r#"
package com.example.service;

import com.example.model.User;

public class UserService {
    public User findUser(String id) {
        return null;
    }
}
"#;

        let model_source = r#"
package com.example.model;

public class User {
    private String name;
    private int age;

    public String getName() {
        return name;
    }
}
"#;

        let service_path = Path::new("src/service/UserService.java");
        let model_path = Path::new("src/model/User.java");

        // Parse both files
        let mut state = ServerState {
            symbol_table: SymbolTable::new(),
            file_imports: HashMap::new(),
            file_packages: HashMap::new(),
            open_files: HashMap::new(),
            workspace_root: None,
        };

        state.reindex_content(service_path, service_source);
        state.reindex_content(model_path, model_source);

        // Verify symbols were indexed
        assert!(state.symbol_table.lookup_by_fqn("com.example.model.User").is_some());
        assert!(state.symbol_table.lookup_by_fqn("com.example.service.UserService").is_some());
        assert!(state.symbol_table.lookup_by_fqn("com.example.model.User.getName").is_some());
        assert!(state.symbol_table.lookup_by_fqn("com.example.model.User.name").is_some());

        // Verify imports were extracted
        let service_imports = state.file_imports.get(service_path).unwrap();
        assert_eq!(service_imports.len(), 1);
        assert_eq!(
            service_imports[0],
            resolve::Import::Single("com.example.model.User".to_string())
        );

        // Verify packages were extracted
        assert_eq!(
            state.file_packages.get(service_path).unwrap(),
            "com.example.service"
        );
        assert_eq!(
            state.file_packages.get(model_path).unwrap(),
            "com.example.model"
        );

        // Simulate goto-definition: in UserService.java, cursor is on "User"
        // This should resolve to com.example.model.User via the import
        let resolved = resolve::resolve_name(
            "User",
            service_path,
            &state.file_imports,
            &state.file_packages,
            &state.symbol_table,
        );
        assert!(resolved.is_some(), "should resolve 'User' from import");
        let user_sym = state.symbol_table.get(resolved.unwrap()).unwrap();
        assert_eq!(user_sym.fqn, "com.example.model.User");
        assert_eq!(user_sym.kind, SymbolKind::Class);
        // Verify it points to the correct file
        let loc = user_sym.location.as_ref().unwrap();
        assert_eq!(loc.file, model_path);

        // Resolve "User.getName" as a compound name from UserService.java
        let resolved_method = resolve::resolve_compound_name(
            "User.getName",
            service_path,
            &state.file_imports,
            &state.file_packages,
            &state.symbol_table,
        );
        assert!(resolved_method.is_some(), "should resolve 'User.getName'");
        let method_sym = state.symbol_table.get(resolved_method.unwrap()).unwrap();
        assert_eq!(method_sym.fqn, "com.example.model.User.getName");
        assert_eq!(method_sym.kind, SymbolKind::Method);

        // Verify same-package resolution: within model, "User" resolves without import
        let resolved_same_pkg = resolve::resolve_name(
            "User",
            model_path,
            &state.file_imports,
            &state.file_packages,
            &state.symbol_table,
        );
        assert!(resolved_same_pkg.is_some());
        let same_pkg_sym = state.symbol_table.get(resolved_same_pkg.unwrap()).unwrap();
        assert_eq!(same_pkg_sym.fqn, "com.example.model.User");

        // Verify find-references: "User" should appear in both files
        let refs = resolve::find_references_by_name("User", &state.symbol_table);
        assert_eq!(refs.len(), 1); // only 1 symbol declaration named "User" (the class)

        // Verify hover text
        let hover = resolve::build_hover_text(user_sym);
        assert!(hover.contains("class User"));
        assert!(hover.contains("com.example.model.User"));
    }

    /// Test incremental re-indexing: modify a file and verify the symbol table updates.
    #[test]
    fn test_incremental_reindex_updates_resolution() {
        use crate::resolve;
        use std::path::Path;

        let path = Path::new("src/Foo.java");
        let mut state = ServerState {
            symbol_table: SymbolTable::new(),
            file_imports: HashMap::new(),
            file_packages: HashMap::new(),
            open_files: HashMap::new(),
            workspace_root: None,
        };

        // Initial version
        state.reindex_content(path, "package com.test;\npublic class Foo {\n    public void oldMethod() {}\n}");
        assert!(state.symbol_table.lookup_by_fqn("com.test.Foo.oldMethod").is_some());

        // User edits the file: renames method
        state.reindex_content(path, "package com.test;\npublic class Foo {\n    public void newMethod() {}\n}");
        assert!(state.symbol_table.lookup_by_fqn("com.test.Foo.oldMethod").is_none(), "old method should be gone");
        assert!(state.symbol_table.lookup_by_fqn("com.test.Foo.newMethod").is_some(), "new method should exist");

        // Class itself should still resolve
        let resolved = resolve::resolve_name(
            "Foo",
            path,
            &state.file_imports,
            &state.file_packages,
            &state.symbol_table,
        );
        assert!(resolved.is_some());
        assert_eq!(state.symbol_table.get(resolved.unwrap()).unwrap().fqn, "com.test.Foo");
    }
}

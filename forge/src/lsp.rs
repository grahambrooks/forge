//! Forge Language Server — provides IDE features for .forge files.
//!
//! Features:
//! - Diagnostics: parse errors and architectural lint warnings on save
//! - Hover: element details (kind, description, technology, tags)
//! - Go to definition: jump to element declarations
//! - Completion: DSL keywords and element identifiers
//! - Document symbols: outline of the .forge file structure

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::check;
use crate::model::*;
use crate::parser;

/// State held per-document.
struct DocState {
    text: String,
    model: Option<Model>,
    /// Map from element id → (line, col) of its definition.
    locations: HashMap<String, Position>,
}

pub struct ForgeLsp {
    client: Client,
    docs: Arc<RwLock<HashMap<Url, DocState>>>,
}

impl ForgeLsp {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            docs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ForgeLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["\"".into(), " ".into(), "{".into()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Forge LSP initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.update_document(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.update_document(uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut docs = self.docs.write().await;
        docs.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let model = match &doc.model {
            Some(m) => m,
            None => return Ok(None),
        };

        let word = word_at_position(&doc.text, pos);
        if word.is_empty() {
            return Ok(None);
        }

        // Try to find an element matching this word
        if let Some(info) = element_hover_info(model, &word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: info,
                }),
                range: None,
            }));
        }

        // Try keyword documentation
        if let Some(info) = keyword_hover(&word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: info,
                }),
                range: None,
            }));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let word = word_at_position(&doc.text, pos);
        if word.is_empty() {
            return Ok(None);
        }

        // Look up location of element definition
        if let Some(def_pos) = doc.locations.get(&word) {
            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri.clone(),
                range: Range::new(*def_pos, *def_pos),
            })));
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let mut items = Vec::new();

        // DSL keywords
        for kw in KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Forge DSL keyword".into()),
                ..Default::default()
            });
        }

        // Element identifiers from the model
        if let Some(model) = &doc.model {
            for (id, el) in &model.elements {
                let local = id.split('.').next_back().unwrap_or(id);
                items.push(CompletionItem {
                    label: local.to_string(),
                    kind: Some(CompletionItemKind::REFERENCE),
                    detail: Some(format!("{} — {}", kind_label(el.kind), el.name)),
                    documentation: el.description.as_ref().map(|d| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::PlainText,
                            value: d.clone(),
                        })
                    }),
                    ..Default::default()
                });
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let model = match &doc.model {
            Some(m) => m,
            None => return Ok(None),
        };

        let mut symbols = Vec::new();

        // Elements
        for (id, el) in &model.elements {
            if el.parent.is_some() {
                continue; // Only show top-level in outline
            }
            let pos = doc
                .locations
                .get(id)
                .copied()
                .unwrap_or(Position::new(0, 0));
            let range = Range::new(pos, pos);
            #[allow(deprecated)]
            symbols.push(DocumentSymbol {
                name: el.name.clone(),
                detail: Some(format!("{} ({})", kind_label(el.kind), id)),
                kind: element_symbol_kind(el.kind),
                range,
                selection_range: range,
                children: None,
                tags: None,
                deprecated: None,
            });
        }

        // Views
        for view in &model.views {
            let range = Range::new(Position::new(0, 0), Position::new(0, 0));
            #[allow(deprecated)]
            symbols.push(DocumentSymbol {
                name: view.title.as_deref().unwrap_or(&view.key).to_string(),
                detail: Some(format!("{:?} view", view.kind)),
                kind: SymbolKind::NAMESPACE,
                range,
                selection_range: range,
                children: None,
                tags: None,
                deprecated: None,
            });
        }

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl ForgeLsp {
    /// Re-parse the document and publish diagnostics.
    async fn update_document(&self, uri: Url, text: String) {
        let (model, locations, diagnostics) = analyze_document(&text);

        {
            let mut docs = self.docs.write().await;
            docs.insert(
                uri.clone(),
                DocState {
                    text,
                    model,
                    locations,
                },
            );
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

/// Parse the document, run lint checks, and collect diagnostics + element locations.
fn analyze_document(text: &str) -> (Option<Model>, HashMap<String, Position>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut locations = HashMap::new();

    let model = match parser::parse(text) {
        Ok(m) => {
            // Collect element definition locations by scanning for their names
            for (id, el) in &m.elements {
                if let Some(pos) = find_element_position(text, &el.name) {
                    locations.insert(id.clone(), pos);
                    // Also index by local name
                    let local = id.split('.').next_back().unwrap_or(id);
                    locations.insert(local.to_string(), pos);
                }
            }

            // Run architectural checks
            let violations = check::check(&m, check::Severity::Info);
            for v in &violations {
                let severity = match v.severity {
                    check::Severity::Error => DiagnosticSeverity::ERROR,
                    check::Severity::Warning => DiagnosticSeverity::WARNING,
                    check::Severity::Info => DiagnosticSeverity::INFORMATION,
                };
                // Try to locate the element
                let pos = v
                    .element_id
                    .as_ref()
                    .and_then(|id| locations.get(id))
                    .copied()
                    .unwrap_or(Position::new(0, 0));

                diagnostics.push(Diagnostic {
                    range: Range::new(pos, pos),
                    severity: Some(severity),
                    source: Some("forge".into()),
                    code: Some(NumberOrString::String(v.rule.to_string())),
                    message: v.message.clone(),
                    ..Default::default()
                });
            }

            Some(m)
        }
        Err(e) => {
            let line = if e.line > 0 { e.line - 1 } else { 0 } as u32;
            let col = if e.col > 0 { e.col - 1 } else { 0 } as u32;
            diagnostics.push(Diagnostic {
                range: Range::new(Position::new(line, col), Position::new(line, col + 1)),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("forge".into()),
                message: e.msg,
                ..Default::default()
            });
            None
        }
    };

    (model, locations, diagnostics)
}

// ─── Helpers ─────────────────────────────────────────────────────

/// Extract the word under the cursor position.
pub fn word_at_position(text: &str, pos: Position) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return String::new();
    }
    let line = lines[line_idx];
    let col = pos.character as usize;
    if col > line.len() {
        return String::new();
    }

    let chars: Vec<char> = line.chars().collect();
    let mut start = col;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }

    chars[start..end].iter().collect()
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '.'
}

/// Find the position of an element name in the source text.
pub fn find_element_position(text: &str, name: &str) -> Option<Position> {
    // Look for `"name"` in the source
    let needle = format!("\"{}\"", name);
    let mut offset = 0;
    for (line_idx, line) in text.lines().enumerate() {
        if let Some(col) = line[offset..].find(&needle).or_else(|| line.find(&needle)) {
            return Some(Position::new(line_idx as u32, col as u32));
        }
        offset = 0;
    }
    None
}

/// Build hover markdown for an element.
pub fn element_hover_info(model: &Model, word: &str) -> Option<String> {
    // Try full id, then partial match
    let el = model.elements.get(word).or_else(|| {
        model
            .elements
            .values()
            .find(|e| e.id.ends_with(&format!(".{}", word)) || e.id == word)
    })?;

    let mut info = format!("### {} `{}`\n", kind_label(el.kind), el.name);
    info.push_str(&format!("**Kind:** {}\n\n", kind_label(el.kind)));
    info.push_str(&format!("**ID:** `{}`\n\n", el.id));

    if let Some(ref desc) = el.description {
        info.push_str(&format!("{}\n\n", desc));
    }
    if let Some(ref tech) = el.technology {
        info.push_str(&format!("**Technology:** {}\n\n", tech));
    }
    if !el.tags.is_empty() {
        info.push_str(&format!("**Tags:** {}\n\n", el.tags.join(", ")));
    }
    if let Some(ref parent) = el.parent {
        info.push_str(&format!("**Parent:** `{}`\n", parent));
    }

    Some(info)
}

/// Hover docs for DSL keywords.
fn keyword_hover(word: &str) -> Option<String> {
    let info = match word {
        "forge" => "Top-level block defining a Forge model.",
        "model" => {
            "Structural architecture: actors, systems, containers, components, and relationships."
        }
        "process" => "Delivery processes: repositories, branching strategies, and CI/CD pipelines.",
        "deployment" => "Infrastructure topology: deployment nodes with container instances.",
        "techStack" => "Technology inventory organized by category.",
        "dataModel" => "Data entities, fields, and entity relationships.",
        "trustBoundaries" => "Security zones grouping containers by trust level.",
        "teams" => "Team definitions with ownership assignments.",
        "views" => {
            "Diagram definitions: systemContext, container, pipelineView, deploymentView, etc."
        }
        "docs" => "Markdown documentation references.",
        "person" => "An actor (user or external system) that interacts with the system.",
        "system" => "A top-level software system boundary.",
        "container" => "A deployable unit within a system (service, database, app).",
        "component" => "A module or package within a container.",
        "pipeline" => "A CI/CD pipeline with stages and gates.",
        "stage" => "A step in a CI/CD pipeline.",
        "gate" => "A quality gate that must pass before proceeding.",
        "node" => "A deployment node (server, cluster, cloud region).",
        "instance" => "Places a container inside a deployment node.",
        "entity" => "A data entity with typed fields.",
        "boundary" => "A trust boundary grouping containers by security level.",
        "team" => "A team with ownership of containers and services.",
        "branch" => "A git branch in a branching strategy.",
        _ => return None,
    };
    Some(format!("**{}** — {}", word, info))
}

fn kind_label(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "Person",
        ElementKind::System => "Software System",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
        ElementKind::Pipeline => "Pipeline",
        ElementKind::Stage => "Stage",
        ElementKind::Gate => "Gate",
        ElementKind::Repository => "Repository",
        ElementKind::Branch => "Branch",
        ElementKind::DeploymentNode => "Deployment Node",
        _ => "Element",
    }
}

fn element_symbol_kind(kind: ElementKind) -> SymbolKind {
    match kind {
        ElementKind::Person => SymbolKind::INTERFACE,
        ElementKind::System => SymbolKind::MODULE,
        ElementKind::Container => SymbolKind::CLASS,
        ElementKind::Component => SymbolKind::METHOD,
        ElementKind::Pipeline => SymbolKind::FUNCTION,
        ElementKind::Stage => SymbolKind::EVENT,
        ElementKind::Repository => SymbolKind::FILE,
        ElementKind::Branch => SymbolKind::VARIABLE,
        ElementKind::DeploymentNode => SymbolKind::PACKAGE,
        _ => SymbolKind::OBJECT,
    }
}

const KEYWORDS: &[&str] = &[
    "forge",
    "model",
    "process",
    "deployment",
    "views",
    "docs",
    "techStack",
    "dataModel",
    "trustBoundaries",
    "teams",
    "person",
    "system",
    "container",
    "component",
    "description",
    "technology",
    "tags",
    "pipeline",
    "stage",
    "gate",
    "step",
    "needs",
    "environment",
    "strategy",
    "branch",
    "protection",
    "branchesFrom",
    "mergesInto",
    "node",
    "instance",
    "entity",
    "field",
    "relationship",
    "boundary",
    "level",
    "includes",
    "team",
    "owns",
    "contact",
    "category",
    "tech",
    "version",
    "purpose",
    "doc",
    "systemContext",
    "container",
    "pipelineView",
    "deploymentView",
    "techStackView",
    "branchingView",
    "dataModelView",
    "trustBoundaryView",
    "teamView",
    "include",
    "autoLayout",
    "title",
];

/// Start the LSP server on stdio.
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ForgeLsp::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_at_position_extracts_identifier() {
        let text = "    api = container \"Payment API\" {\n";
        let word = word_at_position(text, Position::new(0, 6));
        assert_eq!(word, "api");
    }

    #[test]
    fn word_at_position_dotted() {
        let text = "    payments.api -> payments.db\n";
        let word = word_at_position(text, Position::new(0, 8));
        assert_eq!(word, "payments.api");
    }

    #[test]
    fn word_at_position_empty_line() {
        let text = "\n\n";
        let word = word_at_position(text, Position::new(0, 0));
        assert_eq!(word, "");
    }

    #[test]
    fn find_element_in_source() {
        let text = r#"forge "Test" {
  model {
    svc = container "My Service" {}
  }
}"#;
        let pos = find_element_position(text, "My Service");
        assert!(pos.is_some());
        let p = pos.unwrap();
        assert_eq!(p.line, 2);
    }

    #[test]
    fn hover_info_for_element() {
        let text = include_str!("../examples/payments.forge");
        let model = parser::parse(text).unwrap();
        let info = element_hover_info(&model, "payments.api");
        assert!(info.is_some());
        let info = info.unwrap();
        assert!(info.contains("Payment API"));
        assert!(info.contains("Rust / Actix"));
    }

    #[test]
    fn hover_info_for_keyword() {
        let info = keyword_hover("container");
        assert!(info.is_some());
        assert!(info.unwrap().contains("deployable unit"));
    }

    #[test]
    fn analyze_valid_document_produces_model() {
        let text = r#"forge "Test" { model { a = person "Alice" {} } views {} }"#;
        let (model, locations, diags) = analyze_document(text);
        assert!(model.is_some());
        assert!(locations.contains_key("a"));
        // May have info-level diagnostics (orphaned elements) but no errors
        assert!(!diags
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)));
    }

    #[test]
    fn analyze_invalid_document_produces_error() {
        let text = r#"not valid forge"#;
        let (model, _, diags) = analyze_document(text);
        assert!(model.is_none());
        assert!(diags
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR)));
    }
}

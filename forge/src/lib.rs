//! forge as a library: parse a model (with its `!include`s) and lay its
//! views out as [`diagram_scene::Scene`]s, which a host draws natively in
//! its own colours. The command line (`forge`) builds on the same modules.
//!
//! ```no_run
//! let model = forge_dsl::parse_file("architecture.forge").unwrap();
//! for view in forge_dsl::views(&model) {
//!     let scene = forge_dsl::scene(&model, &view.key, &diagram_scene::Estimate);
//! }
//! ```

pub mod layout;
pub mod model;
pub mod parser;
pub mod preprocess;
pub mod scene;
pub mod text;

use std::path::Path;

pub use parser::ParseError;

/// Parses `source`, resolving `!include` and friends against `base_dir`.
pub fn parse(source: &str, base_dir: &Path) -> Result<model::Model, ParseError> {
    parser::parse_with_preprocess(source, base_dir)
}

/// Reads and parses the file at `path`.
pub fn parse_file(path: impl AsRef<Path>) -> Result<model::Model, ParseError> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path).map_err(|e| ParseError {
        msg: format!("{}: {e}", path.display()),
        line: 0,
        col: 0,
    })?;
    parse(&source, path.parent().unwrap_or(Path::new(".")))
}

/// A view a model defines, in declaration order.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewInfo {
    pub key: String,
    /// The view's title, or its key.
    pub title: String,
    pub kind: model::ViewKind,
}

/// The views of `model` that lay out as one diagram (composite views,
/// grids of other views, are left out: their cells are views already).
pub fn views(model: &model::Model) -> Vec<ViewInfo> {
    model
        .views
        .iter()
        .filter(|v| v.kind != model::ViewKind::Composite)
        .map(|v| ViewInfo {
            key: v.key.clone(),
            title: v.title.clone().unwrap_or_else(|| v.key.clone()),
            kind: v.kind,
        })
        .collect()
}

/// The view `key` of `model`, laid out, or `None` if there's no such view.
/// `measure` should measure the font the scene's text will be drawn in.
pub fn scene(
    model: &model::Model,
    key: &str,
    measure: &dyn diagram_scene::Measure,
) -> Option<diagram_scene::Scene> {
    let view = model.views.iter().find(|v| v.key == key)?;
    let layout = layout::compute_layout(model, view);
    Some(scene::from_layout(&layout, measure))
}

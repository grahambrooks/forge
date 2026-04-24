//! `views { ... }` block: view declarations, view bodies, and animation frames.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_views(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in views"));
            }

            let kind_str = self.parse_ident()?;
            // DSL v2: every view kind ends in `-view`. Scoped views take a
            // bare id; unscoped views take only a key. View bodies are
            // optional — a view with no properties other than the key is
            // valid (e.g. `tech-stack-view "TechStack"`).
            let (kind, needs_scope, default_layout) = match kind_str.as_str() {
                "system-context-view" => (ViewKind::SystemContext, true, AutoLayout::LeftRight),
                "container-view" => (ViewKind::Container, true, AutoLayout::TopBottom),
                "component-view" => (ViewKind::Component, true, AutoLayout::TopBottom),
                "pipeline-view" => (ViewKind::PipelineView, true, AutoLayout::LeftRight),
                "deployment-view" => (ViewKind::Deployment, true, AutoLayout::TopBottom),
                "branching-view" => (ViewKind::Branching, true, AutoLayout::LeftRight),
                "dynamic-view" => (ViewKind::Dynamic, true, AutoLayout::TopBottom),
                "tech-stack-view" => (ViewKind::TechStack, false, AutoLayout::TopBottom),
                "data-model-view" => (ViewKind::DataModel, false, AutoLayout::TopBottom),
                "trust-boundary-view" => {
                    (ViewKind::TrustBoundaryView, false, AutoLayout::TopBottom)
                }
                "team-view" => (ViewKind::TeamMap, false, AutoLayout::TopBottom),
                "api-catalog-view" => (ViewKind::ApiCatalogView, false, AutoLayout::TopBottom),
                "event-flow-view" => (ViewKind::EventFlowView, false, AutoLayout::TopBottom),
                "composite-view" => (ViewKind::Composite, false, AutoLayout::TopBottom),
                _ => {
                    return Err(self.error(format!("unknown view kind '{}'", kind_str)));
                }
            };

            let scope = if needs_scope {
                let scope_raw = self.parse_ident()?;
                Some(self.resolve_ref(&scope_raw))
            } else {
                None
            };
            let key = self.parse_string()?;
            let composite = if kind == ViewKind::Composite {
                Some(CompositeView {
                    cells: Vec::new(),
                    cols: 1,
                    rows: 1,
                    cell_size: (600, 400),
                })
            } else {
                None
            };
            let mut view = View {
                kind,
                key,
                scope,
                title: None,
                auto_layout: default_layout,
                include_all: false,
                animation: Animation::default(),
                composite,
            };
            // Optional body — v2 lets you omit `{}` for views with nothing
            // to configure beyond the key.
            if self.peek_after_ws() == Some('{') {
                self.parse_view_body(&mut view)?;
            }
            if view.kind == ViewKind::Composite {
                if let Some(comp) = view.composite.as_mut() {
                    if !comp.cells.is_empty() && comp.rows * comp.cols < comp.cells.len() as u32 {
                        comp.rows = comp.cells.len().div_ceil(comp.cols.max(1) as usize) as u32;
                    }
                }
            }
            self.model.views.push(view);
        }
    }

    fn parse_view_body(&mut self, view: &mut View) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in view"));
            }

            // Numbered relationship for dynamic views: `1. src -> dst "label"`.
            // Detection: leading ASCII digit followed (after optional whitespace)
            // by a `.`. Only meaningful inside dynamic views — we still parse
            // and drop the result elsewhere so a stray number doesn't blow up.
            if view.kind == ViewKind::Dynamic && self.peek().is_some_and(|c| c.is_ascii_digit()) {
                let order = self.parse_u32()?;
                self.skip_ws();
                self.expect('.')?;
                self.skip_ws();
                let src_raw = self.parse_ident()?;
                let src = self.resolve_ref(&src_raw);
                self.skip_ws();
                self.expect('-')?;
                self.expect('>')?;
                self.skip_ws();
                let dst_raw = self.parse_ident()?;
                let dst = self.resolve_ref(&dst_raw);
                let label = self.parse_string()?;
                let tech = if self.peek_after_ws() == Some('"') {
                    Some(self.parse_string()?)
                } else {
                    None
                };
                self.model.add_relationship(Relationship {
                    frm: src,
                    to: dst,
                    label,
                    technology: tech,
                    order: Some(order),
                });
                continue;
            }

            let prop = self.parse_ident()?;
            match prop.as_str() {
                "include" => {
                    self.skip_ws();
                    if self.peek() == Some('*') {
                        self.advance();
                        view.include_all = true;
                    } else {
                        // A list of element id references. Read until we
                        // hit something that isn't an ident-start so
                        // `include a b c` works as one line.
                        while self
                            .peek_after_ws()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_')
                        {
                            self.parse_ident()?;
                        }
                    }
                }
                "auto-layout" => {
                    let d = self.parse_ident()?;
                    view.auto_layout = if d == "lr" {
                        AutoLayout::LeftRight
                    } else if d == "tb" {
                        AutoLayout::TopBottom
                    } else {
                        return Err(self.error(format!(
                            "unknown auto-layout direction '{}', expected 'lr' or 'tb'",
                            d
                        )));
                    };
                }
                "title" => {
                    view.title = Some(self.parse_string()?);
                }
                "animation" => {
                    self.parse_animation(&mut view.animation)?;
                }
                "grid" if view.kind == ViewKind::Composite => {
                    let cols = self.parse_u32()?;
                    self.skip_ws();
                    let rows = self.parse_u32()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cols = cols.max(1);
                        comp.rows = rows.max(1);
                    }
                }
                "cell-size" if view.kind == ViewKind::Composite => {
                    let w = self.parse_u32()?;
                    self.skip_ws();
                    let h = self.parse_u32()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cell_size = (w.max(100), h.max(100));
                    }
                }
                "cell" if view.kind == ViewKind::Composite => {
                    let key = self.parse_string()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cells.push(key);
                    }
                }
                _ => {
                    return Err(self.error(format!("unknown view keyword '{}'", prop)));
                }
            }
        }
    }

    fn parse_animation(&mut self, anim: &mut Animation) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in animation"));
            }
            let kw = self.parse_ident()?;
            if kw == "frame" {
                let label = self.parse_string()?;
                let mut frame = AnimationFrame {
                    label,
                    includes: Vec::new(),
                    include_all: false,
                    highlights: Vec::new(),
                    states: Vec::new(),
                    notes: None,
                };
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    if self.at_end() {
                        return Err(self.error("unexpected EOF in frame"));
                    }
                    let prop = self.parse_ident()?;
                    match prop.as_str() {
                        "include" => {
                            self.skip_ws();
                            if self.peek() == Some('*') {
                                self.advance();
                                frame.include_all = true;
                            } else {
                                // Read element/relationship references
                                // Could be: "element", "el1 -> el2", or comma-separated
                                let ref1 = self.parse_ident()?;
                                self.skip_ws();
                                if self.peek() == Some('-') {
                                    // relationship: ref1 -> ref2
                                    self.advance(); // -
                                    self.expect('>')?;
                                    let ref2 = self.parse_ident()?;
                                    let r1 = self.resolve_ref(&ref1);
                                    let r2 = self.resolve_ref(&ref2);
                                    frame.includes.push(format!("{} -> {}", r1, r2));
                                } else {
                                    frame.includes.push(self.resolve_ref(&ref1));
                                }
                            }
                        }
                        "highlight" => {
                            let target = self.parse_ident()?;
                            let resolved = self.resolve_ref(&target);
                            let mut hl = FrameHighlight {
                                target: resolved,
                                color: None,
                                line_width: None,
                                label: None,
                            };
                            // Check for -> (relationship highlight)
                            self.skip_ws();
                            if self.peek() == Some('-') {
                                self.advance();
                                self.expect('>')?;
                                let t2 = self.parse_ident()?;
                                hl.target = format!("{} -> {}", hl.target, self.resolve_ref(&t2));
                                // May chain: a -> b -> c
                                self.skip_ws();
                                while self.peek() == Some('-') {
                                    self.advance();
                                    self.expect('>')?;
                                    let tn = self.parse_ident()?;
                                    hl.target
                                        .push_str(&format!(" -> {}", self.resolve_ref(&tn)));
                                }
                            }
                            if self.peek_after_ws() == Some('{') {
                                self.expect('{')?;
                                loop {
                                    self.skip_ws();
                                    if self.peek() == Some('}') {
                                        self.advance();
                                        break;
                                    }
                                    let hp = self.parse_ident()?;
                                    match hp.as_str() {
                                        "color" => hl.color = Some(self.parse_string()?),
                                        "line-width" => {
                                            let v = self.parse_ident()?;
                                            hl.line_width = v.parse().ok();
                                        }
                                        "label" => hl.label = Some(self.parse_string()?),
                                        _ => {
                                            return Err(self.error(format!(
                                                "unknown highlight keyword '{}'",
                                                hp
                                            )));
                                        }
                                    }
                                }
                            }
                            frame.highlights.push(hl);
                        }
                        "state" => {
                            let target = self.parse_ident()?;
                            let resolved = self.resolve_ref(&target);
                            let state_label = self.parse_string()?;
                            let mut state = FrameState {
                                target: resolved,
                                label: state_label,
                                color: None,
                                pulse: false,
                            };
                            if self.peek_after_ws() == Some('{') {
                                self.expect('{')?;
                                loop {
                                    self.skip_ws();
                                    if self.peek() == Some('}') {
                                        self.advance();
                                        break;
                                    }
                                    let sp = self.parse_ident()?;
                                    match sp.as_str() {
                                        "color" => state.color = Some(self.parse_string()?),
                                        "pulse" => {
                                            let v = self.parse_ident()?;
                                            state.pulse = v == "true";
                                        }
                                        _ => {
                                            if self.peek_after_ws() == Some('"') {
                                                self.parse_string()?;
                                            }
                                        }
                                    }
                                }
                            }
                            frame.states.push(state);
                        }
                        "notes" => {
                            frame.notes = Some(self.parse_string()?);
                        }
                        _ => {
                            if self.peek_after_ws() == Some('"') {
                                self.parse_string()?;
                            } else if self.peek_after_ws() == Some('{') {
                                self.skip_block()?;
                            }
                        }
                    }
                }
                anim.frames.push(frame);
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }
}

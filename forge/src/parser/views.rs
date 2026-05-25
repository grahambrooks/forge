//! `views { ... }` block: view declarations, view bodies, and animation frames.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_views(&mut self) -> Result<(), ParseError> {
        self.parse_braced("views", |this| {
            let kind_str = this.parse_ident()?;
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
                    return Err(this.error(format!("unknown view kind '{}'", kind_str)));
                }
            };

            let scope = if needs_scope {
                let scope_raw = this.parse_ident()?;
                Some(this.resolve_ref(&scope_raw))
            } else {
                None
            };
            let key = this.parse_string()?;
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
            if this.peek_after_ws() == Some('{') {
                this.parse_view_body(&mut view)?;
            }
            if view.kind == ViewKind::Composite {
                if let Some(comp) = view.composite.as_mut() {
                    if !comp.cells.is_empty() && comp.rows * comp.cols < comp.cells.len() as u32 {
                        comp.rows = comp.cells.len().div_ceil(comp.cols.max(1) as usize) as u32;
                    }
                }
            }
            this.model.views.push(view);
            Ok(())
        })
    }

    fn parse_view_body(&mut self, view: &mut View) -> Result<(), ParseError> {
        self.parse_braced("view", |this| {
            // Numbered relationship for dynamic views: `1. src -> dst "label"`.
            // Detection: leading ASCII digit followed (after optional whitespace)
            // by a `.`. Only meaningful inside dynamic views — we still parse
            // and drop the result elsewhere so a stray number doesn't blow up.
            if view.kind == ViewKind::Dynamic && this.peek().is_some_and(|c| c.is_ascii_digit()) {
                let order = this.parse_u32()?;
                this.skip_ws();
                this.expect('.')?;
                this.skip_ws();
                let src_raw = this.parse_ident()?;
                let src = this.resolve_ref(&src_raw);
                this.skip_ws();
                this.expect('-')?;
                this.expect('>')?;
                this.skip_ws();
                let dst_raw = this.parse_ident()?;
                let dst = this.resolve_ref(&dst_raw);
                let label = this.parse_string()?;
                let tech = if this.peek_after_ws() == Some('"') {
                    Some(this.parse_string()?)
                } else {
                    None
                };
                this.model.add_relationship(Relationship {
                    frm: src,
                    to: dst,
                    label,
                    technology: tech,
                    order: Some(order),
                });
                return Ok(());
            }

            let prop = this.parse_ident()?;
            match prop.as_str() {
                "include" => {
                    this.skip_ws();
                    if this.peek() == Some('*') {
                        this.advance();
                        view.include_all = true;
                    } else {
                        // A list of element id references. Read until we
                        // hit something that isn't an ident-start so
                        // `include a b c` works as one line.
                        while this
                            .peek_after_ws()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_')
                        {
                            this.parse_ident()?;
                        }
                    }
                }
                "auto-layout" => {
                    let d = this.parse_ident()?;
                    view.auto_layout = if d == "lr" {
                        AutoLayout::LeftRight
                    } else if d == "tb" {
                        AutoLayout::TopBottom
                    } else {
                        return Err(this.error(format!(
                            "unknown auto-layout direction '{}', expected 'lr' or 'tb'",
                            d
                        )));
                    };
                }
                "title" => {
                    view.title = Some(this.parse_string()?);
                }
                "animation" => {
                    this.parse_animation(&mut view.animation)?;
                }
                "grid" if view.kind == ViewKind::Composite => {
                    let cols = this.parse_u32()?;
                    this.skip_ws();
                    let rows = this.parse_u32()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cols = cols.max(1);
                        comp.rows = rows.max(1);
                    }
                }
                "cell-size" if view.kind == ViewKind::Composite => {
                    let w = this.parse_u32()?;
                    this.skip_ws();
                    let h = this.parse_u32()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cell_size = (w.max(100), h.max(100));
                    }
                }
                "cell" if view.kind == ViewKind::Composite => {
                    let key = this.parse_string()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cells.push(key);
                    }
                }
                _ => {
                    return Err(this.error(format!("unknown view keyword '{}'", prop)));
                }
            }
            Ok(())
        })
    }

    fn parse_animation(&mut self, anim: &mut Animation) -> Result<(), ParseError> {
        self.parse_braced("animation", |this| {
            let kw = this.parse_ident()?;
            if kw == "frame" {
                let label = this.parse_string()?;
                let mut frame = AnimationFrame {
                    label,
                    includes: Vec::new(),
                    include_all: false,
                    highlights: Vec::new(),
                    states: Vec::new(),
                    notes: None,
                };
                this.parse_braced("frame", |this| {
                    let prop = this.parse_ident()?;
                    match prop.as_str() {
                        "include" => {
                            this.skip_ws();
                            if this.peek() == Some('*') {
                                this.advance();
                                frame.include_all = true;
                            } else {
                                // Read element/relationship references
                                // Could be: "element", "el1 -> el2", or comma-separated
                                let ref1 = this.parse_ident()?;
                                this.skip_ws();
                                if this.peek() == Some('-') {
                                    // relationship: ref1 -> ref2
                                    this.advance(); // -
                                    this.expect('>')?;
                                    let ref2 = this.parse_ident()?;
                                    let r1 = this.resolve_ref(&ref1);
                                    let r2 = this.resolve_ref(&ref2);
                                    frame.includes.push(format!("{} -> {}", r1, r2));
                                } else {
                                    frame.includes.push(this.resolve_ref(&ref1));
                                }
                            }
                        }
                        "highlight" => {
                            let target = this.parse_ident()?;
                            let resolved = this.resolve_ref(&target);
                            let mut hl = FrameHighlight {
                                target: resolved,
                                color: None,
                                line_width: None,
                                label: None,
                            };
                            // Check for -> (relationship highlight)
                            this.skip_ws();
                            if this.peek() == Some('-') {
                                this.advance();
                                this.expect('>')?;
                                let t2 = this.parse_ident()?;
                                hl.target = format!("{} -> {}", hl.target, this.resolve_ref(&t2));
                                // May chain: a -> b -> c
                                this.skip_ws();
                                while this.peek() == Some('-') {
                                    this.advance();
                                    this.expect('>')?;
                                    let tn = this.parse_ident()?;
                                    hl.target
                                        .push_str(&format!(" -> {}", this.resolve_ref(&tn)));
                                }
                            }
                            if this.peek_after_ws() == Some('{') {
                                this.parse_braced("highlight", |this| {
                                    let hp = this.parse_ident()?;
                                    match hp.as_str() {
                                        "color" => hl.color = Some(this.parse_string()?),
                                        "line-width" => {
                                            let v = this.parse_ident()?;
                                            hl.line_width = v.parse().ok();
                                        }
                                        "label" => hl.label = Some(this.parse_string()?),
                                        _ => {
                                            return Err(this.error(format!(
                                                "unknown highlight keyword '{}'",
                                                hp
                                            )));
                                        }
                                    }
                                    Ok(())
                                })?;
                            }
                            frame.highlights.push(hl);
                        }
                        "state" => {
                            let target = this.parse_ident()?;
                            let resolved = this.resolve_ref(&target);
                            let state_label = this.parse_string()?;
                            let mut state = FrameState {
                                target: resolved,
                                label: state_label,
                                color: None,
                                pulse: false,
                            };
                            if this.peek_after_ws() == Some('{') {
                                this.parse_braced("state", |this| {
                                    let sp = this.parse_ident()?;
                                    match sp.as_str() {
                                        "color" => state.color = Some(this.parse_string()?),
                                        "pulse" => {
                                            let v = this.parse_ident()?;
                                            state.pulse = v == "true";
                                        }
                                        _ => {
                                            if this.peek_after_ws() == Some('"') {
                                                this.parse_string()?;
                                            }
                                        }
                                    }
                                    Ok(())
                                })?;
                            }
                            frame.states.push(state);
                        }
                        "notes" => {
                            frame.notes = Some(this.parse_string()?);
                        }
                        _ => {
                            if this.peek_after_ws() == Some('"') {
                                this.parse_string()?;
                            } else if this.peek_after_ws() == Some('{') {
                                this.skip_block()?;
                            }
                        }
                    }
                    Ok(())
                })?;
                anim.frames.push(frame);
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }
}

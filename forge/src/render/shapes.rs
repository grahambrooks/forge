//! Per-shape renderers: the `render_node` dispatcher plus the concrete
//! shape implementations (person, box, stage, gate, cylinder, deployment
//! node, data-class shields).

use super::branching::render_branch;
use super::entity::render_entity_table;
use super::util::{css_kind, esc, kind_label, render_wrapped_text};
use super::{Colors, TM};
use crate::layout::LayoutNode;
use crate::model::ElementKind;
use crate::text::*;

pub(super) fn render_node(o: &mut Vec<String>, n: &LayoutNode, style: &str) {
    let css = css_kind(n.kind);
    let tag_cls = if n.tags.contains(&"database".to_string()) {
        " forge-element--database"
    } else {
        ""
    };

    o.push(format!(
        r#"    <g class="forge-element forge-element--{css}{tag_cls}" data-id="{}">"#,
        esc(&n.id)
    ));

    if n.tags.contains(&"data-entity".to_string()) {
        render_entity_table(o, n);
    } else if n.kind == ElementKind::Branch {
        render_branch(o, n);
    } else if n.kind == ElementKind::DeploymentNode {
        render_deployment_node(o, n);
    } else if n.kind == ElementKind::Gate {
        render_gate(o, n);
    } else if n.kind == ElementKind::Person {
        render_person(o, n, style);
    } else if n.tags.contains(&"database".to_string()) {
        render_cylinder(o, n, style);
    } else if n.kind == ElementKind::Stage {
        render_stage(o, n);
    } else {
        render_box(o, n);
    }

    // Data classification shields overlay the top-right corner of any
    // element with non-empty data_classes. Rendered last so they sit on
    // top of the shape — databases, boxes, cylinders, and deployment
    // nodes all get the same treatment.
    if !n.data_classes.is_empty() {
        render_data_class_shields(o, n);
    }

    o.push("    </g>".into());
}

/// Render a horizontal row of shield badges for each data class at the
/// element's top-right corner. Colours are curated for well-known classes
/// (pii, financial, secret, public, internal); unknown values fall back to
/// a neutral grey badge.
fn render_data_class_shields(o: &mut Vec<String>, n: &LayoutNode) {
    const SHIELD_W: f64 = 18.0;
    const SHIELD_H: f64 = 22.0;
    const SHIELD_GAP: f64 = 4.0;

    let r = &n.rect;
    // Anchor: top-right corner, shields stack leftward. Each shield's
    // bottom-center sits a few pixels below the element's top edge so
    // the shield overlaps the border slightly.
    let top_y = r.y - SHIELD_H / 3.0;
    let mut right_x = r.x + r.w - 6.0;

    for class in &n.data_classes {
        let (color, letter) = data_class_style(class);
        let left_x = right_x - SHIELD_W;
        let shield_cx = left_x + SHIELD_W / 2.0;
        let text_y = top_y + SHIELD_H * 0.68;

        // Shield path: rectangle that tapers to a rounded point at the
        // bottom.
        let left = left_x;
        let right = left_x + SHIELD_W;
        let top = top_y;
        let mid = top_y + SHIELD_H * 0.55;
        let bot = top_y + SHIELD_H;
        let slug = data_class_slug(class);
        let title = esc(class);
        o.push(format!(
            r##"      <path d="M {left:.1} {top:.1} L {right:.1} {top:.1} L {right:.1} {mid:.1} Q {right:.1} {bot:.1} {shield_cx:.1} {bot:.1} Q {left:.1} {bot:.1} {left:.1} {mid:.1} Z" fill="{color}" stroke="#1f2937" stroke-width="1" class="forge-dataclass forge-dataclass--{slug}"><title>{title}</title></path>"##
        ));
        o.push(format!(
            r##"      <text x="{shield_cx:.1}" y="{text_y:.1}" class="forge-dataclass-label" text-anchor="middle" font-size="12" font-weight="700" fill="#ffffff">{letter}</text>"##
        ));

        right_x = left_x - SHIELD_GAP;
    }
}

/// Well-known data class → (fill colour, single-letter badge).
fn data_class_style(class: &str) -> (&'static str, &'static str) {
    match class.to_ascii_lowercase().as_str() {
        "pii" => ("#8b5cf6", "P"),       // purple
        "financial" => ("#d97706", "F"), // gold
        "public" => ("#16a34a", "P"),    // green
        "secret" => ("#dc2626", "S"),    // red
        "internal" => ("#6b7280", "I"),  // grey
        _ => ("#6b7280", "?"),           // unknown → neutral grey
    }
}

/// CSS-friendly slug for the data class so users can override styles per
/// class via `.forge-dataclass--pii { ... }` rules.
fn data_class_slug(class: &str) -> String {
    class
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn render_person(o: &mut Vec<String>, n: &LayoutNode, style: &str) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;

    let head_r = 20.0;
    let head_cy = r.y + head_r + 2.0;
    let shoulder_y = head_cy + head_r + 2.0;
    let shoulder_w = 36.0;
    let box_y = shoulder_y + 18.0;
    let box_h = (r.y + r.h - box_y).max(30.0);

    if style == "outline" {
        let stroke = Colors::PERSON_STROKE;
        let bx = r.x;
        let bw = r.w;
        let rad = 8.0;
        let sl = cx - shoulder_w;
        let sr = cx + shoulder_w;
        let bot = box_y + box_h;

        o.push(format!(
            r#"      <path d="M {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} L {:.0} {:.0} L {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} L {:.0} {:.0} L {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Z" fill="none" stroke="{}" stroke-width="2" />"#,
            bx + rad, bot,
            bx, bot, bx, bot - rad,
            bx, box_y,
            sl, box_y,
            sl, shoulder_y, cx, shoulder_y,
            sr, shoulder_y, sr, box_y,
            bx + bw, box_y,
            bx + bw, bot - rad,
            bx + bw, bot, bx + bw - rad, bot,
            stroke
        ));
        o.push(format!(
            r#"      <circle cx="{:.0}" cy="{:.0}" r="{}" fill="none" stroke="{}" stroke-width="2" />"#,
            cx, head_cy, head_r, stroke
        ));
    } else {
        o.push(format!(
            r#"      <circle cx="{:.0}" cy="{:.0}" r="{}" class="forge-person-head" />"#,
            cx, head_cy, head_r
        ));
        let sl = cx - shoulder_w;
        let sr = cx + shoulder_w;
        o.push(format!(
            r#"      <path d="M {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Z" class="forge-person-body" />"#,
            sl, box_y, sl, shoulder_y, cx, shoulder_y, sr, shoulder_y, sr, box_y
        ));
        o.push(format!(
            r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" class="forge-shadow" />"#,
            r.x, box_y, r.w, box_h
        ));
    }

    // Text
    let ty = box_y + 20.0;
    let max_text_w = r.w - 20.0;
    let name_w = TM.measure(&n.label, &FONT_NAME);
    if name_w > max_text_w {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" textLength="{:.0}" lengthAdjust="spacingAndGlyphs">{}</text>"#,
            cx, ty, max_text_w, esc(&n.label)
        ));
    } else {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name">{}</text>"#,
            cx,
            ty,
            esc(&n.label)
        ));
    }
    if ty + 16.0 < r.y + r.h - 4.0 {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--kind">[Person]</text>"#,
            cx,
            ty + 16.0
        ));
    }
    if let Some(ref desc) = n.description {
        render_wrapped_text(o, cx, ty + 32.0, max_text_w, desc, "forge-label--desc");
    }
}

fn render_box(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;
    let max_text_w = r.w - 40.0; // must match BOX_PAD_X * 2.0 in layout

    o.push(format!(
        r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" class="forge-shadow" />"#,
        r.x, r.y, r.w, r.h
    ));

    // Compute all text lines with wrapping
    struct TextLine {
        cls: &'static str,
        text: String,
    }
    let mut lines: Vec<TextLine> = Vec::new();
    // Max lines that fit vertically (each line ~16px, with padding)
    let max_lines = ((r.h - 8.0) / FONT_NAME.line_height).floor() as usize;

    lines.push(TextLine {
        cls: "name",
        text: n.label.clone(),
    });
    if lines.len() < max_lines {
        if let Some(kl) = kind_label(n.kind) {
            lines.push(TextLine {
                cls: "kind",
                text: format!("[{}]", kl),
            });
        }
    }
    if lines.len() < max_lines {
        if let Some(ref desc) = n.description {
            let wrapped = TM.wrap(desc, max_text_w, &FONT_DESC);
            let take = (max_lines - lines.len()).min(3);
            for line in wrapped.into_iter().take(take) {
                lines.push(TextLine {
                    cls: "desc",
                    text: line,
                });
            }
        }
    }
    if lines.len() < max_lines {
        if let Some(ref sub) = n.sublabel {
            lines.push(TextLine {
                cls: "tech",
                text: sub.clone(),
            });
        }
    }

    let total_h = lines.len() as f64 * FONT_NAME.line_height;
    let mut ty = r.y + (r.h - total_h) / 2.0 + 14.0;

    for line in &lines {
        let spec = match line.cls {
            "name" => &FONT_NAME,
            "desc" => &FONT_DESC,
            "tech" => &FONT_TECH,
            "kind" => &FONT_KIND,
            _ => &FONT_DESC,
        };
        let text_w = TM.measure(&line.text, spec);
        if text_w > max_text_w {
            // Constrain text to fit within box using SVG textLength
            o.push(format!(
                r#"      <text x="{:.0}" y="{:.0}" class="forge-label--{}" textLength="{:.0}" lengthAdjust="spacingAndGlyphs">{}</text>"#,
                cx, ty, line.cls, max_text_w, esc(&line.text)
            ));
        } else {
            o.push(format!(
                r#"      <text x="{:.0}" y="{:.0}" class="forge-label--{}">{}</text>"#,
                cx,
                ty,
                line.cls,
                esc(&line.text)
            ));
        }
        ty += FONT_NAME.line_height;
    }
}

fn render_stage(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;

    o.push(format!(
        r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" class="forge-shadow" />"#,
        r.x, r.y, r.w, r.h
    ));

    let mut lines: Vec<(&str, String)> = vec![("name", n.label.clone())];
    if let Some(ref sub) = n.sublabel {
        lines.push(("tech", format!("[{}]", sub)));
    }

    let total_h = lines.len() as f64 * FONT_NAME.line_height;
    let mut ty = r.y + (r.h - total_h) / 2.0 + 14.0;

    for (cls, text) in &lines {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--{}">{}</text>"#,
            cx,
            ty,
            cls,
            esc(text)
        ));
        ty += 16.0;
    }
}

fn render_deployment_node(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    // Dashed border rect for the deployment node boundary
    o.push(format!(
        r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" />"#,
        r.x, r.y, r.w, r.h
    ));
    // Header label (top-left, constrained to box width)
    let lx = r.x + 10.0;
    let ly = r.y + 18.0;
    let max_label_w = r.w - 20.0;
    let name_w = TM.measure(&n.label, &FONT_DEPLOY_NAME);
    if name_w > max_label_w {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" text-anchor="start" textLength="{:.0}" lengthAdjust="spacingAndGlyphs">{}</text>"#,
            lx, ly, max_label_w, esc(&n.label)
        ));
    } else {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" text-anchor="start">{}</text>"#,
            lx, ly, esc(&n.label)
        ));
    }
    if let Some(ref sub) = n.sublabel {
        let sub_w = TM.measure(sub, &FONT_DEPLOY_TECH);
        if sub_w > max_label_w {
            o.push(format!(
                r#"      <text x="{:.0}" y="{:.0}" class="forge-label--tech" text-anchor="start" textLength="{:.0}" lengthAdjust="spacingAndGlyphs">{}</text>"#,
                lx, ly + 14.0, max_label_w, esc(sub)
            ));
        } else {
            o.push(format!(
                r#"      <text x="{:.0}" y="{:.0}" class="forge-label--tech" text-anchor="start">{}</text>"#,
                lx, ly + 14.0, esc(sub)
            ));
        }
    }
}

fn render_gate(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;
    let cy = r.y + r.h / 2.0;
    let pts = format!(
        "{:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}",
        cx,
        r.y,
        r.x + r.w,
        cy,
        cx,
        r.y + r.h,
        r.x,
        cy
    );
    o.push(format!(r#"      <polygon points="{}" />"#, pts));

    let words: Vec<&str> = n.label.split('-').collect();
    if n.label.len() <= 14 {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" dominant-baseline="central">{}</text>"#,
            cx, cy + 4.0, esc(&n.label)
        ));
    } else {
        let mid = words.len() / 2;
        let l1 = words[..mid].join("-");
        let l2 = words[mid..].join("-");
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" dominant-baseline="central">{}</text>"#,
            cx, cy - 4.0, esc(&l1)
        ));
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" dominant-baseline="central">{}</text>"#,
            cx, cy + 10.0, esc(&l2)
        ));
    }
}

fn render_cylinder(o: &mut Vec<String>, n: &LayoutNode, style: &str) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;
    let ry = 12.0;
    let rx_half = r.w / 2.0;
    let stroke = Colors::DATABASE_STROKE;

    if style == "outline" {
        let top_y = r.y + ry;
        let bot_y = r.y + r.h - ry;
        let lx = r.x;
        let rx_right = r.x + r.w;

        o.push(format!(
            r#"      <path d="M {:.0} {:.0} A {:.0} {} 0 0 1 {:.0} {:.0} L {:.0} {:.0} A {:.0} {} 0 0 1 {:.0} {:.0} Z" fill="none" stroke="{}" stroke-width="2" />"#,
            lx, top_y, rx_half, ry, rx_right, top_y,
            rx_right, bot_y, rx_half, ry, lx, bot_y,
            stroke
        ));
        o.push(format!(
            r#"      <path d="M {:.0} {:.0} A {:.0} {} 0 0 0 {:.0} {:.0}" fill="none" stroke="{}" stroke-width="2" />"#,
            lx, top_y, rx_half, ry, rx_right, top_y, stroke
        ));
    } else {
        let bg = Colors::DATABASE_BG;
        let sw = "1.5";
        o.push(format!(
            r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" fill="{}" stroke="{}" stroke-width="{}" rx="0" ry="0" />"#,
            r.x, r.y + ry, r.w, r.h - 2.0 * ry, bg, stroke, sw
        ));
        o.push(format!(
            r#"      <ellipse cx="{:.0}" cy="{:.0}" rx="{:.0}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
            cx, r.y + ry, rx_half, ry, bg, stroke, sw
        ));
        o.push(format!(
            r#"      <ellipse cx="{:.0}" cy="{:.0}" rx="{:.0}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
            cx, r.y + r.h - ry, rx_half, ry, bg, stroke, sw
        ));
        o.push(format!(
            r#"      <line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" stroke="{}" stroke-width="{}" />"#,
            r.x, r.y + ry, r.x, r.y + r.h - ry, stroke, sw
        ));
        o.push(format!(
            r#"      <line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" stroke="{}" stroke-width="{}" />"#,
            r.x + r.w, r.y + ry, r.x + r.w, r.y + r.h - ry, stroke, sw
        ));
    }

    // Center text vertically, only render lines that fit
    let mut text_lines: Vec<(&str, String)> = vec![("name", n.label.clone())];
    if let Some(ref sub) = n.sublabel {
        text_lines.push(("tech", sub.clone()));
    }
    text_lines.push(("kind", "[Database]".into()));

    let total_h = text_lines.len() as f64 * FONT_NAME.line_height;
    let usable_h = r.h - 24.0; // account for top/bottom ellipses
    let mut ty = r.y + (r.h - total_h.min(usable_h)) / 2.0 + 12.0;
    let max_text_w = r.w - 20.0;

    for (cls, text) in &text_lines {
        if ty > r.y + r.h - 8.0 {
            break;
        }
        let text_w = TM.measure(text, &FONT_NAME);
        if text_w > max_text_w {
            o.push(format!(
                r#"      <text x="{:.0}" y="{:.0}" class="forge-label--{}" textLength="{:.0}" lengthAdjust="spacingAndGlyphs">{}</text>"#,
                cx, ty, cls, max_text_w, esc(text)
            ));
        } else {
            o.push(format!(
                r#"      <text x="{:.0}" y="{:.0}" class="forge-label--{}">{}</text>"#,
                cx,
                ty,
                cls,
                esc(text)
            ));
        }
        ty += FONT_NAME.line_height;
    }
}

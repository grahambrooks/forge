//! Entity-relationship table rendering (data model views).

use super::util::esc;
use crate::layout::LayoutNode;

pub(super) fn render_entity_table(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let header_h = 32.0;
    let row_h = 20.0;
    let pad = 10.0;

    // Outer box
    o.push(format!(
        r#"      <rect class="forge-entity-box" x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" />"#,
        r.x, r.y, r.w, r.h
    ));

    // Header background
    o.push(format!(
        r#"      <rect class="forge-entity-header" x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" rx="4" ry="0" />"#,
        r.x, r.y, r.w, header_h
    ));
    // Header corners fix (cover bottom radius)
    o.push(format!(
        r#"      <rect class="forge-entity-header" x="{:.0}" y="{:.0}" width="{:.0}" height="4" />"#,
        r.x,
        r.y + header_h - 4.0,
        r.w
    ));

    // Entity name in header
    o.push(format!(
        r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name forge-entity-header-text">{}</text>"#,
        r.x + r.w / 2.0,
        r.y + 21.0,
        esc(&n.label)
    ));

    // Owner subtitle
    if let Some(ref sub) = n.sublabel {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-entity-sub" font-size="9" text-anchor="end">{}</text>"#,
            r.x + r.w - pad,
            r.y + 21.0,
            esc(sub)
        ));
    }

    // Field rows from description (newline-separated)
    if let Some(ref desc) = n.description {
        let fields: Vec<&str> = desc.split('\n').collect();
        let mut fy = r.y + header_h + pad + 12.0;
        for field in fields {
            // Split "name: type (constraints)"
            let (fname, frest) = field.split_once(':').unwrap_or((field, ""));
            let frest = frest.trim();

            // Row separator line
            if fy > r.y + header_h + pad + 12.0 {
                o.push(format!(
                    r#"      <line class="forge-entity-sep" x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" />"#,
                    r.x + pad,
                    fy - row_h + 4.0,
                    r.x + r.w - pad,
                    fy - row_h + 4.0
                ));
            }

            // Field name (left-aligned, bold)
            o.push(format!(
                r#"      <text class="forge-entity-field" x="{:.0}" y="{:.0}" text-anchor="start">{}</text>"#,
                r.x + pad,
                fy,
                esc(fname.trim())
            ));

            // Field type + constraints (right-aligned)
            o.push(format!(
                r#"      <text class="forge-entity-type" x="{:.0}" y="{:.0}" text-anchor="end">{}</text>"#,
                r.x + r.w - pad,
                fy,
                esc(frest)
            ));

            fy += row_h;
        }
    }
}

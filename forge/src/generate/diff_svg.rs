use crate::diff::DiffResult;
pub(super) fn inject_diff_highlights(svg: &str, diff: &DiffResult) -> String {
    let mut result = svg.to_string();
    for id in &diff.added_ids {
        let needle = format!(r#"data-id="{}""#, id);
        if let Some(pos) = result.find(&needle) {
            // Find the preceding class=" in this <g> tag
            if let Some(class_start) = result[..pos].rfind("class=\"") {
                let insert_at = class_start + 7; // after class="
                result.insert_str(insert_at, "forge-diff-highlight--added ");
            }
        }
    }
    for id in &diff.modified_ids {
        let needle = format!(r#"data-id="{}""#, id);
        if let Some(pos) = result.find(&needle) {
            if let Some(class_start) = result[..pos].rfind("class=\"") {
                let insert_at = class_start + 7;
                result.insert_str(insert_at, "forge-diff-highlight--modified ");
            }
        }
    }
    // Inject diff CSS into the SVG <style> block
    if !diff.added_ids.is_empty() || !diff.modified_ids.is_empty() {
        let diff_css = r#"
    .forge-diff-highlight--added > rect, .forge-diff-highlight--added > circle, .forge-diff-highlight--added > path { stroke: #16a34a !important; stroke-width: 3 !important; }
    .forge-diff-highlight--modified > rect, .forge-diff-highlight--modified > circle, .forge-diff-highlight--modified > path { stroke: #d97706 !important; stroke-width: 3 !important; }
"#;
        if let Some(style_end) = result.find("</style>") {
            result.insert_str(style_end, diff_css);
        }
    }
    result
}

// ─── Utilities ───────────────────────────────────────────────────

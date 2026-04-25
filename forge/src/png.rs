//! SVG → PNG renderer.
//!
//! Wraps `resvg` with the bundled Inter fonts so output is byte-identical
//! across platforms (no reliance on system font discovery). Also resolves
//! the CSS custom properties Forge emits — `usvg` doesn't expand
//! `var(--name)` references, so we substitute them inline first.

use std::collections::HashMap;
use std::sync::Arc;

use resvg::{tiny_skia, usvg};

const INTER_REGULAR: &[u8] = include_bytes!("fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("fonts/Inter-Bold.ttf");

/// Rasterise an SVG document into a PNG byte buffer.
///
/// `scale` multiplies the SVG's intrinsic pixel size — pass `2.0` for
/// retina-quality output, `1.0` for a 1:1 raster.
pub fn render(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(format!("invalid scale {scale}"));
    }

    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_data(INTER_REGULAR.to_vec());
    fontdb.load_font_data(INTER_BOLD.to_vec());
    fontdb.set_sans_serif_family("Inter");

    let opt = usvg::Options {
        fontdb: Arc::new(fontdb),
        ..usvg::Options::default()
    };

    let prepared = inline_css_variables(svg);
    let tree = usvg::Tree::from_str(&prepared, &opt).map_err(|e| format!("parse svg: {e}"))?;

    let size = tree.size().to_int_size();
    let w = ((size.width() as f32) * scale).round().max(1.0) as u32;
    let h = ((size.height() as f32) * scale).round().max(1.0) as u32;

    let mut pixmap =
        tiny_skia::Pixmap::new(w, h).ok_or_else(|| format!("cannot allocate pixmap {w}×{h}"))?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap.encode_png().map_err(|e| format!("encode png: {e}"))
}

/// Resolve `var(--name)` / `var(--name, fallback)` references against the
/// `--name: value;` declarations defined in the embedded `<style>` block,
/// and drop `@media(prefers-color-scheme:dark) { ... }` so PNGs always
/// render the light theme.
fn inline_css_variables(svg: &str) -> String {
    let stripped = strip_dark_mode_block(svg);
    let vars = collect_css_variables(&stripped);
    expand_var_calls(&stripped, &vars)
}

fn strip_dark_mode_block(svg: &str) -> String {
    let needle = "@media(prefers-color-scheme:dark)";
    let Some(start) = svg.find(needle) else {
        return svg.to_string();
    };
    let after = &svg[start..];
    let Some(open) = after.find('{') else {
        return svg.to_string();
    };
    let mut depth = 1;
    let bytes = after.as_bytes();
    let mut i = open + 1;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    if depth != 0 {
        return svg.to_string();
    }
    let mut out = String::with_capacity(svg.len());
    out.push_str(&svg[..start]);
    out.push_str(&after[i..]);
    out
}

fn collect_css_variables(svg: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    let bytes = svg.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let name_start = i + 2;
            let mut j = name_start;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-' || bytes[j] == b'_')
            {
                j += 1;
            }
            if j > name_start && j < bytes.len() && bytes[j] == b':' {
                let name = &svg[name_start..j];
                let mut k = j + 1;
                while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }
                let value_start = k;
                while k < bytes.len() && bytes[k] != b';' && bytes[k] != b'}' {
                    k += 1;
                }
                let value = svg[value_start..k].trim().to_string();
                if !value.is_empty() && !value.starts_with("var(") {
                    vars.entry(name.to_string()).or_insert(value);
                }
                i = k.max(i + 1);
                continue;
            }
        }
        i += 1;
    }
    vars
}

fn expand_var_calls(svg: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(svg.len());
    let bytes = svg.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"var(--") {
            let inner_start = i + 6;
            let mut j = inner_start;
            let mut depth = 1;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
                j += 1;
            }
            if depth == 0 {
                let inner = &svg[inner_start..j];
                let (name, fallback) = match inner.find(',') {
                    Some(idx) => (inner[..idx].trim(), Some(inner[idx + 1..].trim())),
                    None => (inner.trim(), None),
                };
                let resolved = vars
                    .get(name)
                    .map(String::as_str)
                    .or(fallback)
                    .unwrap_or("currentColor");
                out.push_str(resolved);
                i = j + 1;
                continue;
            }
        }
        out.push(svg[i..].chars().next().unwrap());
        i += svg[i..].chars().next().unwrap().len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10">
        <rect width="20" height="10" fill="white"/>
        <text x="2" y="8" font-family="Inter" font-size="8" fill="black">hi</text>
    </svg>"#;

    #[test]
    fn renders_minimal_svg_to_png() {
        let bytes = render(MIN_SVG, 1.0).expect("render");
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']), "PNG magic");
    }

    #[test]
    fn scale_changes_output_dimensions() {
        let small = render(MIN_SVG, 1.0).unwrap();
        let large = render(MIN_SVG, 4.0).unwrap();
        assert!(large.len() > small.len(), "4x raster should be larger");
    }

    #[test]
    fn rejects_non_positive_scale() {
        assert!(render(MIN_SVG, 0.0).is_err());
        assert!(render(MIN_SVG, -1.0).is_err());
        assert!(render(MIN_SVG, f32::NAN).is_err());
    }

    #[test]
    fn reports_parse_failure() {
        assert!(render("not svg", 1.0).is_err());
    }

    #[test]
    fn inlines_css_variables() {
        let svg = r#"<svg><style>.x{--fg:#abc;}.y{fill:var(--fg);}</style></svg>"#;
        let out = inline_css_variables(svg);
        assert!(out.contains("fill:#abc"), "got: {out}");
        assert!(!out.contains("var(--fg)"));
    }

    #[test]
    fn uses_fallback_when_variable_undefined() {
        let svg = r#"<svg><style>.y{fill:var(--missing, #f00);}</style></svg>"#;
        let out = inline_css_variables(svg);
        assert!(out.contains("fill:#f00"), "got: {out}");
    }

    #[test]
    fn drops_dark_mode_block() {
        let svg = "<svg><style>.x{--fg:#000;}\
            @media(prefers-color-scheme:dark){.x{--fg:#fff;}}\
            .y{fill:var(--fg);}</style></svg>";
        let out = inline_css_variables(svg);
        assert!(!out.contains("prefers-color-scheme"));
        assert!(out.contains("fill:#000"), "got: {out}");
    }

    #[test]
    fn renders_forge_svg_with_var_references() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20">
            <style>
                .pal { --fg: #08427B; }
                .label { font-family: Inter; font-size: 12px; fill: var(--fg); }
            </style>
            <g class="pal"><text x="2" y="14" class="label">hello</text></g>
        </svg>"##;
        let bytes = render(svg, 1.0).expect("render");
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}

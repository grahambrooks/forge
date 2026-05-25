//! SVG string-manipulation helpers shared by the animation transform.

pub(super) fn find_closing_g(svg: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut pos = start;
    while pos < svg.len() {
        if svg[pos..].starts_with("<g ") || svg[pos..].starts_with("<g>") {
            depth += 1;
            pos += 2;
        } else if svg[pos..].starts_with("</g>") {
            depth -= 1;
            if depth <= 0 {
                return Some(pos + 4);
            }
            pos += 4;
        } else {
            pos += 1;
        }
    }
    None
}

pub(super) fn extract_svg_height(svg: &str) -> f64 {
    if let Some(idx) = svg.find("height=\"") {
        let start = idx + 8;
        if let Some(end) = svg[start..].find('"') {
            return svg[start..start + end].parse().unwrap_or(400.0);
        }
    }
    400.0
}

pub(super) fn extract_svg_width(svg: &str) -> f64 {
    if let Some(idx) = svg.find("width=\"") {
        let start = idx + 7;
        if let Some(end) = svg[start..].find('"') {
            return svg[start..start + end].parse().unwrap_or(800.0);
        }
    }
    800.0
}

pub(super) fn adjust_svg_height(svg: &str, extra: f64) -> String {
    let old_h = extract_svg_height(svg);
    let new_h = old_h + extra;
    let mut result = svg.to_string();

    // Update height attribute
    let old_h_str = format!("height=\"{}\"", old_h as u32);
    let new_h_str = format!("height=\"{}\"", new_h as u32);
    result = result.replacen(&old_h_str, &new_h_str, 1);

    // Update viewBox
    let old_vb = format!("0 0 {} {}", extract_svg_width(svg) as u32, old_h as u32);
    let new_vb = format!("0 0 {} {}", extract_svg_width(svg) as u32, new_h as u32);
    result = result.replacen(&old_vb, &new_vb, 1);

    // Update background rect
    let old_bg = format!("height=\"{}\" fill", old_h as u32);
    let new_bg = format!("height=\"{}\" fill", new_h as u32);
    result = result.replacen(&old_bg, &new_bg, 1);

    result
}

pub(super) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

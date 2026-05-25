//! Static SVG → animated SVG: injects the CSS, builds per-frame element/edge
//! visibility maps, wraps matching `<g>` blocks with frame groups, and adds
//! the navigation dots.

use std::collections::HashMap;

use crate::model::*;

use super::derive::derive_dynamic_animation;
use super::style::ANIMATION_CSS;
use super::util::{adjust_svg_height, esc, extract_svg_height, extract_svg_width, find_closing_g};

/// Transform a static SVG into an animated one based on the view's animation
/// definition. Returns the animated SVG string.
pub fn animate_svg(svg: &str, view: &View, model: &Model) -> String {
    // Dynamic views with no explicit `animation { frames }` block
    // auto-generate one frame per unique step number drawn from the
    // ordered relationships in scope. The derived animation becomes the
    // effective one for the rest of this function so a user stepping
    // through the view sees the flow play out from step 1 to step N.
    let derived = derive_dynamic_animation(view, model);
    let anim: &Animation = derived.as_ref().unwrap_or(&view.animation);
    if anim.is_empty() {
        return svg.to_string();
    }

    let result = inject_animation_chrome(svg, anim.frames.len());
    let (element_map, rel_map) = build_frame_maps(anim, model);
    let wrapped = wrap_frame_groups(&result, anim, &element_map, &rel_map);
    let with_nav = inject_frame_controls(&wrapped, anim);
    adjust_svg_height(&with_nav, 40.0)
}

/// Inject the animation CSS into `<style>` and tag `<svg>` with the
/// `forge-animated` class plus frame metadata.
fn inject_animation_chrome(svg: &str, frame_count: usize) -> String {
    let mut result = svg.to_string();
    if let Some(style_end) = result.find("</style>") {
        result.insert_str(style_end, ANIMATION_CSS);
    }
    result.replacen(
        "class=\"forge-diagram\"",
        &format!(
            "class=\"forge-diagram forge-animated\" data-frames=\"{}\" data-current=\"0\"",
            frame_count
        ),
        1,
    )
}

/// Compute (element_id → first_frame_idx) and (edge_key → first_frame_idx)
/// maps so each `<g>` can be wrapped in its earliest-appearing frame.
fn build_frame_maps(
    anim: &Animation,
    model: &Model,
) -> (HashMap<String, usize>, HashMap<String, usize>) {
    let mut element_frame_map: HashMap<String, usize> = HashMap::new();
    let mut rel_frame_map: HashMap<String, usize> = HashMap::new();

    for (frame_idx, frame) in anim.frames.iter().enumerate() {
        if frame.include_all {
            for el in model.elements.values() {
                element_frame_map.entry(el.id.clone()).or_insert(frame_idx);
            }
            for rel in &model.relationships {
                let key = format!("{} -> {}", rel.frm, rel.to);
                rel_frame_map.entry(key).or_insert(frame_idx);
            }
        } else {
            for inc in &frame.includes {
                if inc.contains(" -> ") {
                    rel_frame_map.entry(inc.clone()).or_insert(frame_idx);
                } else {
                    element_frame_map.entry(inc.clone()).or_insert(frame_idx);
                }
            }
        }
    }

    (element_frame_map, rel_frame_map)
}

/// Walk the SVG looking for `<g … data-id="…">` and `<g … data-from="…" data-to="…">`
/// blocks; wrap each matched group in a `<g class="forge-frame" data-frame="N">`
/// shell so the playback script can toggle visibility per-frame.
fn wrap_frame_groups(
    svg: &str,
    anim: &Animation,
    element_map: &HashMap<String, usize>,
    rel_map: &HashMap<String, usize>,
) -> String {
    let mut output = String::with_capacity(svg.len() + 2000);
    let mut pos = 0;
    while pos < svg.len() {
        let next_element = svg[pos..].find("data-id=\"").map(|i| (pos + i, true));
        let next_rel = svg[pos..].find("data-from=\"").map(|i| (pos + i, false));

        let next = match (next_element, next_rel) {
            (Some((ei, _)), Some((ri, _))) => {
                if ei <= ri {
                    Some((ei, true))
                } else {
                    Some((ri, false))
                }
            }
            (Some(e), None) => Some(e),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        };

        let Some((abs_idx, is_element)) = next else {
            output.push_str(&svg[pos..]);
            break;
        };

        if is_element {
            if let Some(new_pos) =
                wrap_element_group(svg, abs_idx, anim, element_map, &mut output, pos)
            {
                pos = new_pos;
                continue;
            }
            output.push_str(&svg[pos..abs_idx + 9]);
            pos = abs_idx + 9;
        } else {
            if let Some(new_pos) =
                wrap_relationship_group(svg, abs_idx, anim, rel_map, &mut output, pos)
            {
                pos = new_pos;
                continue;
            }
            output.push_str(&svg[pos..abs_idx + 11]);
            pos = abs_idx + 11;
        }
    }
    output
}

/// Wrap the `<g … data-id="ID" …>…</g>` block starting near `abs_idx`. On
/// success, appends the rewritten chunk to `output` and returns the new
/// `pos`. Returns `None` if the id isn't in `element_map` or the group is
/// malformed (caller falls back to advancing past the attribute).
fn wrap_element_group(
    svg: &str,
    abs_idx: usize,
    anim: &Animation,
    element_map: &HashMap<String, usize>,
    output: &mut String,
    pos: usize,
) -> Option<usize> {
    let id_start = abs_idx + 9;
    let id_end = svg[id_start..].find('"')?;
    let element_id = &svg[id_start..id_start + id_end];

    let &frame_idx = element_map.get(element_id)?;
    let g_start = svg[..abs_idx].rfind("<g ").unwrap_or(abs_idx);
    let g_end = find_closing_g(svg, abs_idx)?;

    output.push_str(&svg[pos..g_start]);

    let frame = &anim.frames[frame_idx];
    let mut extra_cls = String::from("forge-enter");
    let mut style_attr = String::new();
    for hl in &frame.highlights {
        if hl.target == element_id || hl.target.contains(element_id) {
            extra_cls.push_str(" forge-highlight");
            if let Some(ref color) = hl.color {
                style_attr = format!(" style=\"--forge-hl-color: {}\"", color);
            }
        }
    }
    for st in &frame.states {
        if st.target == element_id && st.pulse {
            extra_cls.push_str(" forge-pulse");
        }
    }

    output.push_str(&format!(
        "<g class=\"forge-frame\" data-frame=\"{}\" data-label=\"{}\"{}>",
        frame_idx,
        esc(&anim.frames[frame_idx].label),
        style_attr,
    ));
    let element_g = &svg[g_start..g_end];
    let patched = element_g.replacen("class=\"", &format!("class=\"{} ", extra_cls), 1);
    output.push_str(&patched);
    output.push_str("</g>");

    Some(g_end)
}

/// Wrap the `<g … data-from="A" data-to="B" …>…</g>` block starting near
/// `abs_idx`. Same shape as [`wrap_element_group`].
fn wrap_relationship_group(
    svg: &str,
    abs_idx: usize,
    anim: &Animation,
    rel_map: &HashMap<String, usize>,
    output: &mut String,
    pos: usize,
) -> Option<usize> {
    let from_start = abs_idx + 11;
    let from_end = svg[from_start..].find('"')?;
    let from_id = svg[from_start..from_start + from_end].to_string();

    let search_area = &svg[from_start + from_end..];
    let to_attr = search_area.find("data-to=\"")?;
    let to_start = from_start + from_end + to_attr + 9;
    let to_end = svg[to_start..].find('"')?;
    let to_id = svg[to_start..to_start + to_end].to_string();

    let rel_key = format!("{} -> {}", from_id, to_id);
    let &frame_idx = rel_map.get(&rel_key)?;
    let g_start = svg[..abs_idx].rfind("<g ").unwrap_or(abs_idx);
    let g_end = find_closing_g(svg, abs_idx)?;

    output.push_str(&svg[pos..g_start]);

    let frame = &anim.frames[frame_idx];
    let mut extra_cls = String::from("forge-enter");
    let mut style_attr = String::new();
    for hl in &frame.highlights {
        if hl.target.contains(&from_id) && hl.target.contains(&to_id) {
            extra_cls.push_str(" forge-highlight");
            if let Some(ref color) = hl.color {
                style_attr = format!(" style=\"--forge-hl-color: {}\"", color);
            }
        }
    }

    output.push_str(&format!(
        "<g class=\"forge-frame\" data-frame=\"{}\"{}>",
        frame_idx, style_attr,
    ));
    let rel_g = &svg[g_start..g_end];
    let patched = rel_g.replacen("class=\"", &format!("class=\"{} ", extra_cls), 1);
    output.push_str(&patched);
    output.push_str("</g>");

    Some(g_end)
}

/// Append a row of clickable navigation dots (plus the active-frame label)
/// just before `</svg>` so users can step through frames visually.
fn inject_frame_controls(svg: &str, anim: &Animation) -> String {
    let Some(svg_end) = svg.rfind("</svg>") else {
        return svg.to_string();
    };
    let frame_count = anim.frames.len();
    let dots_y = extract_svg_height(svg) - 20.0;
    let dots_start_x = extract_svg_width(svg) / 2.0 - (frame_count as f64 * 15.0) / 2.0;

    let mut nav = String::new();
    nav.push_str(&format!(
        "  <g class=\"forge-frame-controls\" transform=\"translate({:.0},{:.0})\">",
        dots_start_x, dots_y
    ));
    for (i, _frame) in anim.frames.iter().enumerate() {
        let cx = i as f64 * 18.0 + 6.0;
        let cls = if i == 0 {
            "forge-frame-dot forge-frame-dot--active"
        } else {
            "forge-frame-dot"
        };
        nav.push_str(&format!(
            "<circle class=\"{}\" data-frame=\"{}\" cx=\"{:.0}\" cy=\"0\" r=\"5\" />",
            cls, i, cx
        ));
    }
    let label_x = frame_count as f64 * 18.0 / 2.0;
    nav.push_str(&format!(
        "<text class=\"forge-frame-label\" x=\"{:.0}\" y=\"18\">{}</text>",
        label_x,
        esc(&anim.frames[0].label)
    ));
    nav.push_str("</g>\n");

    let mut output = svg.to_string();
    output.insert_str(svg_end, &nav);
    output
}

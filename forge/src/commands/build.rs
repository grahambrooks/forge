//! `forge build` — parse a `.forge` file and render SVG/PNG diagrams.

use std::fs;
use std::path::Path;

use crate::{animate, model, png, render};

use super::util::{die, load_model};

pub fn cmd_build(
    source: &Path,
    view: Option<&str>,
    out: &Path,
    style: &str,
    format: &str,
    scale: f32,
) {
    if style != "filled" && style != "outline" {
        die("--style must be 'filled' or 'outline'");
    }
    let (write_svg, write_png) = match format {
        "svg" => (true, false),
        "png" => (false, true),
        "both" => (true, true),
        _ => die("--format must be 'svg', 'png', or 'both'"),
    };

    let model = load_model(source);
    eprintln!("Parsed model: \"{}\"", model.name);
    eprintln!("  Elements: {}", model.elements.len());
    eprintln!("  Relationships: {}", model.relationships.len());
    eprintln!("  Views: {}", model.views.len());

    fs::create_dir_all(out).unwrap_or_else(|e| die(&format!("creating output dir: {}", e)));

    for v in &model.views {
        if let Some(filter) = view {
            if v.key != filter {
                continue;
            }
        }

        let static_svg = render::render_view(&model, v, style);
        let is_animated = !v.animation.is_empty() || v.kind == model::ViewKind::Dynamic;

        if write_svg {
            let svg = if is_animated {
                animate::animate_svg(&static_svg, v, &model)
            } else {
                static_svg.clone()
            };
            let path = out.join(format!("{}.svg", v.key));
            fs::write(&path, &svg)
                .unwrap_or_else(|e| die(&format!("writing {}: {}", path.display(), e)));
            eprintln!("  Wrote: {}", path.display());
        }
        if write_png {
            // Always rasterise the static SVG. The animated wrapper hides
            // every frame initially (opacity 0, toggled by JS at runtime),
            // so an animated SVG → PNG would render a near-empty image.
            let png = png::render(&static_svg, scale)
                .unwrap_or_else(|e| die(&format!("rendering {} png: {}", v.key, e)));
            let path = out.join(format!("{}.png", v.key));
            fs::write(&path, &png)
                .unwrap_or_else(|e| die(&format!("writing {}: {}", path.display(), e)));
            eprintln!("  Wrote: {} ({} bytes)", path.display(), png.len());
        }
    }
    eprintln!("Done.");
}

//! Forge animation — transforms a static SVG into an animated frame-based SVG.
//!
//! Takes a rendered SVG and an Animation definition, then wraps elements
//! in frame groups with CSS transitions for step-by-step reveal.

use crate::model::*;

/// CSS and JS for animated SVGs.
const ANIMATION_CSS: &str = r##"
    /* Animation frames */
    .forge-frame { opacity: 0; pointer-events: none; transition: opacity 0.4s ease-in-out; }
    .forge-frame--active { opacity: 1; pointer-events: auto; }

    /* Pulse effect */
    @keyframes forge-pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }
    .forge-pulse { animation: forge-pulse 1.5s ease-in-out infinite; }

    /* Fade-in for new elements */
    @keyframes forge-fade-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
    .forge-enter { animation: forge-fade-in 0.4s ease-out forwards; }

    /* Highlight glow */
    .forge-highlight > rect, .forge-highlight > circle, .forge-highlight > path,
    .forge-highlight > line, .forge-highlight > ellipse {
      filter: drop-shadow(0 0 6px var(--forge-hl-color, #E65100));
      stroke: var(--forge-hl-color, #E65100) !important;
      stroke-width: 3 !important;
    }

    /* State badge */
    .forge-state-badge rect { rx: 10; ry: 10; }
    .forge-state-badge text { font-size: 10px; font-weight: 600; text-anchor: middle; }

    /* Frame controls */
    .forge-frame-controls { cursor: pointer; }
    .forge-frame-dot { fill: #ccc; transition: fill 0.2s; }
    .forge-frame-dot--active { fill: #1168BD; }
    .forge-frame-label { font-size: 12px; fill: #555; text-anchor: middle; font-weight: 500; }
"##;

/// Minimal JS for keyboard/click frame navigation (<1KB).
const PLAYBACK_JS: &str = r##"
(function(){
  document.querySelectorAll('.forge-animated').forEach(function(svg){
    var frames=parseInt(svg.dataset.frames||'0'),cur=0;
    function show(n){
      cur=Math.max(0,Math.min(n,frames-1));
      svg.dataset.current=''+cur;
      svg.querySelectorAll('.forge-frame').forEach(function(f){
        f.classList.toggle('forge-frame--active',parseInt(f.dataset.frame)<=cur);
      });
      svg.querySelectorAll('.forge-frame-dot').forEach(function(d,i){
        d.classList.toggle('forge-frame-dot--active',i<=cur);
      });
      var lbl=svg.querySelector('.forge-frame-label');
      if(lbl){
        var active=svg.querySelector('.forge-frame[data-frame="'+cur+'"]');
        lbl.textContent=active?active.dataset.label:'';
      }
    }
    show(0);
    svg.addEventListener('click',function(){show(cur+1>=frames?0:cur+1);});
    document.addEventListener('keydown',function(e){
      if(e.key==='ArrowRight'||e.key===' ')show(cur+1);
      else if(e.key==='ArrowLeft')show(cur-1);
    });
    svg.querySelectorAll('.forge-frame-dot').forEach(function(d,i){
      d.addEventListener('click',function(e){e.stopPropagation();show(i);});
    });
  });
})();
"##;

/// For a dynamic view with no explicit animation, synthesize one frame per
/// unique step number drawn from the ordered relationships in the view's
/// scope. Each frame is cumulative: frame N includes every element and
/// relationship with `order <= N`, so a viewer stepping through the view
/// watches the flow progress from step 1 to step N.
///
/// Returns `None` when the view isn't dynamic, already carries an explicit
/// animation, or has no ordered relationships.
fn derive_dynamic_animation(view: &View, model: &Model) -> Option<Animation> {
    if view.kind != ViewKind::Dynamic || !view.animation.is_empty() {
        return None;
    }

    // Collect ordered relationships whose endpoints are in scope. For a
    // dynamic view, scope is the `system` id; include everything whose
    // parent (recursively) rolls up to that system.
    let scope_id = view.scope.as_deref().unwrap_or("");
    let in_scope = |id: &str| -> bool {
        if id == scope_id {
            return true;
        }
        let mut cur = id;
        while let Some(el) = model.elements.get(cur) {
            if el.id == scope_id {
                return true;
            }
            match el.parent.as_deref() {
                Some(p) => cur = p,
                None => return false,
            }
        }
        false
    };

    let mut ordered: Vec<(&Relationship, u32)> = model
        .relationships
        .iter()
        .filter_map(|r| r.order.map(|o| (r, o)))
        .filter(|(r, _)| in_scope(&r.frm) && in_scope(&r.to))
        .collect();
    if ordered.is_empty() {
        return None;
    }
    ordered.sort_by_key(|(_, o)| *o);

    // Unique, sorted step numbers. Gaps are allowed; we emit one frame
    // per distinct value in ascending order.
    let mut seen_steps = Vec::new();
    for (_, o) in &ordered {
        if !seen_steps.contains(o) {
            seen_steps.push(*o);
        }
    }

    let mut frames = Vec::with_capacity(seen_steps.len());
    for &step in &seen_steps {
        let mut includes: Vec<String> = Vec::new();
        for (rel, o) in &ordered {
            if *o > step {
                break;
            }
            // Always include both endpoints so nodes appear at the same
            // time as the arrow that uses them.
            if !includes.contains(&rel.frm) {
                includes.push(rel.frm.clone());
            }
            if !includes.contains(&rel.to) {
                includes.push(rel.to.clone());
            }
            // Include the relationship itself as "frm -> to" so the edge
            // is revealed alongside its endpoints.
            let edge_key = format!("{} -> {}", rel.frm, rel.to);
            if !includes.contains(&edge_key) {
                includes.push(edge_key);
            }
        }
        frames.push(AnimationFrame {
            label: format!("Step {step}"),
            includes,
            include_all: false,
            highlights: Vec::new(),
            states: Vec::new(),
            notes: None,
        });
    }

    Some(Animation { frames })
}

/// Transform a static SVG into an animated one based on the view's animation definition.
/// Returns the animated SVG string.
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

    let frame_count = anim.frames.len();
    let mut result = svg.to_string();

    // 1. Add animation CSS to the <style> block
    if let Some(style_end) = result.find("</style>") {
        result.insert_str(style_end, ANIMATION_CSS);
    }

    // 2. Add forge-animated class and data attributes to <svg>
    result = result.replacen(
        "class=\"forge-diagram\"",
        &format!(
            "class=\"forge-diagram forge-animated\" data-frames=\"{}\" data-current=\"0\"",
            frame_count
        ),
        1,
    );

    // 3. Wrap elements in frame visibility groups
    // Strategy: for each frame, find elements by data-id and wrap them
    // We work backwards to not invalidate earlier positions

    // Build cumulative element sets per frame
    let mut cumulative_ids: Vec<Vec<String>> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for frame in &anim.frames {
        if frame.include_all {
            // All elements visible from this frame onward
            for el in model.elements.values() {
                seen.insert(el.id.clone());
            }
        } else {
            for inc in &frame.includes {
                if inc.contains(" -> ") {
                    // Relationship include — we track the data-from/data-to
                    seen.insert(inc.clone());
                } else {
                    seen.insert(inc.clone());
                }
            }
        }
        cumulative_ids.push(seen.iter().cloned().collect());
    }

    // 4. For each element <g> with data-id, determine which frame it first appears in
    // and inject a wrapping <g class="forge-frame" data-frame="N">
    let mut element_frame_map: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (frame_idx, frame) in anim.frames.iter().enumerate() {
        if frame.include_all {
            for el in model.elements.values() {
                element_frame_map.entry(el.id.clone()).or_insert(frame_idx);
            }
        } else {
            for inc in &frame.includes {
                element_frame_map.entry(inc.clone()).or_insert(frame_idx);
            }
        }
    }

    // Also build a map for relationship edges: "from -> to" => frame_idx
    let mut rel_frame_map: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (frame_idx, frame) in anim.frames.iter().enumerate() {
        for inc in &frame.includes {
            if inc.contains(" -> ") {
                rel_frame_map.entry(inc.clone()).or_insert(frame_idx);
            }
        }
        // For include_all frames, map all model relationships
        if frame.include_all {
            for rel in &model.relationships {
                let key = format!("{} -> {}", rel.frm, rel.to);
                rel_frame_map.entry(key).or_insert(frame_idx);
            }
        }
    }

    // Process the SVG — wrap both data-id elements and data-from/data-to relationships
    let mut output = String::with_capacity(result.len() + 2000);
    let mut pos = 0;
    while pos < result.len() {
        // Try to find the next data-id or data-from attribute
        let next_element = result[pos..].find("data-id=\"").map(|i| (pos + i, true));
        let next_rel = result[pos..].find("data-from=\"").map(|i| (pos + i, false));

        // Pick whichever comes first
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
            output.push_str(&result[pos..]);
            break;
        };

        if is_element {
            // Element: data-id="..."
            let id_start = abs_idx + 9;
            if let Some(id_end) = result[id_start..].find('"') {
                let element_id = &result[id_start..id_start + id_end];

                if let Some(&frame_idx) = element_frame_map.get(element_id) {
                    let g_start = result[..abs_idx].rfind("<g ").unwrap_or(abs_idx);
                    if let Some(g_end) = find_closing_g(&result, abs_idx) {
                        output.push_str(&result[pos..g_start]);

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
                        let element_g = &result[g_start..g_end];
                        let patched =
                            element_g.replacen("class=\"", &format!("class=\"{} ", extra_cls), 1);
                        output.push_str(&patched);
                        output.push_str("</g>");

                        pos = g_end;
                        continue;
                    }
                }
            }
            // Didn't match — skip past this attribute to avoid infinite loop
            output.push_str(&result[pos..abs_idx + 9]);
            pos = abs_idx + 9;
        } else {
            // Relationship: data-from="X" data-to="Y"
            let from_start = abs_idx + 11; // after data-from="
            if let Some(from_end) = result[from_start..].find('"') {
                let from_id = result[from_start..from_start + from_end].to_string();
                // Find data-to="..."
                let search_area = &result[from_start + from_end..];
                if let Some(to_attr) = search_area.find("data-to=\"") {
                    let to_start = from_start + from_end + to_attr + 9;
                    if let Some(to_end) = result[to_start..].find('"') {
                        let to_id = result[to_start..to_start + to_end].to_string();
                        let rel_key = format!("{} -> {}", from_id, to_id);

                        if let Some(&frame_idx) = rel_frame_map.get(&rel_key) {
                            let g_start = result[..abs_idx].rfind("<g ").unwrap_or(abs_idx);
                            if let Some(g_end) = find_closing_g(&result, abs_idx) {
                                output.push_str(&result[pos..g_start]);

                                // Check for highlight on this relationship
                                let frame = &anim.frames[frame_idx];
                                let mut extra_cls = String::from("forge-enter");
                                let mut style_attr = String::new();
                                for hl in &frame.highlights {
                                    if hl.target.contains(&from_id) && hl.target.contains(&to_id) {
                                        extra_cls.push_str(" forge-highlight");
                                        if let Some(ref color) = hl.color {
                                            style_attr =
                                                format!(" style=\"--forge-hl-color: {}\"", color);
                                        }
                                    }
                                }

                                output.push_str(&format!(
                                    "<g class=\"forge-frame\" data-frame=\"{}\"{}>",
                                    frame_idx, style_attr,
                                ));
                                let rel_g = &result[g_start..g_end];
                                let patched = rel_g.replacen(
                                    "class=\"",
                                    &format!("class=\"{} ", extra_cls),
                                    1,
                                );
                                output.push_str(&patched);
                                output.push_str("</g>");

                                pos = g_end;
                                continue;
                            }
                        }
                    }
                }
            }
            // Didn't match — skip past
            output.push_str(&result[pos..abs_idx + 11]);
            pos = abs_idx + 11;
        }
    }

    // 5. Add frame navigation dots before </svg>
    if let Some(svg_end) = output.rfind("</svg>") {
        let mut nav = String::new();
        let dots_y = extract_svg_height(&output) - 20.0;
        let dots_start_x = extract_svg_width(&output) / 2.0 - (frame_count as f64 * 15.0) / 2.0;

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
        // Frame label
        let label_x = frame_count as f64 * 18.0 / 2.0;
        nav.push_str(&format!(
            "<text class=\"forge-frame-label\" x=\"{:.0}\" y=\"18\">{}</text>",
            label_x,
            esc(&anim.frames[0].label)
        ));
        nav.push_str("</g>\n");

        output.insert_str(svg_end, &nav);
    }

    // 6. Increase SVG height to accommodate navigation dots
    output = adjust_svg_height(&output, 40.0);

    output
}

/// Get the playback JavaScript for embedding in HTML pages.
pub fn playback_script() -> &'static str {
    PLAYBACK_JS
}

// ─── Helpers ─────────────────────────────────────────────────────

fn find_closing_g(svg: &str, start: usize) -> Option<usize> {
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

fn extract_svg_height(svg: &str) -> f64 {
    if let Some(idx) = svg.find("height=\"") {
        let start = idx + 8;
        if let Some(end) = svg[start..].find('"') {
            return svg[start..start + end].parse().unwrap_or(400.0);
        }
    }
    400.0
}

fn extract_svg_width(svg: &str) -> f64 {
    if let Some(idx) = svg.find("width=\"") {
        let start = idx + 7;
        if let Some(end) = svg[start..].find('"') {
            return svg[start..start + end].parse().unwrap_or(800.0);
        }
    }
    800.0
}

fn adjust_svg_height(svg: &str, extra: f64) -> String {
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

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_dynamic_animation_generates_cumulative_frames() {
        let mut model = Model::default();
        let mut sys = Element::new("sys", ElementKind::System, "S");
        sys.children = vec!["sys.a".into(), "sys.b".into(), "sys.c".into()];
        model.add_element(sys);
        for id in ["a", "b", "c"] {
            let mut el = Element::new(format!("sys.{id}"), ElementKind::Container, id);
            el.parent = Some("sys".into());
            model.add_element(el);
        }
        model.add_relationship(Relationship {
            frm: "sys.a".into(),
            to: "sys.b".into(),
            label: "1st".into(),
            technology: None,
            order: Some(1),
        });
        model.add_relationship(Relationship {
            frm: "sys.b".into(),
            to: "sys.c".into(),
            label: "2nd".into(),
            technology: None,
            order: Some(2),
        });
        // Unordered relationship — should be ignored by the derivation.
        model.add_relationship(Relationship {
            frm: "sys.a".into(),
            to: "sys.c".into(),
            label: "sidechannel".into(),
            technology: None,
            order: None,
        });

        let view = View {
            kind: ViewKind::Dynamic,
            key: "Flow".into(),
            scope: Some("sys".into()),
            title: None,
            auto_layout: AutoLayout::TopBottom,
            include_all: false,
            animation: Animation::default(),
            composite: None,
        };

        let anim = derive_dynamic_animation(&view, &model).expect("should derive");
        assert_eq!(anim.frames.len(), 2);

        // Frame 1: sys.a + sys.b + the a->b edge
        let f1 = &anim.frames[0];
        assert_eq!(f1.label, "Step 1");
        assert!(f1.includes.contains(&"sys.a".to_string()));
        assert!(f1.includes.contains(&"sys.b".to_string()));
        assert!(f1.includes.contains(&"sys.a -> sys.b".to_string()));
        assert!(!f1.includes.contains(&"sys.c".to_string()));

        // Frame 2: cumulative — everything from frame 1 plus c and b->c
        let f2 = &anim.frames[1];
        assert_eq!(f2.label, "Step 2");
        assert!(f2.includes.contains(&"sys.a".to_string()));
        assert!(f2.includes.contains(&"sys.c".to_string()));
        assert!(f2.includes.contains(&"sys.b -> sys.c".to_string()));
    }

    #[test]
    fn derive_dynamic_animation_ignores_non_dynamic_views() {
        let model = Model::default();
        let view = View {
            kind: ViewKind::Container,
            key: "K".into(),
            scope: None,
            title: None,
            auto_layout: AutoLayout::TopBottom,
            include_all: false,
            animation: Animation::default(),
            composite: None,
        };
        assert!(derive_dynamic_animation(&view, &model).is_none());
    }

    #[test]
    fn derive_dynamic_animation_respects_explicit_animation() {
        let mut model = Model::default();
        let mut sys = Element::new("sys", ElementKind::System, "S");
        sys.children = vec!["sys.a".into()];
        model.add_element(sys);
        let mut a = Element::new("sys.a", ElementKind::Container, "a");
        a.parent = Some("sys".into());
        model.add_element(a);
        model.add_relationship(Relationship {
            frm: "sys.a".into(),
            to: "sys.a".into(),
            label: "self".into(),
            technology: None,
            order: Some(1),
        });

        // User-supplied animation pre-empts the auto-generated one.
        let mut explicit = Animation::default();
        explicit.frames.push(AnimationFrame {
            label: "Handcrafted".into(),
            includes: vec!["sys.a".into()],
            include_all: false,
            highlights: Vec::new(),
            states: Vec::new(),
            notes: None,
        });
        let view = View {
            kind: ViewKind::Dynamic,
            key: "F".into(),
            scope: Some("sys".into()),
            title: None,
            auto_layout: AutoLayout::TopBottom,
            include_all: false,
            animation: explicit,
            composite: None,
        };
        assert!(derive_dynamic_animation(&view, &model).is_none());
    }

    #[test]
    fn empty_animation_passes_through() {
        let svg = "<svg>test</svg>";
        let view = View {
            kind: ViewKind::Container,
            key: "test".into(),
            scope: None,
            title: None,
            auto_layout: AutoLayout::TopBottom,
            include_all: true,
            animation: Animation::default(),
            composite: None,
        };
        let model = Model::default();
        let result = animate_svg(svg, &view, &model);
        assert_eq!(result, svg);
    }

    #[test]
    fn animated_svg_has_frame_classes() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 400" width="800" height="400" class="forge-diagram"><defs><style></style></defs><g class="forge-elements"><g class="forge-element" data-id="svc">content</g></g></svg>"#;

        let mut anim = Animation::default();
        anim.frames.push(AnimationFrame {
            label: "Step 1".into(),
            includes: vec!["svc".into()],
            include_all: false,
            highlights: Vec::new(),
            states: Vec::new(),
            notes: None,
        });

        let view = View {
            kind: ViewKind::Container,
            key: "test".into(),
            scope: None,
            title: None,
            auto_layout: AutoLayout::TopBottom,
            include_all: true,
            animation: anim,
            composite: None,
        };
        let mut model = Model::default();
        model.add_element(Element::new("svc", ElementKind::Container, "Service"));

        let result = animate_svg(svg, &view, &model);
        assert!(result.contains("forge-animated"));
        assert!(result.contains("data-frames=\"1\""));
        assert!(result.contains("forge-frame"));
        assert!(result.contains("forge-frame-dot"));
        assert!(result.contains("forge-enter"));
    }

    #[test]
    fn animation_css_injected() {
        let svg = r#"<svg class="forge-diagram"><defs><style></style></defs></svg>"#;
        let mut anim = Animation::default();
        anim.frames.push(AnimationFrame {
            label: "F1".into(),
            includes: Vec::new(),
            include_all: true,
            highlights: Vec::new(),
            states: Vec::new(),
            notes: None,
        });
        let view = View {
            kind: ViewKind::Container,
            key: "t".into(),
            scope: None,
            title: None,
            auto_layout: AutoLayout::TopBottom,
            include_all: true,
            animation: anim,
            composite: None,
        };
        let result = animate_svg(svg, &view, &Model::default());
        assert!(result.contains("forge-pulse"));
        assert!(result.contains("forge-fade-in"));
    }
}

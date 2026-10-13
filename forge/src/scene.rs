//! A laid-out view as a [`diagram_scene::Scene`]: the same reading of a
//! [`Layout`] the SVG renderer makes (kinds and tags to shapes, deployment
//! nodes and trust zones to containers, entity tables with crow's feet,
//! branch lanes, step badges, data-class badges, the legend), with colours
//! as theme roles so a host draws it in its own theme.

use diagram_scene::{
    route, Anchor, Badge, Cardinality, Container, ContainerKind, Dash, Edge, EdgeLabel, Font,
    Legend, LegendEntry, Marker, Node, Paint, Pen, Point, Rect, Role, Scene, Seg, Shape, Style,
    Text, TextLine, Weight,
};

use crate::layout::{Layout, LayoutEdge, LayoutNode};
use crate::model::ElementKind;

/// Body text size, as the SVG renderer's element names.
const FONT_SIZE: f32 = 14.0;
/// Branch lanes: commit spacing and dot radius, as the renderer draws them.
const COMMIT_SPACING: f32 = 60.0;
const COMMIT_RADIUS: f32 = 8.0;

/// `layout` as a scene; `measure` wraps descriptions to their boxes.
pub fn from_layout(layout: &Layout, measure: &dyn diagram_scene::Measure) -> Scene {
    let mut scene = Scene::new(layout.width as f32, layout.height as f32);
    scene.font_size = FONT_SIZE;
    if let Some(title) = &layout.title {
        scene.title = Some(Text {
            pos: Point::new(scene.width / 2.0, 30.0),
            anchor: Anchor::Middle,
            line: TextLine::new(title.clone(), Font::Title),
            paint: Paint::Role(Role::Text),
            backed: false,
        });
    }

    // Containers (deployment nodes, trust zones), outermost first.
    let mut boxes: Vec<&LayoutNode> = layout
        .nodes
        .iter()
        .filter(|n| n.kind == ElementKind::DeploymentNode)
        .collect();
    boxes.sort_by_key(|n| n.depth);
    for n in boxes {
        scene.containers.push(container(n));
    }

    let mut nodes = Vec::new();
    for n in &layout.nodes {
        match n.kind {
            ElementKind::DeploymentNode => {}
            ElementKind::Branch => branch(&mut scene, n),
            ElementKind::Gate => {
                // The diamond, its name underneath (split at hyphens, as
                // the renderer does, so it's no wider than it needs).
                let mut gate = node(n, measure);
                gate.label.clear();
                let r = gate.rect;
                let words: Vec<&str> = n.label.split('-').collect();
                let lines = if n.label.len() <= 14 {
                    vec![n.label.clone()]
                } else {
                    let mid = words.len() / 2;
                    vec![words[..mid].join("-") + "-", words[mid..].join("-")]
                };
                for (i, line) in lines.into_iter().enumerate() {
                    scene.texts.push(Text {
                        pos: Point::new(r.center().x, r.bottom() + 14.0 + i as f32 * 14.0),
                        anchor: Anchor::Middle,
                        line: TextLine::new(line, Font::Small),
                        paint: Paint::Role(Role::Muted),
                        backed: true,
                    });
                }
                nodes.push(gate);
            }
            _ => nodes.push(node(n, measure)),
        }
    }

    let mut placed_labels: Vec<Rect> = Vec::new();
    for e in &layout.edges {
        if let Some(edge) = edge(e, layout, &nodes, &mut placed_labels, measure) {
            scene.edges.extend(edge);
        }
    }
    scene.nodes.extend(nodes);

    // The legend under the diagram, so it never sits on an element.
    if let Some(mut legend) = legend(layout) {
        legend.rect.y = scene.height + 4.0;
        scene.height += legend.rect.height + 14.0;
        scene.legend = Some(legend);
    }
    scene
}

fn rect(n: &LayoutNode) -> Rect {
    Rect::new(
        n.rect.x as f32,
        n.rect.y as f32,
        n.rect.w as f32,
        n.rect.h as f32,
    )
}

fn has_tag(n: &LayoutNode, tag: &str) -> bool {
    n.tags.iter().any(|t| t == tag)
}

/// The label under an element's name: `[Software System]` and so on.
fn kind_label(kind: ElementKind) -> Option<&'static str> {
    Some(match kind {
        ElementKind::Person => "Person",
        ElementKind::System => "Software System",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
        ElementKind::Pipeline => "Pipeline",
        ElementKind::Repository => "Repository",
        _ => return None,
    })
}

/// The shape and fill role of an element.
fn look(n: &LayoutNode) -> (Shape, Role) {
    if has_tag(n, "data-entity") {
        return (Shape::Table, Role::Surface);
    }
    if has_tag(n, "database") {
        return (Shape::Cylinder, Role::Database);
    }
    if has_tag(n, "external") {
        return (Shape::Rounded, Role::External);
    }
    match n.kind {
        ElementKind::Person => (Shape::Person, Role::Person),
        ElementKind::System => (Shape::Rounded, Role::System),
        ElementKind::Container => (Shape::Rounded, Role::Container),
        ElementKind::Component => (Shape::Rounded, Role::Component),
        ElementKind::Gate => (Shape::Diamond, Role::Highlight),
        ElementKind::Stage | ElementKind::Step => (Shape::Rounded, Role::Surface),
        ElementKind::Artifact => (Shape::Document, Role::Surface),
        ElementKind::Repository => (Shape::Cylinder, Role::Surface),
        _ => (Shape::Rounded, Role::Surface),
    }
}

fn node(n: &LayoutNode, measure: &dyn diagram_scene::Measure) -> Node {
    let (shape, role) = look(n);
    let mut node = Node::new(n.id.clone(), shape, rect(n));
    node.style.fill = Paint::Role(role);
    if shape == Shape::Table {
        // An entity: its name over a row per field (`name: type`).
        node.label = vec![TextLine::new(n.label.clone(), Font::Bold)];
        if let Some(owner) = &n.sublabel {
            node.stereotype = vec![TextLine::new(owner.clone(), Font::Small)];
        }
        let fields = n
            .description
            .as_deref()
            .unwrap_or("")
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|field| {
                let (name, rest) = field.split_once(':').unwrap_or((field, ""));
                let mut line = TextLine::new(name.trim().to_string(), Font::Bold);
                if !rest.trim().is_empty() {
                    line.runs.push(diagram_scene::Run {
                        text: format!("  {}", rest.trim()),
                        font: Font::Small,
                    });
                }
                line
            })
            .collect();
        node.compartments = vec![fields];
    } else if has_tag(n, "api-endpoint") {
        // One line: the method and path, then what it does.
        let mut line = TextLine::new(n.label.clone(), Font::Mono);
        if let Some(desc) = &n.sublabel {
            line.runs.push(diagram_scene::Run {
                text: format!("  {desc}"),
                font: Font::Small,
            });
        }
        node.label = vec![line];
    } else {
        node.label = vec![TextLine::new(n.label.clone(), Font::Bold)];
        let small = |text: String| TextLine::new(text, Font::Small);
        // Kinds the view stands for; a tech entry isn't really a container.
        if let Some(kind) = kind_label(n.kind).filter(|_| !has_tag(n, "tech-entry")) {
            node.sublabel.push(small(format!("[{kind}]")));
        }
        if let Some(desc) = &n.description {
            let width = (node.rect.width - 24.0).max(40.0);
            let size = Font::Small.size(FONT_SIZE);
            node.sublabel.extend(
                wrap(measure, desc, size, width)
                    .into_iter()
                    .take(3)
                    .map(small),
            );
        }
        // The technology, unless it only repeats the description.
        if let Some(sub) = n
            .sublabel
            .as_ref()
            .filter(|sub| n.description.as_deref() != Some(sub.trim_matches(['[', ']'])))
        {
            let tech = if sub.starts_with('[') {
                sub.clone()
            } else {
                format!("[{sub}]")
            };
            node.sublabel.push(small(tech));
        }
        // What fits: lines go from the end (technology, then description)
        // until the text fits the box the layout made, as the renderer
        // drops them.
        let fits = |node: &Node| {
            diagram_scene::shape::natural_size(node, FONT_SIZE, measure).1 <= node.rect.height + 6.0
        };
        while !node.sublabel.is_empty() && !fits(&node) {
            node.sublabel.pop();
        }
    }
    if shape == Shape::Table {
        // Wide enough for its rows in the font it's drawn in.
        let (w, h) = diagram_scene::shape::natural_size(&node, FONT_SIZE, measure);
        node.rect.width = node.rect.width.max(w);
        node.rect.height = node.rect.height.max(h);
    }
    node.badges = n
        .data_classes
        .iter()
        .map(|class| {
            let ((r, g, b), letter) = data_class(class);
            Badge {
                text: letter.to_string(),
                paint: Paint::Rgb(r, g, b),
            }
        })
        .collect();
    node
}

/// Data classes: their badge colour and letter, as the renderer has them.
fn data_class(class: &str) -> ((u8, u8, u8), &'static str) {
    match class.to_ascii_lowercase().as_str() {
        "pii" => ((0x8b, 0x5c, 0xf6), "P"),
        "financial" => ((0xd9, 0x77, 0x06), "F"),
        "public" => ((0x16, 0xa3, 0x4a), "P"),
        "secret" => ((0xdc, 0x26, 0x26), "S"),
        "internal" => ((0x6b, 0x72, 0x80), "I"),
        _ => ((0x6b, 0x72, 0x80), "?"),
    }
}

fn container(n: &LayoutNode) -> Container {
    let kind = if has_tag(n, "trust-zone") {
        let level = n
            .tags
            .iter()
            .find_map(|t| t.strip_prefix("trust-"))
            .map_or(0, |level| match level {
                "public" => 0,
                "dmz" => 1,
                "internal" => 2,
                "pci" => 3,
                "restricted" => 4,
                _ => 2,
            });
        ContainerKind::TrustZone(level)
    } else {
        ContainerKind::DeploymentNode
    };
    // The name, and beside it (in the header the layout leaves) any
    // subtitle: the technology, a team's contact.
    let mut line = TextLine::new(n.label.clone(), Font::Bold);
    if let Some(sub) = &n.sublabel {
        line.runs.push(diagram_scene::Run {
            text: format!("  {sub}"),
            font: Font::Small,
        });
    }
    let label = vec![line];
    Container {
        rect: rect(n),
        style: Style::for_container(&kind),
        kind,
        label,
    }
}

fn edge(
    e: &LayoutEdge,
    layout: &Layout,
    nodes: &[Node],
    placed_labels: &mut Vec<Rect>,
    measure: &dyn diagram_scene::Measure,
) -> Option<Vec<Edge>> {
    let from = layout.nodes.iter().find(|n| n.id == e.frm)?;
    let to = layout.nodes.iter().find(|n| n.id == e.to)?;
    if from.kind == ElementKind::Branch && to.kind == ElementKind::Branch {
        return Some(vec![branch_edge(from, to)]);
    }
    let entities = has_tag(from, "data-entity") && has_tag(to, "data-entity");
    let pipe = from.kind == ElementKind::Stage || to.kind == ElementKind::Stage;
    let (fr, tr) = (rect(from), rect(to));
    let (mut label_text, mut start, mut end) = (e.label.clone(), Marker::None, Marker::Arrow);
    let path = if entities {
        // Up from both tops to a shared line above them.
        let lift = fr.y.min(tr.y) - 25.0;
        let (a, b) = (fr.center().x, tr.center().x);
        if let Some((text, (s, f))) = cardinality(&e.label) {
            label_text = text;
            (start, end) = (s, f);
        }
        route::rounded_polyline(
            &[
                Point::new(a, fr.y),
                Point::new(a, lift),
                Point::new(b, lift),
                Point::new(b, tr.y),
            ],
            8.0,
        )
    } else {
        let others: Vec<Rect> = nodes
            .iter()
            .filter(|n| n.id != from.id && n.id != to.id)
            .map(|n| n.rect)
            .collect();
        avoiding(fr, tr, &others)
    };
    let mut edge = Edge::new(path).between(&from.id, &to.id);
    edge.start = start;
    edge.end = end;
    if pipe {
        edge.pen = Pen {
            dash: Dash::Dashed,
            weight: Weight::Thick,
            paint: Paint::Role(Role::Muted),
        };
    }
    let mut mid = route::label_position(&edge.path, 0.0)?;
    let mut lines = Vec::new();
    if !label_text.is_empty() {
        lines.push(TextLine::new(label_text, Font::Small));
    }
    if let Some(tech) = &e.technology {
        lines.push(TextLine::new(format!("[{tech}]"), Font::Small));
    }
    if !lines.is_empty() {
        let mut label = EdgeLabel::new(mid, lines, true, measure, FONT_SIZE);
        // The first spot along the edge clear of elements and other labels.
        let line_h = diagram_scene::shape::line_height(Font::Small.size(FONT_SIZE));
        let size = (label.width + 14.0, label.lines.len() as f32 * line_h + 6.0);
        let clear = |at: Point| {
            let r = Rect::centered(at, size.0, size.1);
            !nodes.iter().any(|n| n.rect.intersects(&r))
                && !placed_labels.iter().any(|p| p.intersects(&r))
        };
        if let Some(at) = [0.5, 0.38, 0.62, 0.28, 0.72, 0.2, 0.8]
            .into_iter()
            .filter_map(|t| diagram_scene::flatten::point_at(&edge.path, t))
            .find(|&at| clear(at))
        {
            label.pos = at;
            mid = at;
        }
        placed_labels.push(Rect::centered(label.pos, size.0, size.1));
        edge.labels.push(label);
    }
    if let Some(step) = e.order {
        // Beside the label, off the line.
        let at = route::label_position(&edge.path, 18.0).unwrap_or(mid);
        let at = if edge.labels.is_empty() {
            at
        } else {
            let w = edge.labels[0].text_width(FONT_SIZE) / 2.0 + 18.0;
            Point::new(mid.x - w, mid.y)
        };
        edge.step = Some((step, at));
    }
    Some(vec![edge])
}

/// A curve from `from` to `to` that keeps clear of `others` if it can:
/// between the sides facing along the main axis, else the other axis,
/// else arcing over or under.
fn avoiding(from: Rect, to: Rect, others: &[Rect]) -> Vec<Seg> {
    use diagram_scene::route::{port, Side};
    let (a, b) = (from.center(), to.center());
    let horizontal = (b.x - a.x).abs() > (b.y - a.y).abs();
    let crosses = |path: &[Seg]| {
        diagram_scene::flatten::flatten(path, 1.0)
            .iter()
            .flat_map(|p| p.points.iter())
            .any(|&p| others.iter().any(|r| r.inflate(4.0).contains(p)))
    };
    let arc = |side: f32, lift: f32| {
        let s = if horizontal {
            if side < 0.0 {
                Side::Top
            } else {
                Side::Bottom
            }
        } else if side < 0.0 {
            Side::Left
        } else {
            Side::Right
        };
        let (p, q) = (port(from, s), port(to, s));
        let (dx, dy) = if horizontal {
            (0.0, side * lift)
        } else {
            (side * lift, 0.0)
        };
        vec![
            Seg::Move(p),
            Seg::Cubic(p.offset(dx, dy), q.offset(dx, dy), q),
        ]
    };
    let candidates = [
        route::spline(from, to, horizontal),
        route::spline(from, to, !horizontal),
        arc(-1.0, 70.0),
        arc(1.0, 70.0),
        arc(-1.0, 130.0),
        arc(1.0, 130.0),
    ];
    candidates
        .iter()
        .find(|path| !crosses(path))
        .unwrap_or(&candidates[0])
        .clone()
}

/// `owns [1:N]` → the label without its cardinality, and the markers at
/// each end.
fn cardinality(label: &str) -> Option<(String, (Marker, Marker))> {
    let open = label.rfind('[')?;
    let close = label[open..].find(']')? + open;
    let (left, right) = label[open + 1..close].split_once(':')?;
    let marker = |side: &str| {
        Marker::CrowFoot(match side.trim() {
            "1" => Cardinality::One,
            "0..1" | "?" => Cardinality::ZeroOrOne,
            "0..N" | "0..*" | "*" => Cardinality::ZeroOrMany,
            "1..N" | "1..*" | "+" => Cardinality::OneOrMany,
            _ => Cardinality::Many,
        })
    };
    let text = format!("{}{}", &label[..open], &label[close + 1..])
        .trim()
        .to_string();
    Some((text, (marker(left), marker(right))))
}

/// The `commits=…;lane_y=…;timeline_x=…;is_trunk=…` a branch carries.
struct Lane {
    commits: usize,
    y: f32,
    x: f32,
    trunk: bool,
    color: Paint,
}

fn lane(n: &LayoutNode) -> Lane {
    let mut lane = Lane {
        commits: 5,
        y: 0.0,
        x: 0.0,
        trunk: false,
        color: Paint::Role(Role::Series(0)),
    };
    for part in n.sublabel.as_deref().unwrap_or("").split(';') {
        if let Some(v) = part.strip_prefix("commits=") {
            lane.commits = v.parse().unwrap_or(5);
        } else if let Some(v) = part.strip_prefix("lane_y=") {
            lane.y = v.parse().unwrap_or(0.0);
        } else if let Some(v) = part.strip_prefix("timeline_x=") {
            lane.x = v.parse().unwrap_or(0.0);
        } else if let Some(v) = part.strip_prefix("is_trunk=") {
            lane.trunk = v == "true";
        }
    }
    let index = n
        .tags
        .iter()
        .find_map(|t| t.strip_prefix("gitgraph-lane-"))
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(0);
    lane.color = Paint::Role(Role::Series(index));
    lane
}

/// A branch: its name, its line and its commits.
fn branch(scene: &mut Scene, n: &LayoutNode) {
    let lane = lane(n);
    let start = if lane.trunk {
        lane.x
    } else {
        lane.x + COMMIT_SPACING
    };
    let end = start + (lane.commits.max(1) as f32 - 1.0) * COMMIT_SPACING;
    scene.texts.push(Text {
        pos: Point::new(lane.x - 16.0, lane.y + 4.5),
        anchor: Anchor::End,
        line: TextLine::new(n.label.clone(), Font::Bold),
        paint: lane.color,
        backed: false,
    });
    scene.edges.push(Edge {
        pen: Pen {
            dash: Dash::Solid,
            weight: Weight::Heavy,
            paint: lane.color,
        },
        end: Marker::None,
        ..Edge::new(vec![
            Seg::Move(Point::new(start, lane.y)),
            Seg::Line(Point::new(end, lane.y)),
        ])
    });
    for i in 0..lane.commits {
        let at = Point::new(start + i as f32 * COMMIT_SPACING, lane.y);
        let merge = !lane.trunk && i + 1 == lane.commits;
        let r = if merge {
            COMMIT_RADIUS + 3.0
        } else {
            COMMIT_RADIUS
        };
        let mut dot = Node::new(
            format!("{}:{i}", n.id),
            if merge { Shape::End } else { Shape::Circle },
            Rect::centered(at, r * 2.0, r * 2.0),
        );
        dot.style.fill = lane.color;
        dot.style.stroke = lane.color;
        scene.nodes.push(dot);
    }
}

/// A fork from the trunk or a merge back into it.
fn branch_edge(from: &LayoutNode, to: &LayoutNode) -> Edge {
    let (a, b) = (lane(from), lane(to));
    let color = if a.trunk { b.color } else { a.color };
    let (p, q) = if a.trunk {
        (
            Point::new(a.x + COMMIT_SPACING, a.y),
            Point::new(b.x + COMMIT_SPACING, b.y),
        )
    } else {
        (
            Point::new(
                a.x + COMMIT_SPACING + (a.commits as f32 - 1.0) * COMMIT_SPACING,
                a.y,
            ),
            Point::new(b.x + (b.commits as f32 - 2.0) * COMMIT_SPACING, b.y),
        )
    };
    let dy = (q.y - p.y) * 0.5;
    Edge {
        from: Some(from.id.clone()),
        to: Some(to.id.clone()),
        pen: Pen {
            dash: Dash::Solid,
            weight: Weight::Heavy,
            paint: color,
        },
        end: Marker::None,
        ..Edge::new(vec![
            Seg::Move(p),
            Seg::Cubic(p.offset(0.0, dy), q.offset(0.0, -dy), q),
        ])
    }
}

/// A key to the kinds of element shown, in the bottom-right corner.
fn legend(layout: &Layout) -> Option<Legend> {
    let mut entries: Vec<LegendEntry> = Vec::new();
    for n in &layout.nodes {
        if matches!(n.kind, ElementKind::Branch | ElementKind::DeploymentNode)
            || has_tag(n, "data-entity")
        {
            continue;
        }
        let (shape, role) = look(n);
        let label = if shape == Shape::Cylinder && role == Role::Database {
            "Database".to_string()
        } else if role == Role::External {
            "External".to_string()
        } else {
            kind_label(n.kind)
                .map(str::to_string)
                .unwrap_or_else(|| format!("{:?}", n.kind))
        };
        if entries.iter().any(|e| e.label == label) {
            continue;
        }
        let mut style = Style::for_shape(shape);
        style.fill = Paint::Role(role);
        entries.push(LegendEntry {
            shape: if shape == Shape::Person {
                Shape::Rounded
            } else {
                shape
            },
            style,
            label,
        });
    }
    if entries.len() < 2 {
        return None;
    }
    let row = FONT_SIZE * 1.6;
    let (w, h) = (170.0, 16.0 + row * entries.len() as f32);
    Some(Legend {
        rect: Rect::new(
            layout.width as f32 - w - 20.0,
            layout.height as f32 - h - 10.0,
            w,
            h,
        ),
        entries,
    })
}

/// `text` in lines no wider than `width`, breaking at spaces.
fn wrap(measure: &dyn diagram_scene::Measure, text: &str, size: f32, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && measure.width(&candidate, size, false) > width {
            lines.push(std::mem::replace(&mut current, word.to_string()));
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAYMENTS: &str = include_str!("../examples/payments.forge");

    fn model() -> crate::model::Model {
        crate::parse(PAYMENTS, std::path::Path::new("examples")).expect("the example parses")
    }

    #[test]
    fn every_view_becomes_a_scene_with_everything_placed() {
        let model = model();
        let views = crate::views(&model);
        assert!(views.len() >= 8, "{views:?}");
        for view in &views {
            let scene = crate::scene(&model, &view.key, &diagram_scene::Estimate).unwrap();
            let bounds = scene.bounds().inflate(1.0);
            for n in &scene.nodes {
                assert!(
                    bounds.encloses(&n.rect, 30.0),
                    "{}: {} at {:?}",
                    view.key,
                    n.id,
                    n.rect
                );
            }
            assert!(
                !scene.nodes.is_empty() || !scene.edges.is_empty(),
                "{} draws nothing",
                view.key
            );
            // Colours are the theme's, but for data classes' own badges.
            for n in &scene.nodes {
                assert!(!matches!(n.style.fill, Paint::Rgb(..)), "{}", n.id);
            }
        }
    }

    #[test]
    fn views_come_out_the_same_every_time() {
        // Each parse gets maps with their own random order; the layout
        // mustn't follow it, or a preview reshuffles on every edit.
        let (a, b) = (model(), model());
        for view in crate::views(&a) {
            let one = crate::scene(&a, &view.key, &diagram_scene::Estimate).unwrap();
            let two = crate::scene(&b, &view.key, &diagram_scene::Estimate).unwrap();
            assert_eq!(one, two, "{}", view.key);
        }
    }

    #[test]
    fn elements_take_their_shapes() {
        let model = model();
        let shapes: Vec<Shape> = crate::views(&model)
            .iter()
            .filter_map(|v| crate::scene(&model, &v.key, &diagram_scene::Estimate))
            .flat_map(|s| s.nodes.into_iter().map(|n| n.shape))
            .collect();
        for shape in [Shape::Person, Shape::Rounded, Shape::Cylinder, Shape::Table] {
            assert!(shapes.contains(&shape), "no {shape:?} in {shapes:?}");
        }
    }

    #[test]
    fn data_models_get_crows_feet() {
        assert_eq!(
            cardinality("owns [1:N]"),
            Some((
                "owns".to_string(),
                (
                    Marker::CrowFoot(Cardinality::One),
                    Marker::CrowFoot(Cardinality::Many)
                )
            ))
        );
        assert_eq!(cardinality("plain"), None);
    }
}

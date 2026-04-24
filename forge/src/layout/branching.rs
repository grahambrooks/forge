use super::*;

const BRANCH_LANE_GAP: f64 = 40.0; // vertical spacing between branch lanes
const BRANCH_LABEL_W: f64 = 120.0; // space reserved for branch name labels
const COMMIT_SPACING: f64 = 60.0; // horizontal distance between commit dots
const COMMIT_R: f64 = 8.0; // commit dot radius

pub(super) fn layout_branching(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let strategy_id = view.scope.as_deref().unwrap_or("");

    let branches: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| {
            e.kind == ElementKind::Branch
                && e.properties.get("strategy").map(|s| s.as_str()) == Some(strategy_id)
        })
        .collect();

    // Sort: trunk first, then others
    let mut sorted = branches.clone();
    sorted.sort_by_key(|b| {
        if b.properties.contains_key("protection") {
            0
        } else {
            1
        }
    });

    // Assign each branch a lane index (row) and a color index
    let num_branches = sorted.len();
    // Commits per branch: trunk gets more (it's the long-lived line),
    // feature branches get fewer (short-lived)
    let trunk_commits = 7usize;
    let feature_commits = 3usize;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let timeline_x = PAD + BRANCH_LABEL_W;
    let top_y = TITLE_H + 20.0;

    for (lane, branch) in sorted.iter().enumerate() {
        let is_trunk = branch.properties.contains_key("protection");
        let num_commits = if is_trunk {
            trunk_commits
        } else {
            feature_commits
        };
        let lane_y = top_y + lane as f64 * BRANCH_LANE_GAP;

        let mut tags = branch.tags.clone();
        if is_trunk {
            tags.push("trunk".into());
        }
        // Encode the lane index and color for the renderer
        tags.push(format!("gitgraph-lane-{}", lane));

        // Sublabel encodes commit count and lane_y for the renderer
        let sublabel = Some(format!(
            "commits={};lane_y={:.0};timeline_x={:.0};is_trunk={}",
            num_commits, lane_y, timeline_x, is_trunk
        ));

        // The node rect spans the full timeline width
        let timeline_w = (trunk_commits as f64) * COMMIT_SPACING;
        nodes.push(LayoutNode {
            id: branch.id.clone(),
            label: branch.name.clone(),
            sublabel,
            kind: ElementKind::Branch,
            tags,
            rect: Rect {
                x: PAD,
                y: lane_y - COMMIT_R,
                w: BRANCH_LABEL_W + timeline_w + PAD,
                h: COMMIT_R * 2.0,
            },
            description: branch.description.clone(),
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });
    }

    // Edges for branches-from / merges-into (renderer draws curves)
    for branch in &sorted {
        if let Some(from) = branch.properties.get("branches-from") {
            edges.push(LayoutEdge {
                frm: from.clone(),
                to: branch.id.clone(),
                label: String::new(),
                technology: None,
                order: None,
            });
        }
        if let Some(into) = branch.properties.get("merges-into") {
            edges.push(LayoutEdge {
                frm: branch.id.clone(),
                to: into.clone(),
                label: String::new(),
                technology: None,
                order: None,
            });
        }
    }

    let timeline_w = (trunk_commits as f64) * COMMIT_SPACING;
    let canvas_w = PAD + BRANCH_LABEL_W + timeline_w + PAD * 2.0;
    let canvas_h = top_y + (num_branches as f64) * BRANCH_LANE_GAP + PAD;
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Branching Strategy", model.name));

    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges,
    }
}

//! Gitgraph-style branch and merge rendering for branching views.

use super::util::esc;
use crate::layout::{LayoutEdge, LayoutNode};

pub(super) fn render_branch(o: &mut Vec<String>, n: &LayoutNode) {
    // Parse gitgraph layout parameters from sublabel
    let (num_commits, lane_y, timeline_x, is_trunk) = parse_gitgraph_params(&n.sublabel);

    // Determine lane color index from tags
    let lane_idx = n
        .tags
        .iter()
        .find_map(|t| t.strip_prefix("gitgraph-lane-"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let color_var = format!("var(--svg-git{})", lane_idx % 8);

    let commit_spacing = 60.0;
    let commit_r = 8.0;

    // Branch name label (left of timeline)
    o.push(format!(
        r##"      <text x="{:.0}" y="{:.1}" class="forge-label--name" text-anchor="end">{}</text>"##,
        timeline_x - 16.0,
        lane_y + 4.5,
        esc(&n.label)
    ));

    // Horizontal branch line
    let line_x1 = timeline_x;
    let line_x2 = if is_trunk {
        timeline_x + (num_commits as f64 - 1.0) * commit_spacing
    } else {
        // Feature branches: start offset and end before trunk end
        timeline_x + commit_spacing + (num_commits as f64 - 1.0) * commit_spacing
    };
    let actual_x1 = if is_trunk {
        line_x1
    } else {
        timeline_x + commit_spacing
    };

    o.push(format!(
        r##"      <line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" class="forge-gitgraph-line" stroke="{}" />"##,
        actual_x1, lane_y, line_x2, lane_y, color_var
    ));

    // Commit dots along the line
    let start_x = actual_x1;
    for i in 0..num_commits {
        let cx = start_x + i as f64 * commit_spacing;
        // Last commit on a non-trunk branch is a merge commit (double circle)
        let is_merge = !is_trunk && i == num_commits - 1;
        if is_merge {
            // Merge commit: filled circle with larger outer ring
            o.push(format!(
                r##"      <circle cx="{:.0}" cy="{:.0}" r="{:.0}" class="forge-gitgraph-merge" fill="{}" />"##,
                cx,
                lane_y,
                commit_r + 3.0,
                color_var
            ));
            o.push(format!(
                r##"      <circle cx="{:.0}" cy="{:.0}" r="{:.0}" fill="{}" />"##,
                cx, lane_y, commit_r, color_var
            ));
        } else {
            o.push(format!(
                r##"      <circle cx="{:.0}" cy="{:.0}" r="{:.0}" class="forge-gitgraph-commit" fill="{}" />"##,
                cx, lane_y, commit_r, color_var
            ));
        }
    }
}

fn parse_gitgraph_params(sublabel: &Option<String>) -> (usize, f64, f64, bool) {
    let s = sublabel.as_deref().unwrap_or("");
    let mut commits = 5usize;
    let mut lane_y = 0.0f64;
    let mut timeline_x = 0.0f64;
    let mut is_trunk = false;
    for part in s.split(';') {
        if let Some(v) = part.strip_prefix("commits=") {
            commits = v.parse().unwrap_or(5);
        } else if let Some(v) = part.strip_prefix("lane_y=") {
            lane_y = v.parse().unwrap_or(0.0);
        } else if let Some(v) = part.strip_prefix("timeline_x=") {
            timeline_x = v.parse().unwrap_or(0.0);
        } else if let Some(v) = part.strip_prefix("is_trunk=") {
            is_trunk = v == "true";
        }
    }
    (commits, lane_y, timeline_x, is_trunk)
}

pub(super) fn render_gitgraph_edge(
    o: &mut Vec<String>,
    _edge: &LayoutEdge,
    frm: &LayoutNode,
    to: &LayoutNode,
) {
    let (_, frm_y, frm_timeline_x, _) = parse_gitgraph_params(&frm.sublabel);
    let (_, to_y, to_timeline_x, _) = parse_gitgraph_params(&to.sublabel);

    // Determine which node is the trunk (parent) and which is the feature
    let (_, _, _, frm_trunk) = parse_gitgraph_params(&frm.sublabel);
    let (to_commits, _, _, _) = parse_gitgraph_params(&to.sublabel);

    let commit_spacing = 60.0;

    // Color: use the feature branch's lane color
    let feature_node = if frm_trunk { to } else { frm };
    let lane_idx = feature_node
        .tags
        .iter()
        .find_map(|t| t.strip_prefix("gitgraph-lane-"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let color_var = format!("var(--svg-git{})", lane_idx % 8);

    if frm_trunk {
        // branches-from: trunk → feature (fork point)
        // Curved path from a commit on trunk down to first commit on feature
        let fork_x = frm_timeline_x + commit_spacing; // 2nd commit on trunk
        let start_y = frm_y;
        let end_x = to_timeline_x + commit_spacing; // 1st commit on feature
        let end_y = to_y;
        let ctrl_dy = (end_y - start_y).abs() * 0.5;

        o.push(format!(
            r##"    <path d="M {:.0} {:.0} C {:.0} {:.0}, {:.0} {:.0}, {:.0} {:.0}" class="forge-gitgraph-branch-path" stroke="{}" />"##,
            fork_x,
            start_y,
            fork_x,
            start_y + ctrl_dy,
            end_x,
            end_y - ctrl_dy,
            end_x,
            end_y,
            color_var
        ));
    } else {
        // merges-into: feature → trunk (merge point)
        // Curved path from last commit on feature up to a commit on trunk
        let (frm_commits, _, _, _) = parse_gitgraph_params(&frm.sublabel);
        let merge_from_x =
            frm_timeline_x + commit_spacing + (frm_commits as f64 - 1.0) * commit_spacing;
        let merge_to_x = to_timeline_x + (to_commits as f64 - 2.0) * commit_spacing;
        let start_y = frm_y;
        let end_y = to_y;
        let ctrl_dy = (start_y - end_y).abs() * 0.5;

        o.push(format!(
            r##"    <path d="M {:.0} {:.0} C {:.0} {:.0}, {:.0} {:.0}, {:.0} {:.0}" class="forge-gitgraph-branch-path" stroke="{}" />"##,
            merge_from_x,
            start_y,
            merge_from_x,
            start_y - ctrl_dy,
            merge_to_x,
            end_y + ctrl_dy,
            merge_to_x,
            end_y,
            color_var
        ));
    }
}

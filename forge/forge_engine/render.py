"""Forge SVG renderer — clean SVG with semantic CSS classes.

Rendering conventions based on Structurizr / C4 model defaults:
  - Person: head+shoulders silhouette above a rounded label box
  - Software System: large rounded box, background #1168BD
  - Container: rounded box, background #438DD5
  - Component: rounded box, background #85BBF0
  - Database: cylinder shape
  - External system: grey (#999999)
  - Text layout: Name (bold) → Description (normal) → [Technology] (italic)
  - Relationships: solid grey lines with open arrowheads, label centered on line
  - Drop shadows on all structure elements
  - Legend/key box in bottom-right corner
"""

from .layout import Layout, LayoutNode, LayoutEdge, Rect
from .model import ElementKind
import html

# ─── Canonical C4 colour palette (from Structurizr default theme) ───
COLORS = {
    'person_bg':     '#08427B',
    'person_stroke':  '#073B6F',
    'system_bg':     '#1168BD',
    'system_stroke':  '#0E4D8B',
    'container_bg':  '#438DD5',
    'container_stroke': '#3178B9',
    'component_bg':  '#85BBF0',
    'component_stroke': '#6BA3D6',
    'database_bg':   '#438DD5',
    'database_stroke': '#3178B9',
    'external_bg':   '#999999',
    'external_stroke': '#6B6B6B',
    'stage_bg':      '#F5F5F5',
    'stage_stroke':  '#9E9E9E',
    'gate_bg':       '#FFF3E0',
    'gate_stroke':   '#E65100',
    'font_white':    '#FFFFFF',
    'font_dark':     '#333333',
    'rel_line':      '#707070',
    'pipe_line':     '#9E9E9E',
    'shadow':        'rgba(0,0,0,0.15)',
}

DEFAULT_CSS = f"""
    .forge-diagram {{
      font-family: 'Open Sans', system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif;
    }}
    .forge-title {{
      font-size: 20px; font-weight: 600; fill: #333; text-anchor: middle;
    }}

    /* ── Drop shadow filter ── */
    .forge-shadow {{ filter: url(#dropShadow); }}

    /* ── Element base ── */
    .forge-element text {{ text-anchor: middle; }}
    .forge-label--name {{ font-size: 14px; font-weight: 600; }}
    .forge-label--desc {{ font-size: 11px; font-weight: 400; opacity: 0.85; }}
    .forge-label--tech {{ font-size: 11px; font-weight: 400; font-style: italic; opacity: 0.7; }}
    .forge-label--kind {{ font-size: 10px; font-weight: 400; opacity: 0.55; }}

    /* ── Person ── */
    .forge-element--person .forge-person-head {{ fill: {COLORS['person_bg']}; }}
    .forge-element--person .forge-person-body {{ fill: {COLORS['person_bg']}; }}
    .forge-element--person rect {{ fill: {COLORS['person_bg']}; stroke: {COLORS['person_stroke']}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--person .forge-label--name {{ fill: #fff; }}
    .forge-element--person .forge-label--desc {{ fill: #b0c4de; }}
    .forge-element--person .forge-label--kind {{ fill: #7a9cc6; }}

    /* ── Software System ── */
    .forge-element--system rect {{ fill: {COLORS['system_bg']}; stroke: {COLORS['system_stroke']}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--system .forge-label--name {{ fill: #fff; }}
    .forge-element--system .forge-label--desc {{ fill: #c0d8ec; }}
    .forge-element--system .forge-label--tech {{ fill: #a8cce8; }}
    .forge-element--system .forge-label--kind {{ fill: #8bb8de; }}

    /* ── Container ── */
    .forge-element--container rect {{ fill: {COLORS['container_bg']}; stroke: {COLORS['container_stroke']}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--container .forge-label--name {{ fill: #fff; }}
    .forge-element--container .forge-label--desc {{ fill: #d4e6f5; }}
    .forge-element--container .forge-label--tech {{ fill: #c0d8ec; }}
    .forge-element--container .forge-label--kind {{ fill: #a0c4e0; }}

    /* ── Component ── */
    .forge-element--component rect {{ fill: {COLORS['component_bg']}; stroke: {COLORS['component_stroke']}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--component .forge-label--name {{ fill: #fff; }}
    .forge-element--component .forge-label--desc {{ fill: #e0eef8; }}
    .forge-element--component .forge-label--tech {{ fill: #c0d8ec; }}

    /* ── Database (cylinder) ── */
    .forge-element--database .forge-label--name {{ fill: #fff; }}
    .forge-element--database .forge-label--tech {{ fill: #c0d8ec; }}
    .forge-element--database .forge-label--kind {{ fill: #a0c4e0; }}

    /* ── Stage ── */
    .forge-element--stage rect {{ fill: {COLORS['stage_bg']}; stroke: {COLORS['stage_stroke']}; stroke-width: 2; rx: 6; ry: 6; }}
    .forge-element--stage .forge-label--name {{ fill: #333; }}
    .forge-element--stage .forge-label--desc {{ fill: #757575; }}
    .forge-element--stage .forge-label--tech {{ fill: #757575; }}

    /* ── Gate (diamond) ── */
    .forge-element--gate polygon {{ fill: {COLORS['gate_bg']}; stroke: {COLORS['gate_stroke']}; stroke-width: 2; }}
    .forge-element--gate .forge-label--name {{ fill: #BF360C; font-size: 9px; }}

    /* ── Relationships ── */
    .forge-relationship line {{ stroke: {COLORS['rel_line']}; stroke-width: 1.5; }}
    .forge-relationship path {{ stroke: {COLORS['rel_line']}; stroke-width: 1.5; fill: none; }}
    .forge-relationship--arrow {{ fill: {COLORS['rel_line']}; }}
    .forge-label--rel {{ font-size: 11px; fill: #555; text-anchor: middle; }}
    .forge-label--rel-tech {{ font-size: 10px; fill: #888; font-style: italic; text-anchor: middle; }}

    /* ── Pipeline connectors ── */
    .forge-connector line {{ stroke: {COLORS['pipe_line']}; stroke-width: 2.5; stroke-dasharray: 8,4; }}
    .forge-connector--arrow {{ fill: {COLORS['pipe_line']}; }}

    /* ── Legend ── */
    .forge-legend rect.forge-legend-bg {{ fill: #fff; stroke: #ccc; stroke-width: 1; rx: 4; ry: 4; }}
    .forge-legend text {{ font-size: 10px; fill: #555; }}
    .forge-legend .forge-legend-title {{ font-size: 11px; font-weight: 600; fill: #333; }}
    .forge-legend rect.forge-legend-swatch {{ stroke-width: 1; rx: 2; ry: 2; }}
"""


OUTLINE_CSS = """
    .forge-diagram {
      font-family: 'Open Sans', system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif;
    }
    .forge-title {
      font-size: 20px; font-weight: 600; fill: #333; text-anchor: middle;
    }

    /* ── No drop shadow in outline mode ── */
    .forge-shadow { filter: none; }

    /* ── Element base ── */
    .forge-element text { text-anchor: middle; }
    .forge-label--name { font-size: 14px; font-weight: 600; }
    .forge-label--desc { font-size: 11px; font-weight: 400; opacity: 0.7; }
    .forge-label--tech { font-size: 11px; font-weight: 400; font-style: italic; opacity: 0.6; }
    .forge-label--kind { font-size: 10px; font-weight: 400; opacity: 0.5; }

    /* ── Person (outline) ── */
    .forge-element--person .forge-person-head { fill: none; stroke: #08427B; stroke-width: 2; }
    .forge-element--person .forge-person-body { fill: none; stroke: #08427B; stroke-width: 2; }
    .forge-element--person rect { fill: none; stroke: #08427B; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--person .forge-label--name { fill: #08427B; }
    .forge-element--person .forge-label--desc { fill: #555; }
    .forge-element--person .forge-label--kind { fill: #888; }

    /* ── Software System (outline) ── */
    .forge-element--system rect { fill: none; stroke: #1168BD; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--system .forge-label--name { fill: #1168BD; }
    .forge-element--system .forge-label--desc { fill: #555; }
    .forge-element--system .forge-label--tech { fill: #777; }
    .forge-element--system .forge-label--kind { fill: #888; }

    /* ── Container (outline) ── */
    .forge-element--container rect { fill: none; stroke: #438DD5; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--container .forge-label--name { fill: #438DD5; }
    .forge-element--container .forge-label--desc { fill: #555; }
    .forge-element--container .forge-label--tech { fill: #777; }
    .forge-element--container .forge-label--kind { fill: #888; }

    /* ── Component (outline) ── */
    .forge-element--component rect { fill: none; stroke: #6BA3D6; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--component .forge-label--name { fill: #6BA3D6; }
    .forge-element--component .forge-label--desc { fill: #555; }
    .forge-element--component .forge-label--tech { fill: #777; }

    /* ── Database cylinder (outline) ── */
    .forge-element--database .forge-label--name { fill: #438DD5; }
    .forge-element--database .forge-label--tech { fill: #777; }
    .forge-element--database .forge-label--kind { fill: #888; }

    /* ── Stage (outline) ── */
    .forge-element--stage rect { fill: none; stroke: #9E9E9E; stroke-width: 2; rx: 6; ry: 6; }
    .forge-element--stage .forge-label--name { fill: #333; }
    .forge-element--stage .forge-label--desc { fill: #757575; }
    .forge-element--stage .forge-label--tech { fill: #757575; }

    /* ── Gate diamond (outline) ── */
    .forge-element--gate polygon { fill: none; stroke: #E65100; stroke-width: 2; }
    .forge-element--gate .forge-label--name { fill: #BF360C; font-size: 9px; }

    /* ── Relationships ── */
    .forge-relationship line { stroke: #707070; stroke-width: 1.5; }
    .forge-relationship path { stroke: #707070; stroke-width: 1.5; fill: none; }
    .forge-relationship--arrow { fill: #707070; }
    .forge-label--rel { font-size: 11px; fill: #555; text-anchor: middle; }
    .forge-label--rel-tech { font-size: 10px; fill: #888; font-style: italic; text-anchor: middle; }

    /* ── Pipeline connectors ── */
    .forge-connector line { stroke: #9E9E9E; stroke-width: 2.5; stroke-dasharray: 8,4; }
    .forge-connector--arrow { fill: #9E9E9E; }

    /* ── Legend ── */
    .forge-legend rect.forge-legend-bg { fill: #fff; stroke: #ccc; stroke-width: 1; rx: 4; ry: 4; }
    .forge-legend text { font-size: 10px; fill: #555; }
    .forge-legend .forge-legend-title { font-size: 11px; font-weight: 600; fill: #333; }
    .forge-legend rect.forge-legend-swatch { stroke-width: 2; rx: 2; ry: 2; }
"""


def render_svg(layout: Layout, style: str = "filled") -> str:
    o = []
    o.append(f'<svg xmlns="http://www.w3.org/2000/svg" '
             f'viewBox="0 0 {layout.width} {layout.height}" '
             f'width="{layout.width}" height="{layout.height}" '
             f'class="forge-diagram">')
    # White background rect for clean rendering in all contexts
    o.append(f'  <rect width="{layout.width}" height="{layout.height}" fill="#ffffff" />')
    o.append('  <defs>')
    css = OUTLINE_CSS if style == "outline" else DEFAULT_CSS
    o.append(f'    <style>{css}    </style>')

    # Drop shadow filter
    o.append('    <filter id="dropShadow" x="-4%" y="-4%" width="110%" height="114%">')
    o.append('      <feDropShadow dx="2" dy="3" stdDeviation="3" flood-color="#000" flood-opacity="0.12"/>')
    o.append('    </filter>')

    # Arrowhead markers
    o.append('    <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" '
             'markerWidth="7" markerHeight="7" orient="auto-start-reverse">'
             f'<path d="M 0 1 L 8 5 L 0 9 z" fill="{COLORS["rel_line"]}"/></marker>')
    o.append('    <marker id="arrow-pipe" viewBox="0 0 10 10" refX="9" refY="5" '
             'markerWidth="7" markerHeight="7" orient="auto-start-reverse">'
             f'<path d="M 0 1 L 8 5 L 0 9 z" fill="{COLORS["pipe_line"]}"/></marker>')
    o.append('  </defs>')

    # Title
    if layout.title:
        o.append(f'  <text x="{layout.width/2:.0f}" y="30" class="forge-title">'
                 f'{esc(layout.title)}</text>')

    # Edges first (under nodes)
    o.append('  <g class="forge-relationships">')
    for e in layout.edges:
        _render_edge(o, e, layout.nodes)
    o.append('  </g>')

    # Nodes
    o.append('  <g class="forge-elements">')
    for n in layout.nodes:
        _render_node(o, n, style)
    o.append('  </g>')

    # Legend
    _render_legend(o, layout, style)

    o.append('</svg>')
    return '\n'.join(o) + '\n'


# ─── Node Rendering ────────────────────────────────────────────────

def _render_node(o: list, n: LayoutNode, style: str = "filled"):
    r = n.rect
    css = _css_kind(n.kind)
    tag_cls = " forge-element--database" if "database" in n.tags else ""

    o.append(f'    <g class="forge-element forge-element--{css}{tag_cls}" data-id="{esc(n.id)}">')

    if n.kind == ElementKind.GATE:
        _render_gate(o, n)
    elif n.kind == ElementKind.PERSON:
        _render_person(o, n, style)
    elif "database" in n.tags:
        _render_cylinder(o, n, style)
    elif n.kind == ElementKind.STAGE:
        _render_stage(o, n)
    else:
        _render_box(o, n)

    o.append('    </g>')


def _render_person(o, n, style="filled"):
    """Structurizr-style person: head circle + shoulders arc above a label box."""
    r = n.rect
    cx = r.x + r.w / 2

    # The person silhouette sits above the box
    head_r = 20
    head_cy = r.y + head_r + 2
    shoulder_y = head_cy + head_r + 2
    shoulder_w = 36
    box_y = shoulder_y + 18
    box_h = r.y + r.h - box_y

    if style == "outline":
        # Single unified outline: one combined path for head+shoulders+box
        # Draw as: rounded rect for the label box, then a separate head circle
        # sitting above it — but no internal stroke between shoulders and box.
        stroke = COLORS['person_stroke']
        bx, bw = r.x, r.w
        rad = 8  # corner radius

        # Unified body path: shoulders arc merging into the box sides and bottom
        sl = cx - shoulder_w
        sr = cx + shoulder_w
        bot = box_y + box_h

        # Path: start at bottom-left (with radius), go up left side to shoulder
        # level, shoulder arc up to head area, back down right side, bottom-right
        # corner with radius, across bottom
        o.append(
            f'      <path d="'
            f'M {bx + rad:.0f} {bot:.0f} '                           # bottom-left corner start
            f'Q {bx:.0f} {bot:.0f} {bx:.0f} {bot - rad:.0f} '       # bottom-left radius
            f'L {bx:.0f} {box_y:.0f} '                               # up left side to box top
            f'L {sl:.0f} {box_y:.0f} '                               # across to shoulder-left
            f'Q {sl:.0f} {shoulder_y:.0f} {cx:.0f} {shoulder_y:.0f} ' # shoulder arc left
            f'Q {sr:.0f} {shoulder_y:.0f} {sr:.0f} {box_y:.0f} '     # shoulder arc right
            f'L {bx + bw:.0f} {box_y:.0f} '                          # across to box right
            f'L {bx + bw:.0f} {bot - rad:.0f} '                      # down right side
            f'Q {bx + bw:.0f} {bot:.0f} {bx + bw - rad:.0f} {bot:.0f} '  # bottom-right radius
            f'Z" '
            f'fill="none" stroke="{stroke}" stroke-width="2" />')

        # Separate head circle (disconnected from body, like a person icon)
        o.append(f'      <circle cx="{cx:.0f}" cy="{head_cy:.0f}" r="{head_r}" '
                 f'fill="none" stroke="{stroke}" stroke-width="2" />')
    else:
        # Filled mode: original multi-part rendering
        # Head
        o.append(f'      <circle cx="{cx:.0f}" cy="{head_cy:.0f}" r="{head_r}" '
                 f'class="forge-person-head" />')
        # Shoulders (arc)
        sl = cx - shoulder_w
        sr = cx + shoulder_w
        o.append(f'      <path d="M {sl:.0f} {box_y:.0f} '
                 f'Q {sl:.0f} {shoulder_y:.0f} {cx:.0f} {shoulder_y:.0f} '
                 f'Q {sr:.0f} {shoulder_y:.0f} {sr:.0f} {box_y:.0f} Z" '
                 f'class="forge-person-body" />')

        # Label box below the silhouette
        o.append(f'      <rect x="{r.x:.0f}" y="{box_y:.0f}" width="{r.w:.0f}" '
                 f'height="{box_h:.0f}" class="forge-shadow" />')

    # Text: name, then [Person], then description
    ty = box_y + 20
    o.append(f'      <text x="{cx:.0f}" y="{ty:.0f}" class="forge-label--name">{esc(n.label)}</text>')
    o.append(f'      <text x="{cx:.0f}" y="{ty + 16:.0f}" class="forge-label--kind">[Person]</text>')
    if n.description:
        _render_wrapped_text(o, cx, ty + 32, r.w - 20, n.description, "forge-label--desc")


def _render_box(o, n):
    """Standard rounded box: name → description → [Technology] → [Kind]."""
    r = n.rect
    cx = r.x + r.w / 2

    # Shadow + box
    o.append(f'      <rect x="{r.x:.0f}" y="{r.y:.0f}" width="{r.w:.0f}" '
             f'height="{r.h:.0f}" class="forge-shadow" />')

    # Text layout: vertically centered cluster
    lines = []
    lines.append(('name', n.label))
    kind_label = _kind_label(n.kind)
    if kind_label:
        lines.append(('kind', f'[{kind_label}]'))
    if n.description:
        lines.append(('desc', n.description))
    if n.sublabel:
        lines.append(('tech', n.sublabel))

    total_h = len(lines) * 16
    ty = r.y + (r.h - total_h) / 2 + 14

    for cls, text in lines:
        o.append(f'      <text x="{cx:.0f}" y="{ty:.0f}" class="forge-label--{cls}">{esc(text)}</text>')
        ty += 16


def _render_stage(o, n):
    """Pipeline stage: rounded box with no shadow, slightly different layout."""
    r = n.rect
    cx = r.x + r.w / 2

    o.append(f'      <rect x="{r.x:.0f}" y="{r.y:.0f}" width="{r.w:.0f}" '
             f'height="{r.h:.0f}" class="forge-shadow" />')

    lines = [('name', n.label)]
    if n.sublabel:
        lines.append(('tech', f'[{n.sublabel}]'))

    total_h = len(lines) * 16
    ty = r.y + (r.h - total_h) / 2 + 14

    for cls, text in lines:
        o.append(f'      <text x="{cx:.0f}" y="{ty:.0f}" class="forge-label--{cls}">{esc(text)}</text>')
        ty += 16


def _render_gate(o, n):
    """Diamond-shaped gate."""
    r = n.rect
    cx, cy = r.x + r.w / 2, r.y + r.h / 2
    pts = f"{cx:.0f},{r.y:.0f} {r.x + r.w:.0f},{cy:.0f} {cx:.0f},{r.y + r.h:.0f} {r.x:.0f},{cy:.0f}"
    o.append(f'      <polygon points="{pts}" />')

    # Wrap long gate names
    words = n.label.split('-')
    if len(n.label) <= 14:
        o.append(f'      <text x="{cx:.0f}" y="{cy + 4:.0f}" class="forge-label--name" '
                 f'dominant-baseline="central">{esc(n.label)}</text>')
    else:
        # Two lines
        mid = len(words) // 2
        l1 = '-'.join(words[:mid])
        l2 = '-'.join(words[mid:])
        o.append(f'      <text x="{cx:.0f}" y="{cy - 4:.0f}" class="forge-label--name" '
                 f'dominant-baseline="central">{esc(l1)}</text>')
        o.append(f'      <text x="{cx:.0f}" y="{cy + 10:.0f}" class="forge-label--name" '
                 f'dominant-baseline="central">{esc(l2)}</text>')


def _render_cylinder(o, n, style="filled"):
    """Database cylinder with proper SVG path."""
    r = n.rect
    cx = r.x + r.w / 2
    ry = 12  # ellipse cap height
    rx_half = r.w / 2
    stroke = COLORS['database_stroke']

    if style == "outline":
        # Single unified outline: one path for the outer cylinder contour
        # plus an inner arc for the top-cap "lip"
        top_y = r.y + ry
        bot_y = r.y + r.h - ry
        lx = r.x
        rx_right = r.x + r.w

        # Outer contour: top-left → top arc (back of cap) → top-right →
        # down right side → bottom arc → up left side → close
        o.append(
            f'      <path d="'
            f'M {lx:.0f} {top_y:.0f} '
            f'A {rx_half:.0f} {ry} 0 0 1 {rx_right:.0f} {top_y:.0f} '  # top arc (front)
            f'L {rx_right:.0f} {bot_y:.0f} '                             # right side down
            f'A {rx_half:.0f} {ry} 0 0 1 {lx:.0f} {bot_y:.0f} '         # bottom arc
            f'Z" '
            f'fill="none" stroke="{stroke}" stroke-width="2" />')

        # Top cap back arc (the visible rear ellipse rim)
        o.append(
            f'      <path d="'
            f'M {lx:.0f} {top_y:.0f} '
            f'A {rx_half:.0f} {ry} 0 0 0 {rx_right:.0f} {top_y:.0f}" '
            f'fill="none" stroke="{stroke}" stroke-width="2" />')
    else:
        bg = COLORS['database_bg']
        sw = "1.5"
        # Body
        o.append(f'      <rect x="{r.x:.0f}" y="{r.y + ry:.0f}" width="{r.w:.0f}" '
                 f'height="{r.h - 2 * ry:.0f}" fill="{bg}" stroke="{stroke}" stroke-width="{sw}" rx="0" ry="0" />')
        # Top cap (full ellipse)
        o.append(f'      <ellipse cx="{cx:.0f}" cy="{r.y + ry:.0f}" rx="{rx_half:.0f}" ry="{ry}" '
                 f'fill="{bg}" stroke="{stroke}" stroke-width="{sw}" />')
        # Bottom cap
        o.append(f'      <ellipse cx="{cx:.0f}" cy="{r.y + r.h - ry:.0f}" rx="{rx_half:.0f}" ry="{ry}" '
                 f'fill="{bg}" stroke="{stroke}" stroke-width="{sw}" />')
        # Side lines
        o.append(f'      <line x1="{r.x:.0f}" y1="{r.y + ry:.0f}" x2="{r.x:.0f}" y2="{r.y + r.h - ry:.0f}" '
                 f'stroke="{stroke}" stroke-width="{sw}" />')
        o.append(f'      <line x1="{r.x + r.w:.0f}" y1="{r.y + ry:.0f}" x2="{r.x + r.w:.0f}" y2="{r.y + r.h - ry:.0f}" '
                 f'stroke="{stroke}" stroke-width="{sw}" />')

    # Labels
    ty = r.y + r.h / 2 - 4
    o.append(f'      <text x="{cx:.0f}" y="{ty:.0f}" class="forge-label--name">{esc(n.label)}</text>')
    if n.sublabel:
        o.append(f'      <text x="{cx:.0f}" y="{ty + 16:.0f}" class="forge-label--tech">{esc(n.sublabel)}</text>')
    o.append(f'      <text x="{cx:.0f}" y="{ty + 32:.0f}" class="forge-label--kind">[Database]</text>')


# ─── Edge Rendering ────────────────────────────────────────────────

def _render_edge(o: list, edge: LayoutEdge, nodes: list[LayoutNode]):
    frm = next((n for n in nodes if n.id == edge.frm), None)
    to = next((n for n in nodes if n.id == edge.to), None)
    if not frm or not to:
        return

    fx, fy = frm.rect.x + frm.rect.w / 2, frm.rect.y + frm.rect.h / 2
    tx, ty = to.rect.x + to.rect.w / 2, to.rect.y + to.rect.h / 2

    ex1, ey1 = _edge_point(frm.rect, tx, ty)
    ex2, ey2 = _edge_point(to.rect, fx, fy)

    is_pipe = frm.kind == ElementKind.STAGE or to.kind == ElementKind.STAGE
    cls = "forge-connector" if is_pipe else "forge-relationship"
    marker = "url(#arrow-pipe)" if is_pipe else "url(#arrow)"

    o.append(f'    <g class="{cls}" data-from="{esc(edge.frm)}" data-to="{esc(edge.to)}">')
    o.append(f'      <line x1="{ex1:.1f}" y1="{ey1:.1f}" '
             f'x2="{ex2:.1f}" y2="{ey2:.1f}" marker-end="{marker}" />')

    if edge.label:
        mx = (ex1 + ex2) / 2
        my = (ey1 + ey2) / 2

        # Position label above the line with a white background pill
        label_text = edge.label
        tech_text = f'[{edge.technology}]' if edge.technology else None
        label_y = my - 8
        if tech_text:
            label_y = my - 14

        # White background pill behind the label
        pill_w = max(len(label_text), len(tech_text or '')) * 6.5 + 16
        pill_h = 20 if not tech_text else 34
        pill_x = mx - pill_w / 2
        pill_y = label_y - 12
        o.append(f'      <rect x="{pill_x:.0f}" y="{pill_y:.0f}" width="{pill_w:.0f}" '
                 f'height="{pill_h:.0f}" rx="4" ry="4" fill="#fff" fill-opacity="0.85" />')

        o.append(f'      <text x="{mx:.1f}" y="{label_y:.1f}" class="forge-label--rel">{esc(label_text)}</text>')
        if tech_text:
            o.append(f'      <text x="{mx:.1f}" y="{label_y + 14:.1f}" class="forge-label--rel-tech">{esc(tech_text)}</text>')

    o.append('    </g>')


# ─── Legend ─────────────────────────────────────────────────────────

def _render_legend(o: list, layout: Layout, style: str = "filled"):
    """Render a notation legend in the bottom-right corner."""
    # Collect unique element kinds present in the diagram
    kinds_seen = {}
    for n in layout.nodes:
        k = n.kind
        tag = "database" if "database" in n.tags else None
        key = (k, tag)
        if key not in kinds_seen:
            kinds_seen[key] = n

    if not kinds_seen:
        return

    entries = []
    for (kind, tag), node in kinds_seen.items():
        label = _kind_label(kind) or kind.value.capitalize()
        if tag == "database":
            label = "Database"
            color = COLORS['database_bg']
        else:
            color = {
                ElementKind.PERSON: COLORS['person_bg'],
                ElementKind.SYSTEM: COLORS['system_bg'],
                ElementKind.CONTAINER: COLORS['container_bg'],
                ElementKind.COMPONENT: COLORS['component_bg'],
                ElementKind.STAGE: COLORS['stage_bg'],
                ElementKind.GATE: COLORS['gate_bg'],
            }.get(kind, '#ccc')
        entries.append((label, color))

    row_h = 20
    pad = 10
    legend_w = 160
    legend_h = pad * 2 + len(entries) * row_h + 16

    lx = layout.width - legend_w - 20
    ly = layout.height - legend_h - 10

    o.append(f'  <g class="forge-legend">')
    o.append(f'    <rect class="forge-legend-bg" x="{lx:.0f}" y="{ly:.0f}" '
             f'width="{legend_w}" height="{legend_h:.0f}" />')
    o.append(f'    <text x="{lx + pad:.0f}" y="{ly + pad + 10:.0f}" class="forge-legend-title">Legend</text>')

    ey = ly + pad + 26
    for label, color in entries:
        if style == "outline":
            swatch_fill = "none"
            swatch_stroke = color
        else:
            swatch_fill = color
            swatch_stroke = color
        o.append(f'    <rect class="forge-legend-swatch" x="{lx + pad:.0f}" y="{ey - 8:.0f}" '
                 f'width="14" height="14" fill="{swatch_fill}" stroke="{swatch_stroke}" />')
        o.append(f'    <text x="{lx + pad + 22:.0f}" y="{ey + 3:.0f}">{esc(label)}</text>')
        ey += row_h
    o.append(f'  </g>')


# ─── Utilities ──────────────────────────────────────────────────────

def _edge_point(r: Rect, tx: float, ty: float):
    cx, cy = r.x + r.w / 2, r.y + r.h / 2
    dx, dy = tx - cx, ty - cy
    if abs(dx) < 0.01 and abs(dy) < 0.01:
        return cx, cy
    hw, hh = r.w / 2, r.h / 2
    sx = hw / abs(dx) if abs(dx) > 0.01 else 1e9
    sy = hh / abs(dy) if abs(dy) > 0.01 else 1e9
    s = min(sx, sy)
    return cx + dx * s, cy + dy * s


def _render_wrapped_text(o, cx, y, max_w, text, cls, line_h=14):
    """Simple word-wrap for description text."""
    words = text.split()
    lines = []
    line = []
    est_w = 0
    for w in words:
        w_est = len(w) * 6.5
        if est_w + w_est > max_w and line:
            lines.append(' '.join(line))
            line = [w]
            est_w = w_est
        else:
            line.append(w)
            est_w += w_est + 4
    if line:
        lines.append(' '.join(line))
    for l in lines[:3]:  # max 3 lines
        o.append(f'      <text x="{cx:.0f}" y="{y:.0f}" class="{cls}">{esc(l)}</text>')
        y += line_h


def _kind_label(kind: ElementKind) -> str | None:
    return {
        ElementKind.PERSON: "Person",
        ElementKind.SYSTEM: "Software System",
        ElementKind.CONTAINER: "Container",
        ElementKind.COMPONENT: "Component",
        ElementKind.STAGE: None,
        ElementKind.GATE: None,
        ElementKind.PIPELINE: "Pipeline",
        ElementKind.REPOSITORY: "Repository",
    }.get(kind)


def _css_kind(kind: ElementKind) -> str:
    return {
        ElementKind.PERSON: "person",
        ElementKind.SYSTEM: "system",
        ElementKind.CONTAINER: "container",
        ElementKind.COMPONENT: "component",
        ElementKind.STAGE: "stage",
        ElementKind.GATE: "gate",
        ElementKind.PIPELINE: "pipeline",
        ElementKind.REPOSITORY: "repository",
    }.get(kind, "element")


def _is_dark(hex_color: str) -> bool:
    """Quick luminance check for choosing font color."""
    h = hex_color.lstrip('#')
    if len(h) != 6:
        return False
    r, g, b = int(h[:2], 16), int(h[2:4], 16), int(h[4:6], 16)
    return (r * 0.299 + g * 0.587 + b * 0.114) < 160


def esc(s: str) -> str:
    return html.escape(s, quote=True)

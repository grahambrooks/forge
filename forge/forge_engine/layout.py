"""Forge layout engine — assigns positions to elements for SVG rendering.

Dimensions updated to match Structurizr-style proportions:
  - Structure elements: 240×120 (wider, taller for 3-line text)
  - Person elements: 240×160 (extra height for head+shoulders silhouette)
  - System elements: 280×140 (larger to emphasise scope)
  - Pipeline stages: 170×80
  - Gates: 70×70
"""

from dataclasses import dataclass, field
from typing import Optional
from .model import Model, Element, View, ViewKind, ElementKind, AutoLayout

# ── Dimensions (closer to Structurizr proportions) ──
NODE_W = 240.0
NODE_H = 120.0
PERSON_W = 240.0
PERSON_H = 160.0    # extra room for head+shoulders silhouette
SYSTEM_W = 280.0
SYSTEM_H = 140.0
STAGE_W = 170.0
STAGE_H = 80.0
GATE_W = 70.0
GATE_H = 70.0
H_GAP = 80.0
V_GAP = 70.0
PAD = 60.0
TITLE_H = 50.0      # space reserved for the diagram title


@dataclass
class Rect:
    x: float
    y: float
    w: float
    h: float


@dataclass
class LayoutNode:
    id: str
    label: str
    sublabel: Optional[str]
    kind: ElementKind
    tags: list[str]
    rect: Rect
    description: Optional[str] = None


@dataclass
class LayoutEdge:
    frm: str
    to: str
    label: str = ""
    technology: Optional[str] = None


@dataclass
class Layout:
    width: float
    height: float
    title: Optional[str]
    nodes: list[LayoutNode] = field(default_factory=list)
    edges: list[LayoutEdge] = field(default_factory=list)


def compute_layout(model: Model, view: View) -> Layout:
    if view.kind == ViewKind.SYSTEM_CONTEXT:
        return _layout_system_context(model, view)
    elif view.kind == ViewKind.CONTAINER:
        return _layout_container(model, view)
    elif view.kind == ViewKind.PIPELINE_VIEW:
        return _layout_pipeline(model, view)
    return Layout(400, 200, "Unknown view type")


# ─── Helpers ────────────────────────────────────────────────────────

def _dims_for(el: Element) -> tuple[float, float]:
    """Return (width, height) for an element based on its kind."""
    if el.kind == ElementKind.PERSON:
        return PERSON_W, PERSON_H
    if el.kind == ElementKind.SYSTEM:
        return SYSTEM_W, SYSTEM_H
    return NODE_W, NODE_H


def _make_node(el: Element, x: float, y: float) -> LayoutNode:
    w, h = _dims_for(el)
    sub = f"[{el.technology}]" if el.technology else None
    return LayoutNode(
        id=el.id, label=el.name, sublabel=sub,
        kind=el.kind, tags=el.tags,
        rect=Rect(x, y, w, h),
        description=el.description,
    )


# ─── System Context ────────────────────────────────────────────────

def _layout_system_context(model: Model, view: View) -> Layout:
    scope_id = view.scope or ""
    system = model.elements.get(scope_id)

    child_ids = {eid for eid, e in model.elements.items() if e.parent == scope_id}
    scope_set = {scope_id} | child_ids

    rels = model.relationships_involving(scope_set)
    actor_ids = []
    for r in rels:
        other = r.to if r.frm in scope_set else r.frm
        if other not in scope_set and other not in actor_ids:
            actor_ids.append(other)

    nodes = []
    edges = []

    # Actors on the left column
    y_cursor = TITLE_H
    for aid in actor_ids:
        actor = model.elements.get(aid)
        if not actor:
            continue
        nodes.append(_make_node(actor, PAD, y_cursor))
        y_cursor += _dims_for(actor)[1] + V_GAP

    actor_col_h = y_cursor - V_GAP + PAD

    # System on the right
    sys_x = PAD + PERSON_W + H_GAP * 2.5
    sys_y = TITLE_H + max(0, (actor_col_h - TITLE_H - SYSTEM_H) / 2)
    if system:
        nodes.append(LayoutNode(
            id=system.id, label=system.name, sublabel=None,
            kind=system.kind, tags=system.tags,
            rect=Rect(sys_x, sys_y, SYSTEM_W, SYSTEM_H),
            description=system.description,
        ))

    # Collapse edges to system level
    seen = set()
    for r in rels:
        frm = scope_id if r.frm in scope_set else r.frm
        to = scope_id if r.to in scope_set else r.to
        key = (frm, to)
        if key not in seen:
            seen.add(key)
            edges.append(LayoutEdge(frm=frm, to=to, label=r.label, technology=r.technology))

    w = sys_x + SYSTEM_W + PAD
    h = max(actor_col_h, sys_y + SYSTEM_H + PAD)
    title = view.title or (f"{system.name} — System Context" if system else "System Context")
    return Layout(width=w, height=h, title=title, nodes=nodes, edges=edges)


# ─── Container View ────────────────────────────────────────────────

def _layout_container(model: Model, view: View) -> Layout:
    scope_id = view.scope or ""
    system = model.elements.get(scope_id)

    containers = [e for e in model.elements.values()
                  if e.parent == scope_id and e.kind == ElementKind.CONTAINER]

    container_ids = {c.id for c in containers}
    rels = model.relationships_involving(container_ids)

    ext_ids = []
    for r in rels:
        for eid in (r.frm, r.to):
            if eid not in container_ids and eid != scope_id and eid not in ext_ids:
                ext_ids.append(eid)

    externals = [model.elements[eid] for eid in ext_ids if eid in model.elements]

    nodes = []

    # Externals on top row (centered)
    ext_total_w = sum(_dims_for(e)[0] for e in externals) + H_GAP * max(len(externals) - 1, 0)
    cols = 3
    grid_w = cols * NODE_W + (cols - 1) * H_GAP
    canvas_w = max(ext_total_w, grid_w) + PAD * 2
    ext_x = (canvas_w - ext_total_w) / 2

    for ext in externals:
        ew, eh = _dims_for(ext)
        nodes.append(_make_node(ext, ext_x, TITLE_H))
        ext_x += ew + H_GAP

    # Containers in a grid
    y_start = TITLE_H + (PERSON_H + V_GAP * 1.3 if externals else 0)
    grid_start_x = (canvas_w - grid_w) / 2

    for i, cont in enumerate(containers):
        col = i % cols
        row = i // cols
        x = grid_start_x + col * (NODE_W + H_GAP)
        y = y_start + row * (NODE_H + V_GAP)
        nodes.append(_make_node(cont, x, y))

    all_ids = {n.id for n in nodes}
    view_rels = model.relationships_between(all_ids)
    edges = [LayoutEdge(frm=r.frm, to=r.to, label=r.label, technology=r.technology)
             for r in view_rels]

    max_x = max((n.rect.x + n.rect.w for n in nodes), default=400) + PAD
    max_y = max((n.rect.y + n.rect.h for n in nodes), default=200) + PAD + 40  # legend room
    title = view.title or (f"{system.name} — Containers" if system else "Containers")
    return Layout(width=max(max_x, canvas_w), height=max_y, title=title, nodes=nodes, edges=edges)


# ─── Pipeline View ─────────────────────────────────────────────────

def _layout_pipeline(model: Model, view: View) -> Layout:
    pipeline_id = view.scope or ""
    pipeline = model.elements.get(pipeline_id)

    stages = [e for e in model.elements.values()
              if e.parent == pipeline_id and e.kind == ElementKind.STAGE]

    ordered = _topo_sort(stages, model.stage_links)

    nodes = []
    edges = []
    x = PAD
    stage_cy = TITLE_H + 30

    for stage in ordered:
        nodes.append(LayoutNode(
            id=stage.id, label=stage.name,
            sublabel=stage.properties.get('environment', None),
            kind=ElementKind.STAGE, tags=stage.tags,
            rect=Rect(x, stage_cy, STAGE_W, STAGE_H),
        ))
        x += STAGE_W + H_GAP

        gates = [e for e in model.elements.values()
                 if e.kind == ElementKind.GATE and e.parent == stage.id]
        for g in gates:
            gx = x - H_GAP / 2 - GATE_W / 2
            gy = stage_cy + (STAGE_H - GATE_H) / 2
            nodes.append(LayoutNode(
                id=g.id, label=g.name, sublabel=None,
                kind=ElementKind.GATE, tags=g.tags,
                rect=Rect(gx, gy, GATE_W, GATE_H),
            ))

    for i in range(1, len(ordered)):
        edges.append(LayoutEdge(frm=ordered[i - 1].id, to=ordered[i].id))

    w = x + PAD
    h = TITLE_H + STAGE_H + 100  # room for legend
    title = view.title or (f"{pipeline.name} — Pipeline" if pipeline else "Pipeline")
    return Layout(width=w, height=h, title=title, nodes=nodes, edges=edges)


def _topo_sort(stages, links):
    depth = {s.id: 0 for s in stages}
    for _ in range(len(stages)):
        for link in links:
            if link.frm in depth and link.to in depth:
                new = depth[link.frm] + 1
                if new > depth[link.to]:
                    depth[link.to] = new
    return sorted(stages, key=lambda s: depth.get(s.id, 0))

"""Forge semantic model — typed directed graph of elements and relationships."""

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class ElementKind(Enum):
    PERSON = "person"
    SYSTEM = "system"
    CONTAINER = "container"
    COMPONENT = "component"
    REPOSITORY = "repository"
    BRANCH = "branch"
    PIPELINE = "pipeline"
    STAGE = "stage"
    ENVIRONMENT = "environment"
    GATE = "gate"
    STEP = "step"
    ARTIFACT = "artifact"


class ViewKind(Enum):
    SYSTEM_CONTEXT = "systemContext"
    CONTAINER = "container"
    PIPELINE_VIEW = "pipelineView"


class AutoLayout(Enum):
    TOP_BOTTOM = "tb"
    LEFT_RIGHT = "lr"


@dataclass
class Element:
    id: str
    kind: ElementKind
    name: str
    description: Optional[str] = None
    technology: Optional[str] = None
    tags: list[str] = field(default_factory=list)
    parent: Optional[str] = None
    properties: dict[str, str] = field(default_factory=dict)
    children: list[str] = field(default_factory=list)


@dataclass
class Relationship:
    frm: str
    to: str
    label: str = ""
    technology: Optional[str] = None


@dataclass
class StageLink:
    frm: str
    to: str


@dataclass
class View:
    kind: ViewKind
    key: str
    scope: Optional[str] = None
    title: Optional[str] = None
    auto_layout: AutoLayout = AutoLayout.TOP_BOTTOM
    include_all: bool = False


@dataclass
class Model:
    name: str = ""
    description: str = ""
    elements: dict[str, Element] = field(default_factory=dict)
    relationships: list[Relationship] = field(default_factory=list)
    views: list[View] = field(default_factory=list)
    stage_links: list[StageLink] = field(default_factory=list)

    def add_element(self, el: Element):
        if el.parent and el.parent in self.elements:
            parent = self.elements[el.parent]
            if el.id not in parent.children:
                parent.children.append(el.id)
        self.elements[el.id] = el

    def add_relationship(self, rel: Relationship):
        self.relationships.append(rel)

    def relationships_between(self, ids: set[str]) -> list[Relationship]:
        return [r for r in self.relationships if r.frm in ids and r.to in ids]

    def relationships_involving(self, ids: set[str]) -> list[Relationship]:
        return [r for r in self.relationships if r.frm in ids or r.to in ids]

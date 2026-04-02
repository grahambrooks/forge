"""Forge DSL parser — hand-written recursive descent."""

from .model import (
    Model, Element, ElementKind, Relationship, StageLink,
    View, ViewKind, AutoLayout,
)


class ParseError(Exception):
    def __init__(self, msg, line=0, col=0):
        self.msg = msg
        self.line = line
        self.col = col
        super().__init__(f"Parse error at {line}:{col}: {msg}")


class Parser:
    def __init__(self, text: str):
        self.text = text
        self.pos = 0
        self.model = Model()
        self.scope: list[str] = []
        self.id_map: dict[str, str] = {}

    # ── Helpers ──

    def _line_col(self):
        line = self.text[:self.pos].count('\n') + 1
        col = self.pos - self.text[:self.pos].rfind('\n')
        return line, col

    def _error(self, msg):
        l, c = self._line_col()
        return ParseError(msg, l, c)

    def _at_end(self):
        return self.pos >= len(self.text)

    def _peek(self):
        return self.text[self.pos] if self.pos < len(self.text) else None

    def _advance(self):
        if self.pos < len(self.text):
            c = self.text[self.pos]
            self.pos += 1
            return c
        return None

    def _skip_ws(self):
        while True:
            while self.pos < len(self.text) and self.text[self.pos] in ' \t\r\n':
                self.pos += 1
            if self.pos + 1 < len(self.text) and self.text[self.pos:self.pos+2] == '//':
                while self.pos < len(self.text) and self.text[self.pos] != '\n':
                    self.pos += 1
                continue
            break

    def _expect(self, ch):
        self._skip_ws()
        c = self._advance()
        if c != ch:
            raise self._error(f"expected '{ch}', got '{c}'")

    def _parse_string(self):
        self._skip_ws()
        if self._peek() != '"':
            raise self._error("expected quoted string")
        self._advance()
        s = []
        while True:
            c = self._advance()
            if c is None:
                raise self._error("unterminated string")
            if c == '"':
                return ''.join(s)
            if c == '\\':
                c2 = self._advance()
                if c2:
                    s.append(c2)
            else:
                s.append(c)

    def _parse_ident(self):
        self._skip_ws()
        start = self.pos
        while self.pos < len(self.text) and (
            self.text[self.pos].isalnum() or self.text[self.pos] in '_-.*/'
        ):
            self.pos += 1
        if self.pos == start:
            raise self._error("expected identifier")
        return self.text[start:self.pos]

    def _peek_after_ws(self):
        saved = self.pos
        self._skip_ws()
        c = self._peek()
        self.pos = saved
        return c

    def _peek_keyword(self):
        saved = self.pos
        self._skip_ws()
        start = self.pos
        while self.pos < len(self.text) and (self.text[self.pos].isalnum() or self.text[self.pos] in '_.'):
            self.pos += 1
        word = self.text[start:self.pos]
        self.pos = saved
        return word if word else None

    def _skip_block(self):
        self._expect('{')
        depth = 1
        while depth > 0:
            c = self._advance()
            if c is None:
                raise self._error("unexpected EOF in block")
            if c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
            elif c == '"':
                while True:
                    c2 = self._advance()
                    if c2 is None:
                        raise self._error("unterminated string")
                    if c2 == '"':
                        break
                    if c2 == '\\':
                        self._advance()

    def _scoped_id(self, local):
        if not self.scope:
            return local
        return '.'.join(self.scope) + '.' + local

    def _resolve_ref(self, name):
        if name in self.id_map:
            return self.id_map[name]
        if self.scope:
            scoped = '.'.join(self.scope) + '.' + name
            if scoped in self.model.elements:
                return scoped
        return name

    # ── Top level ──

    def parse(self):
        self._skip_ws()
        kw = self._parse_ident()
        if kw != 'forge':
            raise self._error("expected 'forge'")
        self.model.name = self._parse_string()
        self._expect('{')
        while True:
            self._skip_ws()
            if self._peek() == '}':
                self._advance()
                break
            if self._at_end():
                raise self._error("unexpected EOF")
            kw = self._parse_ident()
            if kw == 'description':
                self.model.description = self._parse_string()
            elif kw == 'model':
                self._parse_model()
            elif kw == 'process':
                self._parse_process()
            elif kw == 'views':
                self._parse_views()
            elif kw == 'styles':
                self._skip_block()
            else:
                if self._peek_after_ws() == '{':
                    self._skip_block()
                elif self._peek_after_ws() == '"':
                    self._parse_string()
        return self.model

    # ── Model ──

    def _parse_model(self):
        self._expect('{')
        while True:
            self._skip_ws()
            if self._peek() == '}':
                self._advance()
                return
            if self._at_end():
                raise self._error("unexpected EOF in model")
            self._parse_model_stmt()

    def _parse_model_stmt(self):
        saved = self.pos
        first = self._parse_ident()
        self._skip_ws()

        if self._peek() == '=':
            self._advance()
            kind_str = self._parse_ident()
            kind_map = {
                'person': ElementKind.PERSON,
                'system': ElementKind.SYSTEM,
                'container': ElementKind.CONTAINER,
                'component': ElementKind.COMPONENT,
            }
            kind = kind_map.get(kind_str)
            if kind is None:
                raise self._error(f"unknown element kind '{kind_str}'")
            name = self._parse_string()
            full_id = self._scoped_id(first)
            parent = '.'.join(self.scope) if self.scope else None

            el = Element(id=full_id, kind=kind, name=name, parent=parent)
            self.id_map[first] = full_id

            if self._peek_after_ws() == '{':
                self._expect('{')
                self.scope.append(full_id if self.scope else first)
                while True:
                    self._skip_ws()
                    if self._peek() == '}':
                        self._advance()
                        break
                    if self._at_end():
                        raise self._error("unexpected EOF in element")

                    saved2 = self.pos
                    inner = self._parse_ident()
                    self._skip_ws()

                    if inner == 'description':
                        el.description = self._parse_string()
                    elif inner == 'technology':
                        el.technology = self._parse_string()
                    elif inner == 'tags':
                        while self._peek_after_ws() == '"':
                            el.tags.append(self._parse_string())
                    elif self._peek() == '=':
                        # Child element — save current element first
                        self.model.add_element(el)
                        self.pos = saved2
                        self._parse_model_stmt()
                        el = self.model.elements.get(full_id, el)
                        continue
                    elif self._peek() == '-':
                        self.pos = saved2
                        self._parse_relationship()
                        continue
                    else:
                        if self._peek_after_ws() == '"':
                            self._parse_string()
                        elif self._peek_after_ws() == '{':
                            self._skip_block()
                self.scope.pop()
            self.model.add_element(el)
            return

        if self._peek() == '-':
            self.pos = saved
            self._parse_relationship()
            return

        if first == 'description':
            self.model.description = self._parse_string()
            return

        if self._peek_after_ws() == '"':
            self._parse_string()
        elif self._peek_after_ws() == '{':
            self._skip_block()

    def _parse_relationship(self):
        frm_raw = self._parse_ident()
        self._skip_ws()
        self._expect('-')
        self._expect('>')
        to_raw = self._parse_ident()
        label = self._parse_string() if self._peek_after_ws() == '"' else ""
        tech = self._parse_string() if self._peek_after_ws() == '"' else None

        frm = self._resolve_ref(frm_raw)
        to = self._resolve_ref(to_raw)
        self.model.add_relationship(Relationship(frm=frm, to=to, label=label, technology=tech))

    # ── Process ──

    def _parse_process(self):
        self._expect('{')
        while True:
            self._skip_ws()
            if self._peek() == '}':
                self._advance()
                return
            if self._at_end():
                raise self._error("unexpected EOF in process")
            self._parse_process_stmt()

    def _parse_process_stmt(self):
        saved = self.pos
        first = self._parse_ident()
        self._skip_ws()

        if self._peek() == '=':
            self._advance()
            kind_str = self._parse_ident()
            if kind_str == 'repository':
                name = self._parse_string()
                el = Element(id=first, kind=ElementKind.REPOSITORY, name=name)
                self.id_map[first] = first
                if self._peek_after_ws() == '{':
                    self._expect('{')
                    while True:
                        self._skip_ws()
                        if self._peek() == '}':
                            self._advance()
                            break
                        prop = self._parse_ident()
                        if prop == 'url':
                            el.properties['url'] = self._parse_string()
                        elif prop == 'system':
                            sys = self._parse_ident()
                            el.properties['system'] = self._resolve_ref(sys)
                        else:
                            if self._peek_after_ws() == '"':
                                self._parse_string()
                            elif self._peek_after_ws() == '{':
                                self._skip_block()
                self.model.add_element(el)
            else:
                if self._peek_after_ws() == '"':
                    self._parse_string()
                if self._peek_after_ws() == '{':
                    self._skip_block()
            return

        if first == 'strategy':
            self._parse_string()
            self._skip_block()
            return

        if first == 'pipeline':
            self._parse_pipeline()
            return

        # Unknown
        if self._peek_after_ws() == '"':
            self._parse_string()
        if self._peek_after_ws() == '{':
            self._skip_block()

    def _parse_pipeline(self):
        pipeline_name = self._parse_string()
        pipeline_id = pipeline_name.replace(' ', '-').lower()
        self.model.add_element(Element(id=pipeline_id, kind=ElementKind.PIPELINE, name=pipeline_name))
        self.id_map[pipeline_id] = pipeline_id

        self._expect('{')
        while True:
            self._skip_ws()
            if self._peek() == '}':
                self._advance()
                return
            if self._at_end():
                raise self._error("unexpected EOF in pipeline")

            saved = self.pos
            first = self._parse_ident()
            self._skip_ws()

            if first == 'triggers':
                # Skip to end of line
                while self.pos < len(self.text) and self.text[self.pos] != '\n':
                    self.pos += 1
                continue

            if self._peek() == '=':
                self._advance()
                kind_str = self._parse_ident()
                if kind_str == 'stage':
                    stage_name = self._parse_string()
                    stage_id = f"{pipeline_id}.{first}"
                    el = Element(
                        id=stage_id, kind=ElementKind.STAGE,
                        name=stage_name, parent=pipeline_id,
                    )
                    self.id_map[first] = stage_id

                    if self._peek_after_ws() == '{':
                        self._expect('{')
                        while True:
                            self._skip_ws()
                            if self._peek() == '}':
                                self._advance()
                                break
                            if self._at_end():
                                raise self._error("unexpected EOF in stage")
                            prop = self._parse_ident()
                            if prop == 'needs':
                                dep = self._parse_ident()
                                dep_full = self._resolve_ref(dep)
                                self.model.stage_links.append(StageLink(frm=dep_full, to=stage_id))
                            elif prop == 'step':
                                self._parse_string()
                            elif prop == 'environment':
                                env = self._parse_ident()
                                el.properties['environment'] = env
                            elif prop == 'gate':
                                gate_name = self._parse_string()
                                gate_id = f"{stage_id}.gate"
                                gate = Element(id=gate_id, kind=ElementKind.GATE, name=gate_name, parent=stage_id)
                                if self._peek_after_ws() == '{':
                                    self._expect('{')
                                    while True:
                                        self._skip_ws()
                                        if self._peek() == '}':
                                            self._advance()
                                            break
                                        gp = self._parse_ident()
                                        if self._peek_after_ws() == '"':
                                            gate.properties[gp] = self._parse_string()
                                self.model.add_element(gate)
                            elif prop == 'produces':
                                self._parse_ident()  # artifact
                                self._parse_string()  # name
                                if self._peek_after_ws() == '{':
                                    self._skip_block()
                            else:
                                if self._peek_after_ws() == '"':
                                    self._parse_string()
                                elif self._peek_after_ws() == '{':
                                    self._skip_block()
                    self.model.add_element(el)
                else:
                    if self._peek_after_ws() == '"':
                        self._parse_string()
                    if self._peek_after_ws() == '{':
                        self._skip_block()
                continue

            # Unknown
            if self._peek_after_ws() == '"':
                self._parse_string()
            if self._peek_after_ws() == '{':
                self._skip_block()

    # ── Views ──

    def _parse_views(self):
        self._expect('{')
        while True:
            self._skip_ws()
            if self._peek() == '}':
                self._advance()
                return
            if self._at_end():
                raise self._error("unexpected EOF in views")

            kind_str = self._parse_ident()
            if kind_str == 'systemContext':
                scope_raw = self._parse_ident()
                scope = self._resolve_ref(scope_raw)
                key = self._parse_string()
                view = View(kind=ViewKind.SYSTEM_CONTEXT, key=key, scope=scope, auto_layout=AutoLayout.LEFT_RIGHT)
                self._parse_view_body(view)
                self.model.views.append(view)
            elif kind_str == 'container':
                scope_raw = self._parse_ident()
                scope = self._resolve_ref(scope_raw)
                key = self._parse_string()
                view = View(kind=ViewKind.CONTAINER, key=key, scope=scope, auto_layout=AutoLayout.TOP_BOTTOM)
                self._parse_view_body(view)
                self.model.views.append(view)
            elif kind_str == 'pipelineView':
                scope_raw = self._parse_string()
                scope = scope_raw.replace(' ', '-').lower()
                scope = self._resolve_ref(scope)
                key = self._parse_string()
                view = View(kind=ViewKind.PIPELINE_VIEW, key=key, scope=scope, auto_layout=AutoLayout.LEFT_RIGHT)
                self._parse_view_body(view)
                self.model.views.append(view)
            else:
                # Skip unknown view types
                while self._peek_after_ws() == '"' or (self._peek_after_ws() and self._peek_after_ws().isalnum()):
                    if self._peek_after_ws() == '"':
                        self._parse_string()
                    else:
                        self._parse_ident()
                if self._peek_after_ws() == '{':
                    self._skip_block()

    def _parse_view_body(self, view: View):
        self._expect('{')
        while True:
            self._skip_ws()
            if self._peek() == '}':
                self._advance()
                return
            if self._at_end():
                raise self._error("unexpected EOF in view")
            prop = self._parse_ident()
            if prop == 'include':
                self._skip_ws()
                if self._peek() == '*':
                    self._advance()
                    view.include_all = True
                else:
                    self._parse_ident()
            elif prop == 'autoLayout':
                d = self._parse_ident()
                view.auto_layout = AutoLayout.LEFT_RIGHT if d == 'lr' else AutoLayout.TOP_BOTTOM
            elif prop == 'title':
                view.title = self._parse_string()
            else:
                if self._peek_after_ws() == '"':
                    self._parse_string()
                elif self._peek_after_ws() == '{':
                    self._skip_block()


def parse(text: str) -> Model:
    p = Parser(text)
    return p.parse()

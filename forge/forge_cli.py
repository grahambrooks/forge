#!/usr/bin/env python3
"""Forge CLI — prototype entry point."""

import sys
import os
from pathlib import Path

from forge_engine.parser import parse
from forge_engine.layout import compute_layout
from forge_engine.render import render_svg


def main():
    args = sys.argv[1:]
    source = Path("forge.forge")
    view_filter = None
    out_dir = Path("forge-output")
    style = "filled"

    i = 0
    while i < len(args):
        a = args[i]
        if a == "build":
            pass
        elif a == "--source" and i + 1 < len(args):
            i += 1
            source = Path(args[i])
        elif a == "--view" and i + 1 < len(args):
            i += 1
            view_filter = args[i]
        elif a == "--out" and i + 1 < len(args):
            i += 1
            out_dir = Path(args[i])
        elif a == "--style" and i + 1 < len(args):
            i += 1
            style = args[i]
            if style not in ("filled", "outline"):
                print(f"Error: --style must be 'filled' or 'outline'", file=sys.stderr)
                sys.exit(1)
        elif a in ("--help", "-h"):
            print_help()
            return
        elif a.endswith(".forge"):
            source = Path(a)
        i += 1

    # Read
    try:
        text = source.read_text()
    except FileNotFoundError:
        print(f"Error: {source} not found", file=sys.stderr)
        sys.exit(1)

    # Parse
    try:
        model = parse(text)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    print(f'Parsed model: "{model.name}"', file=sys.stderr)
    print(f"  Elements: {len(model.elements)}", file=sys.stderr)
    print(f"  Relationships: {len(model.relationships)}", file=sys.stderr)
    print(f"  Views: {len(model.views)}", file=sys.stderr)

    # Render
    out_dir.mkdir(parents=True, exist_ok=True)

    for view in model.views:
        if view_filter and view.key != view_filter:
            continue

        layout = compute_layout(model, view)
        svg = render_svg(layout, style=style)

        path = out_dir / f"{view.key}.svg"
        path.write_text(svg)
        print(f"  Wrote: {path}", file=sys.stderr)

    print("Done.", file=sys.stderr)


def print_help():
    print("""forge — A unified software modeling tool (prototype)

USAGE:
    python forge_cli.py build [OPTIONS]

OPTIONS:
    --source <FILE>    Input .forge file (default: ./forge.forge)
    --view <NAME>      Render a specific view (default: all)
    --out <DIR>        Output directory (default: ./forge-output/)
    --style <MODE>     Rendering style: 'filled' (default) or 'outline'
    -h, --help         Show this help

EXAMPLES:
    python forge_cli.py build --source examples/payments.forge
    python forge_cli.py build --source examples/payments.forge --style outline
    python forge_cli.py build --source examples/payments.forge --view Containers
""")


if __name__ == "__main__":
    main()

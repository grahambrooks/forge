#!/usr/bin/env -S npx tsx
/**
 * Forge plugin: extract Container metadata from OpenAPI specs.
 *
 * Speaks the newline-delimited-JSON protocol over stdio. Matches files named
 * openapi.json or swagger.json (no YAML to keep the demo dependency-free) and
 * emits one inferred Container per spec, tagged with `api:rest` and stamped
 * with openapi:title / openapi:version / openapi:routes properties.
 *
 * Run via:
 *   forge analyze . --plugin "npx tsx examples/plugins/openapi/plugin.ts"
 */

import * as fs from "node:fs";
import * as readline from "node:readline";

type Init = {
  type: "init";
  protocol: number;
  forge_version: string;
  scan_root: string;
  options: unknown;
};

type AnalyzeFile = {
  type: "analyze_file";
  protocol: number;
  path: string;
  relative_path: string;
};

type Finalize = { type: "finalize"; protocol: number };

type Inbound = Init | AnalyzeFile | Finalize;

const out = process.stdout;
const rl = readline.createInterface({ input: process.stdin });

function send(obj: unknown): void {
  out.write(JSON.stringify(obj) + "\n");
}

function slug(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .replace(/-+/g, "-");
}

function processSpec(absPath: string, relPath: string): void {
  let spec: any;
  try {
    spec = JSON.parse(fs.readFileSync(absPath, "utf8"));
  } catch (e: any) {
    send({
      protocol: 1,
      type: "error",
      code: "parse_failed",
      message: String(e?.message ?? e),
      path: relPath,
    });
    return;
  }

  const title: string = spec?.info?.title ?? relPath;
  const version: string = spec?.info?.version ?? "unknown";
  const description: string | undefined = spec?.info?.description;
  const routes = spec?.paths ? Object.keys(spec.paths).length : 0;
  const id = `api.${slug(title)}`;

  send({
    protocol: 1,
    type: "patch",
    elements: [
      {
        id,
        kind: "Container",
        name: title,
        description,
        technology: `OpenAPI ${spec?.openapi ?? spec?.swagger ?? "?"}`,
        tags: ["api:rest"],
        properties: {
          "openapi:title": title,
          "openapi:version": version,
          "openapi:routes": String(routes),
        },
      },
    ],
  });
}

rl.on("line", (line) => {
  if (!line.trim()) return;
  let msg: Inbound;
  try {
    msg = JSON.parse(line);
  } catch {
    return; // malformed line — forge will log and continue
  }

  switch (msg.type) {
    case "init":
      send({
        protocol: 1,
        type: "ready",
        name: "openapi",
        version: "0.1.0",
        match: [
          { kind: "filename", pattern: "openapi.json" },
          { kind: "filename", pattern: "swagger.json" },
        ],
        wants_finalize: false,
      });
      break;

    case "analyze_file":
      processSpec(msg.path, msg.relative_path);
      break;

    case "finalize":
      send({ protocol: 1, type: "patch", elements: [] });
      break;
  }
});

rl.on("close", () => process.exit(0));

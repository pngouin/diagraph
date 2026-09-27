import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Model } from "./model";

export const DIAGRAPH_BIN = fileURLToPath(new URL("../../target/debug/diagraph", import.meta.url));

function tomlString(s: string): string {
  return JSON.stringify(s);
}

/** Indices in the model are resolved modulo the pool they point into, so any
 * shrunk model still describes a valid tree. */
export function resolvedExternals(model: Model): string[] {
  const componentNames = new Set(model.components.map((c) => c.name));
  return model.externals.map((e, i) => (componentNames.has(e) ? `${e}-ext${i}` : e));
}

export function writeTree(model: Model, root: string): void {
  const externals = resolvedExternals(model);
  model.components.forEach((c, i) => {
    const lines: string[] = [`name = ${tomlString(c.name)}`];
    if (c.environment !== null && model.environments.length > 0) {
      lines.push(`environment = ${tomlString(model.environments[c.environment % model.environments.length]!)}`);
    }
    if (c.edges.length > 0) lines.push("edges = [");
    for (const e of c.edges) {
      const external = e.target.kind === "external" && externals.length > 0;
      const target = external
        ? externals[e.target.index % externals.length]!
        : model.components[e.target.index % model.components.length]!;
      const targetName = tomlString(typeof target === "string" ? target : target.name);
      const fields = [`target = ${targetName}`];
      if (e.via !== null) fields.push(`via = ${tomlString(e.via)}`);
      if (e.data !== null) fields.push(`data = ${tomlString(e.data)}`);
      if (external) fields.push("external = true");
      if (e.fromPart !== null && c.parts.length > 0) {
        fields.push(`from_part = ${tomlString(c.parts[e.fromPart % c.parts.length]!.name)}`);
      }
      if (!external && e.toPart !== null && typeof target !== "string" && target.parts.length > 0) {
        fields.push(`to_part = ${tomlString(target.parts[e.toPart % target.parts.length]!.name)}`);
      }
      lines.push(fields.length === 1 ? `  ${targetName},` : `  { ${fields.join(", ")} },`);
    }
    if (c.edges.length > 0) lines.push("]");
    c.parts.forEach((p, pi) => {
      const key = `parts.${tomlString(p.name)}`;
      lines.push("", `[${key}]`);
      for (const pe of c.partEdges) {
        if (pe.from % c.parts.length !== pi) continue;
        lines.push("", `[[${key}.edges]]`, `target = ${tomlString(c.parts[pe.to % c.parts.length]!.name)}`);
        if (pe.via !== null) lines.push(`via = ${tomlString(pe.via)}`);
      }
    });
    const dir = join(root, `c${i}`);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "diagraph.toml"), lines.join("\n") + "\n");
  });
}

export function renderHtml(root: string, out: string): void {
  try {
    execFileSync(DIAGRAPH_BIN, ["view", "--root", root, "-o", out], { stdio: "pipe" });
  } catch (err) {
    const stderr = (err as { stderr?: Buffer }).stderr?.toString() ?? String(err);
    throw new Error(`diagraph view rejected a generated tree (generator or pipeline bug):\n${stderr}`);
  }
}

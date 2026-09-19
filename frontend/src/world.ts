import { packGrid, packRow, type Rect, union } from "./geometry";
import { measureWidth } from "./text";
import type { JsonView, Payload } from "./types";

export interface PartNode {
  uid: string;
  name: string;
  owner: string;
  rect: Rect;
}

export interface ComponentNode {
  name: string;
  environment: string | null;
  rect: Rect;
  hasParts: boolean;
  parts: PartNode[];
}

export interface ExternalNode {
  name: string;
  rect: Rect;
}

export interface EnvironmentBox {
  name: string | null;
  rect: Rect;
  components: ComponentNode[];
}

export type EndpointRef =
  | { type: "component"; name: string }
  | { type: "external"; name: string }
  | { type: "part"; owner: string; uid: string };

export type Anchor = { kind: "rect" } | { kind: "part"; uid: string };

export interface EndpointSide {
  ref: EndpointRef;
  anchor: Anchor;
}

export interface WorldEdge {
  id: string;
  label: string | null;
  crossEnvironment: boolean;
  scope: "boundary" | "part-internal";
  from: EndpointSide;
  to: EndpointSide;
}

export interface World {
  environments: EnvironmentBox[];
  externals: ExternalNode[];
  edges: WorldEdge[];
  componentsByName: Map<string, ComponentNode>;
  externalsByName: Map<string, ExternalNode>;
}

/** Environments and externals can be dragged/resized after layout, so bounds
 * must be recomputed from current rects rather than cached once at build time. */
export function computeBounds(world: World): Rect {
  let bounds: Rect | null = null;
  for (const env of world.environments) bounds = bounds ? union(bounds, env.rect) : env.rect;
  for (const ext of world.externals) bounds = bounds ? union(bounds, ext.rect) : ext.rect;
  return bounds ?? { x: 0, y: 0, w: 0, h: 0 };
}

const PART_H = 30;
const PART_GAP = 26;
const COMPONENT_HEADER_H = 24;
const COMPONENT_PAD = 18;
const COMPONENT_MIN_W = 96;
const COMPONENT_PLAIN_H = 46;
const COMPONENT_GAP = 56;
const ENV_HEADER_H = 30;
const ENV_PAD = 24;
const ENV_GAP = 72;
const EXTERNAL_SIZE = 52;
const EXTERNAL_GAP = 40;

function labelText(name: string, extra = 32, min = 84, max = 200): number {
  return Math.min(max, Math.max(min, measureWidth(name) + extra));
}

/** Mirrors `with_remote_part_hint` on the Rust side: a zoomed-view boundary
 * edge's label is the plain edge label, optionally with " → part" appended. */
function labelMatches(globalLabel: string | null, zoomedLabel: string | null): boolean {
  if (zoomedLabel === globalLabel) return true;
  if (globalLabel != null && zoomedLabel != null) return zoomedLabel.startsWith(`${globalLabel} → `);
  if (globalLabel == null && zoomedLabel != null) return zoomedLabel.startsWith("→ ");
  return false;
}

interface BoundaryDescriptor {
  neighbor: EndpointRef;
  direction: "out" | "in";
  localAnchor: Anchor;
  label: string | null;
  used: boolean;
}

export function buildWorld(payload: Payload): World {
  const global = payload.views.global;

  // 1. Component sizing (parts nested inside), grouped by environment.
  const componentsByName = new Map<string, ComponentNode>();
  const byEnv = new Map<string | null, ComponentNode[]>();

  for (const node of global.nodes) {
    if (node.kind !== "component") continue;
    const zoomed = payload.views.zoomed[node.label];
    const parts = zoomed ? zoomed.nodes.filter((n) => n.kind === "part") : [];
    const hasParts = parts.length > 0;

    let w: number;
    let h: number;
    let partNodes: PartNode[] = [];
    if (hasParts) {
      const sizes = parts.map((p) => ({ w: labelText(p.label, 18, 60, 160), h: PART_H }));
      const grid = packGrid(sizes, PART_GAP);
      w = Math.max(grid.size.w + COMPONENT_PAD * 2, labelText(node.label, 32, COMPONENT_MIN_W));
      h = COMPONENT_HEADER_H + grid.size.h + COMPONENT_PAD * 2;
      partNodes = parts.map((p, i) => {
        const pos = grid.positions[i]!;
        const size = sizes[i]!;
        return {
          uid: `${node.label}\u0000${p.id}`,
          name: p.label,
          owner: node.label,
          rect: {
            x: COMPONENT_PAD + pos.x,
            y: COMPONENT_HEADER_H + COMPONENT_PAD + pos.y,
            w: size.w,
            h: size.h,
          },
        };
      });
    } else {
      w = labelText(node.label, 32, COMPONENT_MIN_W);
      h = COMPONENT_PLAIN_H;
    }

    const component: ComponentNode = {
      name: node.label,
      environment: node.environment,
      rect: { x: 0, y: 0, w, h },
      hasParts,
      parts: partNodes,
    };
    componentsByName.set(node.label, component);
    const bucket = byEnv.get(node.environment) ?? [];
    bucket.push(component);
    byEnv.set(node.environment, bucket);
  }

  // 2. Pack components inside each environment, then environments in a row.
  const envNames = [...byEnv.keys()].sort((a, b) => (a ?? "").localeCompare(b ?? ""));
  const environments: EnvironmentBox[] = envNames.map((envName) => {
    const members = byEnv.get(envName)!;
    const grid = packGrid(
      members.map((c) => ({ w: c.rect.w, h: c.rect.h })),
      COMPONENT_GAP
    );
    members.forEach((c, i) => {
      const pos = grid.positions[i]!;
      c.rect = { ...c.rect, x: ENV_PAD + pos.x, y: ENV_HEADER_H + ENV_PAD + pos.y };
    });
    const w = grid.size.w + ENV_PAD * 2;
    const h = ENV_HEADER_H + grid.size.h + ENV_PAD * 2;
    return { name: envName, rect: { x: 0, y: 0, w, h }, components: members };
  });

  const envRow = packRow(
    environments.map((e) => e.rect),
    ENV_GAP
  );
  environments.forEach((e, i) => {
    const pos = envRow.positions[i]!;
    e.rect = { ...e.rect, x: pos.x, y: pos.y };
    for (const c of e.components) c.rect = { ...c.rect, x: c.rect.x + pos.x, y: c.rect.y + pos.y };
  });

  // 3. External targets in a lane below the environment row.
  const externalsByName = new Map<string, ExternalNode>();
  const externalNames = global.nodes.filter((n) => n.kind === "external").map((n) => n.label);
  const extRow = packRow(
    externalNames.map(() => ({ w: EXTERNAL_SIZE, h: EXTERNAL_SIZE })),
    EXTERNAL_GAP
  );
  const extY = envRow.size.h + 72;
  const extXOffset = (envRow.size.w - extRow.size.w) / 2;
  const externals: ExternalNode[] = externalNames.map((name, i) => {
    const pos = extRow.positions[i]!;
    const ext: ExternalNode = {
      name,
      rect: { x: extXOffset + pos.x, y: extY + pos.y, w: EXTERNAL_SIZE, h: EXTERNAL_SIZE },
    };
    externalsByName.set(name, ext);
    return ext;
  });

  const edges = buildEdges(global, payload, componentsByName);

  return { environments, externals, edges, componentsByName, externalsByName };
}

function resolveGlobalRef(
  label: string,
  kind: "component" | "external"
): EndpointRef {
  return kind === "component" ? { type: "component", name: label } : { type: "external", name: label };
}

function buildEdges(
  global: JsonView,
  payload: Payload,
  componentsByName: Map<string, ComponentNode>
): WorldEdge[] {
  const globalIdToRef = new Map<string, EndpointRef>();
  for (const n of global.nodes) {
    if (n.kind === "component" || n.kind === "external") {
      globalIdToRef.set(n.id, resolveGlobalRef(n.label, n.kind));
    }
  }

  const boundaryDescriptors = new Map<string, BoundaryDescriptor[]>();
  const partInternalEdges: WorldEdge[] = [];
  let internalCounter = 0;

  for (const name of componentsByName.keys()) {
    const zoomed = payload.views.zoomed[name];
    if (!zoomed) continue;

    type Local =
      | { type: "part"; uid: string }
      | { type: "center" }
      | { type: "neighbor"; ref: EndpointRef };

    const localIdToRef = new Map<string, Local>();
    for (const n of zoomed.nodes) {
      if (n.kind === "part") {
        localIdToRef.set(n.id, { type: "part", uid: `${name}\u0000${n.id}` });
      } else if (n.kind === "component" && n.label === name) {
        localIdToRef.set(n.id, { type: "center" });
      } else if (n.kind === "component" || n.kind === "external") {
        localIdToRef.set(n.id, { type: "neighbor", ref: resolveGlobalRef(n.label, n.kind) });
      }
    }

    const descriptors: BoundaryDescriptor[] = [];
    for (const e of zoomed.edges) {
      const a = localIdToRef.get(e.from);
      const b = localIdToRef.get(e.to);
      if (!a || !b) continue;
      if (a.type !== "neighbor" && b.type !== "neighbor") {
        // Purely internal part-to-part edge.
        if (a.type === "part" && b.type === "part") {
          partInternalEdges.push({
            id: `internal-${internalCounter++}`,
            label: e.label,
            crossEnvironment: false,
            scope: "part-internal",
            from: { ref: { type: "part", owner: name, uid: a.uid }, anchor: { kind: "rect" } },
            to: { ref: { type: "part", owner: name, uid: b.uid }, anchor: { kind: "rect" } },
          });
        }
        continue;
      }
      const local = a.type !== "neighbor" ? a : b;
      const neighbor = (a.type === "neighbor" ? a : b) as Extract<Local, { type: "neighbor" }>;
      const direction: "out" | "in" = a.type !== "neighbor" ? "out" : "in";
      descriptors.push({
        neighbor: neighbor.ref,
        direction,
        localAnchor: local.type === "part" ? { kind: "part", uid: local.uid } : { kind: "rect" },
        label: e.label,
        used: false,
      });
    }
    boundaryDescriptors.set(name, descriptors);
  }

  function findAnchor(componentName: string, direction: "out" | "in", neighbor: EndpointRef, label: string | null): Anchor {
    const component = componentsByName.get(componentName);
    if (!component || !component.hasParts) return { kind: "rect" };
    const descriptors = boundaryDescriptors.get(componentName) ?? [];
    const match = descriptors.find(
      (d) =>
        !d.used &&
        d.direction === direction &&
        sameRef(d.neighbor, neighbor) &&
        labelMatches(label, d.label)
    );
    if (!match) return { kind: "rect" };
    match.used = true;
    return match.localAnchor;
  }

  const boundaryEdges: WorldEdge[] = global.edges.map((e, i) => {
    const fromRef = globalIdToRef.get(e.from)!;
    const toRef = globalIdToRef.get(e.to)!;
    const fromAnchor =
      fromRef.type === "component" ? findAnchor(fromRef.name, "out", toRef, e.label) : { kind: "rect" as const };
    const toAnchor =
      toRef.type === "component" ? findAnchor(toRef.name, "in", fromRef, e.label) : { kind: "rect" as const };
    return {
      id: `boundary-${i}`,
      label: e.label,
      crossEnvironment: e.crossEnvironment,
      scope: "boundary",
      from: { ref: fromRef, anchor: fromAnchor },
      to: { ref: toRef, anchor: toAnchor },
    };
  });

  return [...boundaryEdges, ...partInternalEdges];
}

function sameRef(a: EndpointRef, b: EndpointRef): boolean {
  if (a.type !== b.type) return false;
  if (a.type === "component" && b.type === "component") return a.name === b.name;
  if (a.type === "external" && b.type === "external") return a.name === b.name;
  if (a.type === "part" && b.type === "part") return a.uid === b.uid;
  return false;
}

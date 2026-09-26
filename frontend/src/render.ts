import { edgeEndpoints, worldRectOfPart } from "./anchors";
import { withAlpha } from "./colors";
import { maxGrowth, rectCenter, smoothstep, type Rect } from "./geometry";
import { curvePolyline, type Curve, type Polyline } from "./curves";
import { placeLabel } from "./labels";
import { bowCurve, chooseShift, memberBow, shiftOf, type RouteMember, type RouteObstacle } from "./routing";
import { COMPONENT_FONT, ENVIRONMENT_FONT, fitText, measureWidth, MONO_FONT } from "./text";
import type { ComponentNode, EndpointRef, EndpointSide, World, WorldEdge } from "./world";

const SVG_NS = "http://www.w3.org/2000/svg";
// Parts reveal once a component has been zoomed in well past the initial
// fit-all scale, not at any fixed pixel size — otherwise a compact monorepo
// where "fit all" already sits close to a component's natural size would
// reveal parts before the user asked to step in.
const REVEAL_START_MULT = 2.6;
const REVEAL_END_MULT = 4.6;
// How far a user-dragged edge label may stray from its edge's midpoint —
// enough to dodge an overlap, not enough to read as detached from the edge.
const MAX_LABEL_DRAG = 120;
const PARALLEL_SPACING = 18;

export type SelectableRef = EndpointRef | { type: "environment"; name: string | null };

export interface RenderCallbacks {
  onSelect(ref: SelectableRef | null): void;
  onFocusRequest(rect: Rect): void;
  getScale(): number;
  requestTick(): void;
}

export interface RenderHandles {
  tick(): void;
  setSelection(ref: SelectableRef | null): void;
  setSearch(query: string): void;
  edgesTouching(ref: SelectableRef): WorldEdge[];
}

function el<K extends keyof SVGElementTagNameMap>(tag: K): SVGElementTagNameMap[K] {
  return document.createElementNS(SVG_NS, tag);
}

function refEquals(a: SelectableRef, b: SelectableRef): boolean {
  if (a.type !== b.type) return false;
  if (a.type === "component" && b.type === "component") return a.name === b.name;
  if (a.type === "external" && b.type === "external") return a.name === b.name;
  if (a.type === "environment" && b.type === "environment") return a.name === b.name;
  if (a.type === "part" && b.type === "part") return a.uid === b.uid;
  return false;
}

// A boundary edge's `ref` always names the owning component, even when it
// visually anchors to one of that component's parts (`anchor.kind ===
// "part"`) — parts aren't nodes in the global graph. So a part selection
// only "touches" such an edge through its anchor, not through `ref` equality.
function edgeSideMatches(ref: SelectableRef, side: EndpointSide): boolean {
  if (refEquals(ref, side.ref)) return true;
  if (ref.type === "part" && side.ref.type === "component" && side.anchor.kind === "part") {
    return side.ref.name === ref.owner && side.anchor.uid === ref.uid;
  }
  return false;
}

export function mount(root: SVGGElement, world: World, envColor: (env: string | null) => string, cb: RenderCallbacks): RenderHandles {
  const revealFactors = new Map<string, number>();
  let baseScale: number | null = null;
  const fittedLabels = new Map<SVGTextElement, string>();

  // Text is sized in world units times --inv-zoom, so the box width available
  // to the unscaled font is `width / invZoom`.
  function setFittedText(text: SVGTextElement, full: string, width: number, invZoom: number, font: string) {
    const budget = Math.round(width / invZoom);
    const key = `${budget}\u0000${full}`;
    if (fittedLabels.get(text) === key) return;
    fittedLabels.set(text, key);
    text.textContent = fitText(full, budget, font);
  }

  function titled(g: SVGGElement, name: string) {
    const title = el("title");
    title.textContent = name;
    g.appendChild(title);
  }

  const envFrameLayer = el("g");
  const edgeLayer = el("g");
  const componentLayer = el("g");
  // Part-to-part edges connect nodes *inside* an already-drawn component
  // interior, so they must paint above componentLayer or the interior
  // background hides them once revealed.
  const internalEdgeLayer = el("g");
  const externalLayer = el("g");
  const edgeLabelLayer = el("g");
  root.append(envFrameLayer, edgeLayer, componentLayer, internalEdgeLayer, externalLayer, edgeLabelLayer);

  let selection: SelectableRef | null = null;
  let searchQuery = "";

  interface ComponentEntry {
    g: SVGGElement;
    frame: SVGRectElement;
    interior: SVGRectElement | null;
    caption: SVGTextElement | null;
    resizeHandle: SVGGElement;
    parts: Map<string, { g: SVGGElement; resizeHandle: SVGGElement }>;
  }
  const componentGroups = new Map<string, ComponentEntry>();
  const externalGroups = new Map<string, SVGGElement>();
  const environmentGroups = new Map<
    string | null,
    { g: SVGGElement; frame: SVGRectElement; label: SVGTextElement; resizeHandle: SVGGElement }
  >();
  interface EdgeLineEntry {
    path: SVGPathElement;
    label: SVGTextElement | null;
    labelBg: SVGRectElement | null;
    edge: WorldEdge;
    /** Set once the user drags the label; overrides the auto bow placement. */
    labelOffset: { dx: number; dy: number } | null;
    /** Edge midpoint as of the last tick, so a fresh drag can seed labelOffset without a jump. */
    lastAnchor: { x: number; y: number };
    /** Which side this edge bows to, and how many lanes out, among edges joining the same two nodes. */
    lane: { side: 1 | -1; rank: number; along: 1 | -1 };
    /** Obstacle keys this edge may pass through: its endpoints, their environments, its anchor parts. */
    touches: Set<string>;
    /** Label extent at --inv-zoom 1, measured once; the text never changes and scales linearly. */
    labelSize: { w: number; h: number; ascent: number } | null;
    /** Candidate chosen last tick, so placement can prefer to stay put. */
    labelKey: number | null;
  }
  const edgeLines = new Map<string, EdgeLineEntry>();

  function labelMatchesSearch(name: string): boolean {
    return searchQuery !== "" && name.toLowerCase().includes(searchQuery);
  }

  // --- environment frames -------------------------------------------------
  for (const env of world.environments) {
    const g = el("g");
    g.setAttribute("class", "environment");
    if (env.name !== null) g.dataset.name = env.name;
    const color = envColor(env.name);
    const frame = el("rect");
    frame.setAttribute("class", "environment-frame");
    frame.setAttribute("rx", "16");
    frame.setAttribute("fill", withAlpha(color, 0.05));
    frame.setAttribute("stroke", withAlpha(color, 0.55));
    const label = el("text");
    label.setAttribute("class", "environment-label");
    label.setAttribute("x", "20");
    label.setAttribute("y", "24");
    label.setAttribute("fill", color);
    label.textContent = env.name ?? "no environment";
    g.append(frame, label);
    titled(g, env.name ?? "no environment");
    g.addEventListener("click", (ev) => {
      ev.stopPropagation();
      cb.onSelect({ type: "environment", name: env.name });
    });
    g.addEventListener("dblclick", (ev) => {
      ev.stopPropagation();
      cb.onFocusRequest(env.rect);
    });
    attachDrag(g, (dx, dy) => {
      env.rect.x += dx;
      env.rect.y += dy;
      for (const c of env.components) {
        c.rect.x += dx;
        c.rect.y += dy;
      }
      cb.requestTick();
    });
    const resizeHandle = attachResize(
      g,
      () => {
        const maxRight = env.components.reduce((m, c) => Math.max(m, c.rect.x - env.rect.x + c.rect.w), 0);
        const maxBottom = env.components.reduce((m, c) => Math.max(m, c.rect.y - env.rect.y + c.rect.h), 0);
        return { w: Math.max(160, maxRight + 24), h: Math.max(100, maxBottom + 24) };
      },
      () => ({ w: env.rect.w, h: env.rect.h }),
      (w, h) => {
        env.rect.w = w;
        env.rect.h = h;
      },
      () => {
        const siblings = world.environments.filter((e) => e !== env).map((e) => e.rect);
        return maxGrowth(env.rect, siblings, 24);
      }
    );
    envFrameLayer.appendChild(g);
    environmentGroups.set(env.name, { g, frame, label, resizeHandle });
  }

  // --- components + nested parts -------------------------------------------
  for (const env of world.environments) {
    for (const component of env.components) {
      buildComponent(component, envColor(env.name));
    }
  }

  function buildComponent(component: ComponentNode, color: string) {
    const g = el("g");
    g.setAttribute("class", "component");
    g.dataset.name = component.name;
    if (component.environment !== null) g.dataset.environment = component.environment;

    const frame = el("rect");
    frame.setAttribute("class", "component-frame");
    frame.setAttribute("rx", component.hasParts ? "12" : "9");
    frame.setAttribute("fill", withAlpha(color, component.hasParts ? 0.07 : 0.16));
    frame.setAttribute("stroke", color);
    g.appendChild(frame);

    let interior: SVGRectElement | null = null;
    const partEls = new Map<string, { g: SVGGElement; resizeHandle: SVGGElement }>();
    let caption: SVGTextElement | null = null;

    if (component.hasParts) {
      interior = el("rect");
      interior.setAttribute("class", "component-interior");
      interior.setAttribute("rx", "8");
      interior.setAttribute("fill", "var(--paper)");
      g.appendChild(interior);

      caption = el("text");
      caption.setAttribute("class", "component-caption");
      caption.textContent = `${component.parts.length} part${component.parts.length === 1 ? "" : "s"}`;
      g.appendChild(caption);

      for (const part of component.parts) {
        const pg = el("g");
        pg.setAttribute("class", "part");
        pg.dataset.uid = part.uid;
        pg.dataset.name = part.name;
        const prect = el("rect");
        prect.setAttribute("class", "part-frame");
        prect.setAttribute("rx", "6");
        pg.appendChild(prect);
        const ptext = el("text");
        ptext.setAttribute("class", "part-label");
        ptext.textContent = part.name;
        pg.appendChild(ptext);
        titled(pg, part.name);
        g.appendChild(pg);

        pg.addEventListener("click", (ev) => {
          ev.stopPropagation();
          cb.onSelect({ type: "part", owner: part.owner, uid: part.uid });
        });
        attachDrag(pg, (dx, dy) => {
          const owner = world.componentsByName.get(part.owner)!;
          const interiorBounds = interiorRect(owner);
          part.rect.x = clampAxis(part.rect.x + dx, interiorBounds.x - owner.rect.x, interiorBounds.x - owner.rect.x + interiorBounds.w - part.rect.w);
          part.rect.y = clampAxis(part.rect.y + dy, interiorBounds.y - owner.rect.y, interiorBounds.y - owner.rect.y + interiorBounds.h - part.rect.h);
          cb.requestTick();
        });
        const partResizeHandle = attachResize(
          pg,
          () => ({ w: 44, h: 22 }),
          () => ({ w: part.rect.w, h: part.rect.h }),
          (w, h) => {
            part.rect.w = w;
            part.rect.h = h;
          },
          () => {
            const siblings = component.parts.filter((p) => p !== part).map((p) => p.rect);
            const growth = maxGrowth(part.rect, siblings, 14);
            return {
              w: Math.min(growth.w, component.rect.w - 16 - part.rect.x),
              h: Math.min(growth.h, component.rect.h - 16 - part.rect.y),
            };
          }
        );
        partEls.set(part.uid, { g: pg, resizeHandle: partResizeHandle });
      }
    }

    const label = el("text");
    label.setAttribute("class", "component-label");
    label.textContent = component.name;
    g.append(label);
    titled(g, component.name);

    const resizeHandle = attachResize(
      g,
      () => {
        if (!component.hasParts) return { w: 76, h: 40 };
        const maxRight = component.parts.reduce((m, p) => Math.max(m, p.rect.x + p.rect.w), 0);
        const maxBottom = component.parts.reduce((m, p) => Math.max(m, p.rect.y + p.rect.h), 0);
        return { w: Math.max(96, maxRight + 18), h: Math.max(64, maxBottom + 18) };
      },
      () => ({ w: component.rect.w, h: component.rect.h }),
      (w, h) => {
        component.rect.w = w;
        component.rect.h = h;
      },
      () => {
        const parentEnv = world.environments.find((e) => e.components.includes(component))!;
        const siblings = parentEnv.components.filter((c) => c !== component).map((c) => c.rect);
        const growth = maxGrowth(component.rect, siblings, 20);
        return {
          w: Math.min(growth.w, parentEnv.rect.x + parentEnv.rect.w - 12 - component.rect.x),
          h: Math.min(growth.h, parentEnv.rect.y + parentEnv.rect.h - 12 - component.rect.y),
        };
      }
    );

    componentGroups.set(component.name, { g, frame, interior, caption, resizeHandle, parts: partEls });
    componentLayer.appendChild(g);

    g.addEventListener("click", (ev) => {
      ev.stopPropagation();
      cb.onSelect({ type: "component", name: component.name });
    });
    g.addEventListener("dblclick", (ev) => {
      ev.stopPropagation();
      cb.onFocusRequest(component.rect);
    });
    attachDrag(g, (dx, dy) => {
      const parentEnv = world.environments.find((e) => e.components.includes(component))!;
      component.rect.x = clampAxis(component.rect.x + dx, parentEnv.rect.x + 12, parentEnv.rect.x + parentEnv.rect.w - component.rect.w - 12);
      component.rect.y = clampAxis(component.rect.y + dy, parentEnv.rect.y + 34, parentEnv.rect.y + parentEnv.rect.h - component.rect.h - 12);
      cb.requestTick();
    });
  }

  function interiorRect(component: ComponentNode): Rect {
    return { x: component.rect.x + 16, y: component.rect.y + 28, w: component.rect.w - 32, h: component.rect.h - 44 };
  }

  function clampAxis(v: number, lo: number, hi: number): number {
    return Math.min(Math.max(v, lo), Math.max(lo, hi));
  }

  function attachDrag(g: SVGGElement, onMove: (dx: number, dy: number) => void) {
    let start: { x: number; y: number } | null = null;
    let moved = false;
    g.addEventListener("pointerdown", (ev) => {
      if (ev.button !== 0) return;
      ev.stopPropagation();
      start = { x: ev.clientX, y: ev.clientY };
      moved = false;
      (ev.target as Element).setPointerCapture(ev.pointerId);
    });
    g.addEventListener("pointermove", (ev) => {
      if (!start) return;
      const scale = cb.getScale();
      const dx = (ev.clientX - start.x) / scale;
      const dy = (ev.clientY - start.y) / scale;
      if (Math.abs(dx) > 2 || Math.abs(dy) > 2) moved = true;
      if (!moved) return;
      start = { x: ev.clientX, y: ev.clientY };
      onMove(dx, dy);
    });
    const end = () => {
      start = null;
      if (moved) suppressNextClick(g);
      moved = false;
    };
    g.addEventListener("pointerup", end);
    g.addEventListener("pointercancel", end);
  }

  function suppressNextClick(g: SVGGElement) {
    const handler = (ev: Event) => {
      ev.stopPropagation();
      g.removeEventListener("click", handler, true);
    };
    g.addEventListener("click", handler, true);
  }

  /** A drag handle in the bottom-right corner of `parent`'s own local origin,
   * repositioned each tick to (rect.w, rect.h) by the caller. `getMaxSize` is
   * recomputed on every move so growing past a sibling stops right at it. */
  function attachResize(
    parent: SVGGElement,
    getMinSize: () => { w: number; h: number },
    getCurrentSize: () => { w: number; h: number },
    onResize: (w: number, h: number) => void,
    getMaxSize: () => { w: number; h: number } = () => ({ w: Infinity, h: Infinity })
  ): SVGGElement {
    const handle = el("g");
    handle.setAttribute("class", "resize-handle-group");
    const hit = el("rect");
    hit.setAttribute("class", "resize-handle");
    hit.setAttribute("x", "-9");
    hit.setAttribute("y", "-9");
    hit.setAttribute("width", "18");
    hit.setAttribute("height", "18");
    const glyph = el("path");
    glyph.setAttribute("class", "resize-handle-glyph");
    glyph.setAttribute("d", "M -6 1 L 1 -6 M -1 4 L 4 -1");
    handle.append(hit, glyph);
    parent.appendChild(handle);

    let start: { x: number; y: number; w: number; h: number } | null = null;
    handle.addEventListener("pointerdown", (ev) => {
      if (ev.button !== 0) return;
      ev.stopPropagation();
      const current = getCurrentSize();
      start = { x: ev.clientX, y: ev.clientY, w: current.w, h: current.h };
      (ev.target as Element).setPointerCapture(ev.pointerId);
    });
    handle.addEventListener("pointermove", (ev) => {
      if (!start) return;
      ev.stopPropagation();
      const scale = cb.getScale();
      const dx = (ev.clientX - start.x) / scale;
      const dy = (ev.clientY - start.y) / scale;
      const min = getMinSize();
      const max = getMaxSize();
      onResize(
        Math.min(max.w, Math.max(min.w, start.w + dx)),
        Math.min(max.h, Math.max(min.h, start.h + dy))
      );
      cb.requestTick();
    });
    const end = (ev: PointerEvent) => {
      ev.stopPropagation();
      if (start) suppressNextClick(parent);
      start = null;
    };
    handle.addEventListener("pointerup", end);
    handle.addEventListener("pointercancel", end);
    return handle;
  }

  // --- externals ------------------------------------------------------------
  for (const ext of world.externals) {
    const g = el("g");
    g.setAttribute("class", "external");
    g.dataset.name = ext.name;
    const diamond = el("polygon");
    diamond.setAttribute("class", "external-shape");
    const label = el("text");
    label.setAttribute("class", "external-label");
    label.textContent = ext.name;
    g.append(diamond, label);
    titled(g, ext.name);
    externalLayer.appendChild(g);
    externalGroups.set(ext.name, g);
    g.addEventListener("click", (ev) => {
      ev.stopPropagation();
      cb.onSelect({ type: "external", name: ext.name });
    });
    attachDrag(g, (dx, dy) => {
      ext.rect.x += dx;
      ext.rect.y += dy;
      cb.requestTick();
    });
  }

  // --- edges ------------------------------------------------------------------
  for (const edge of world.edges) {
    const path = el("path");
    path.setAttribute("class", `edge edge-${edge.scope}` + (edge.crossEnvironment ? " cross-env" : ""));
    (edge.scope === "part-internal" ? internalEdgeLayer : edgeLayer).appendChild(path);

    const entry: EdgeLineEntry = {
      path,
      label: null,
      labelBg: null,
      edge,
      labelOffset: null,
      lastAnchor: { x: 0, y: 0 },
      lane: { side: 1, rank: 0, along: 1 },
      touches: touchKeys(edge),
      labelSize: null,
      labelKey: null,
    };
    edgeLines.set(edge.id, entry);

    if (edge.label && edge.scope !== "part-internal") {
      const labelGroup = el("g");
      labelGroup.setAttribute("class", "edge-label-group");
      const labelBg = el("rect");
      labelBg.setAttribute("class", "edge-label-bg");
      const label = el("text");
      label.setAttribute("class", "edge-label");
      label.textContent = edge.label;
      labelGroup.append(labelBg, label);
      edgeLabelLayer.appendChild(labelGroup);
      entry.label = label;
      entry.labelBg = labelBg;

      attachDrag(labelGroup, (dx, dy) => {
        if (!entry.labelOffset) {
          entry.labelOffset = {
            dx: Number(label.getAttribute("x") ?? "0") - entry.lastAnchor.x,
            dy: Number(label.getAttribute("y") ?? "0") - entry.lastAnchor.y,
          };
        }
        entry.labelOffset.dx += dx;
        entry.labelOffset.dy += dy;
        const mag = Math.hypot(entry.labelOffset.dx, entry.labelOffset.dy);
        if (mag > MAX_LABEL_DRAG) {
          entry.labelOffset.dx *= MAX_LABEL_DRAG / mag;
          entry.labelOffset.dy *= MAX_LABEL_DRAG / mag;
        }
        cb.requestTick();
      });
    }
  }

  // The bow's perpendicular flips with edge direction, so sides are assigned
  // in the frame of the pair's first edge: a lone edge keeps side +1, and an
  // A→B / B→A pair still lands on opposite sides.
  function touchKeys(edge: WorldEdge): Set<string> {
    const keys = new Set<string>();
    for (const side of [edge.from, edge.to]) {
      keys.add(keyOf(side.ref));
      if (side.anchor.kind === "part") keys.add(`part:${side.anchor.uid}`);
      const owner = side.ref.type === "part" ? side.ref.owner : side.ref.type === "component" ? side.ref.name : null;
      if (owner === null) continue;
      keys.add(keyOf({ type: "component", name: owner }));
      keys.add(keyOf({ type: "environment", name: world.componentsByName.get(owner)!.environment }));
    }
    return keys;
  }

  const parallelGroups = new Map<string, EdgeLineEntry[]>();
  for (const entry of edgeLines.values()) {
    const ends = [keyOf(entry.edge.from.ref), keyOf(entry.edge.to.ref)].sort();
    const pair = ends.join("\u0000");
    const group = parallelGroups.get(pair);
    if (group) group.push(entry);
    else parallelGroups.set(pair, [entry]);
  }
  for (const group of parallelGroups.values()) {
    const canonicalFrom = keyOf(group[0]!.edge.from.ref);
    group.forEach((entry, k) => {
      const alongCanonical = keyOf(entry.edge.from.ref) === canonicalFrom ? 1 : -1;
      entry.lane = {
        side: (k % 2 === 0 ? alongCanonical : -alongCanonical) as 1 | -1,
        rank: Math.floor(k / 2),
        along: alongCanonical,
      };
    });
  }
  const routeGroups = [...parallelGroups.values()].map((entries) => ({ entries, shiftKey: null as number | null }));

  function routeObstacles(): RouteObstacle[] {
    const obstacles: RouteObstacle[] = [];
    for (const env of world.environments) {
      obstacles.push({ key: keyOf({ type: "environment", name: env.name }), rect: env.rect, weight: 1 });
      for (const component of env.components) {
        obstacles.push({ key: keyOf({ type: "component", name: component.name }), rect: component.rect, weight: 10 });
        const reveal = revealFactors.get(component.name) ?? 0;
        if (reveal === 0) continue;
        for (const part of component.parts) {
          obstacles.push({ key: `part:${part.uid}`, rect: worldRectOfPart(component.rect, part), weight: 6 * reveal });
        }
      }
    }
    for (const ext of world.externals) obstacles.push({ key: keyOf({ type: "external", name: ext.name }), rect: ext.rect, weight: 10 });
    return obstacles;
  }

  // A hidden SVG measures as zero, so only a real measurement is cached.
  // The padded background box, and how far below its top the text baseline sits.
  function labelBox(entry: EdgeLineEntry, invZoom: number): { w: number; h: number; baseline: number } {
    if (!entry.labelSize && entry.label) {
      const bbox = entry.label.getBBox();
      if (bbox.width > 0) {
        const baseline = Number(entry.label.getAttribute("y") ?? "0");
        entry.labelSize = { w: bbox.width / invZoom, h: bbox.height / invZoom, ascent: (baseline - bbox.y) / invZoom };
      }
    }
    const size = entry.labelSize ?? { w: 0, h: 0, ascent: 0 };
    return { w: size.w * invZoom + 8, h: size.h * invZoom + 4, baseline: size.ascent * invZoom + 2 };
  }

  function labelObstacleNodes(invZoom: number): Rect[] {
    const nodes: Rect[] = [];
    for (const env of world.environments) {
      const titleW = measureWidth(env.name ?? "no environment", ENVIRONMENT_FONT) * invZoom + 24;
      nodes.push({ x: env.rect.x, y: env.rect.y, w: Math.min(env.rect.w, titleW), h: 32 });
      for (const component of env.components) nodes.push(component.rect);
    }
    for (const ext of world.externals) {
      const nameW = measureWidth(ext.name, MONO_FONT) * invZoom;
      nodes.push(ext.rect, { x: ext.rect.x + ext.rect.w / 2 - nameW / 2, y: ext.rect.y + ext.rect.h, w: nameW, h: 20 });
    }
    return nodes;
  }

  function tick() {
    const scale = cb.getScale();
    if (baseScale === null) baseScale = scale;
    const zoomRatio = scale / baseScale;
    // Text, strokes, and arrowheads are sized in world units, so zooming in
    // deep would otherwise blow them up past readable size. Once you zoom
    // past the initial fit, shrink them in world units by the same factor
    // the camera is magnifying, so their on-screen size stays where it was
    // at the initial view instead of growing without bound. That correction
    // is only meant to hold through the part-reveal transition, though — floor
    // it at REVEAL_END_MULT so on-screen size grows again (bounded by the
    // zoom's own scaleExtent) once parts are fully revealed, instead of
    // shrinking toward unreadable all the way to max zoom.
    const invZoom = Math.max(1 / REVEAL_END_MULT, Math.min(1, 1 / zoomRatio));
    root.style.setProperty("--inv-zoom", String(invZoom));
    for (const env of world.environments) {
      for (const component of env.components) {
        // A part matching the search must be visible regardless of zoom —
        // otherwise it stays opacity-0 until the user happens to zoom in
        // deep enough to trigger the ordinary reveal.
        const searchMatchesPart = searchQuery !== "" && component.parts.some((p) => labelMatchesSearch(p.name));
        const reveal = component.hasParts
          ? Math.max(smoothstep(REVEAL_START_MULT, REVEAL_END_MULT, zoomRatio), searchMatchesPart ? 1 : 0)
          : 0;
        revealFactors.set(component.name, reveal);
      }
    }

    for (const env of world.environments) {
      for (const component of env.components) {
        const entry = componentGroups.get(component.name)!;
        const reveal = revealFactors.get(component.name) ?? 0;
        entry.g.setAttribute("transform", `translate(${component.rect.x},${component.rect.y})`);
        entry.frame.setAttribute("width", String(component.rect.w));
        entry.frame.setAttribute("height", String(component.rect.h));
        const label = entry.g.querySelector<SVGTextElement>(".component-label")!;
        label.setAttribute("x", "16");
        label.setAttribute("y", component.hasParts ? "19" : String(component.rect.h / 2 + 4));
        setFittedText(label, component.name, component.rect.w - 32, invZoom, COMPONENT_FONT);

        if (entry.interior) {
          const inset = { x: 16, y: 28, w: component.rect.w - 32, h: component.rect.h - 44 };
          entry.interior.setAttribute("x", String(inset.x));
          entry.interior.setAttribute("y", String(inset.y));
          entry.interior.setAttribute("width", String(Math.max(inset.w, 0)));
          entry.interior.setAttribute("height", String(Math.max(inset.h, 0)));
          entry.interior.setAttribute("opacity", String(reveal * 0.92));
        }
        if (entry.caption) {
          entry.caption.setAttribute("x", "16");
          entry.caption.setAttribute("y", "40");
          entry.caption.setAttribute("opacity", String(1 - smoothstep(0, 0.45, reveal)));
        }
        entry.resizeHandle.setAttribute(
          "transform",
          `translate(${component.rect.w},${component.rect.h}) scale(${invZoom})`
        );
        for (const part of component.parts) {
          const { g: pg, resizeHandle: partHandle } = entry.parts.get(part.uid)!;
          pg.setAttribute("transform", `translate(${part.rect.x},${part.rect.y})`);
          pg.setAttribute("opacity", String(reveal));
          pg.style.pointerEvents = reveal > 0.6 ? "auto" : "none";
          const prect = pg.querySelector<SVGRectElement>(".part-frame")!;
          prect.setAttribute("width", String(part.rect.w));
          prect.setAttribute("height", String(part.rect.h));
          const ptext = pg.querySelector<SVGTextElement>(".part-label")!;
          ptext.setAttribute("x", String(part.rect.w / 2));
          ptext.setAttribute("y", String(part.rect.h / 2 + 4));
          setFittedText(ptext, part.name, part.rect.w - 12, invZoom, MONO_FONT);
          partHandle.setAttribute("transform", `translate(${part.rect.w},${part.rect.h}) scale(${invZoom})`);
        }
      }
    }

    for (const env of world.environments) {
      const entry = environmentGroups.get(env.name)!;
      entry.g.setAttribute("transform", `translate(${env.rect.x},${env.rect.y})`);
      entry.frame.setAttribute("width", String(env.rect.w));
      entry.frame.setAttribute("height", String(env.rect.h));
      setFittedText(entry.label, env.name ?? "no environment", env.rect.w - 40, invZoom, ENVIRONMENT_FONT);
      entry.resizeHandle.setAttribute("transform", `translate(${env.rect.w},${env.rect.h}) scale(${invZoom})`);
    }

    for (const ext of world.externals) {
      const g = externalGroups.get(ext.name)!;
      const c = rectCenter(ext.rect);
      const half = ext.rect.w / 2;
      const points = [
        [c.x, c.y - half],
        [c.x + half, c.y],
        [c.x, c.y + half],
        [c.x - half, c.y],
      ]
        .map((p) => p.join(","))
        .join(" ");
      g.querySelector("polygon")!.setAttribute("points", points);
      const label = g.querySelector("text")!;
      label.setAttribute("x", String(c.x));
      label.setAttribute("y", String(ext.rect.y + ext.rect.h + 16));
    }

    const curves = new Map<EdgeLineEntry, Curve>();
    const bowSides = new Map<EdgeLineEntry, 1 | -1>();
    const edgeLinesDrawn: Polyline[] = [];
    const obstaclesForRoutes = routeObstacles();
    for (const group of routeGroups) {
      const members: RouteMember[] = group.entries.map((entry) => {
        const { a, b } = edgeEndpoints(entry.edge.from, entry.edge.to, world, revealFactors);
        const laneBow = entry.lane.side * (Math.min(40, Math.hypot(b.x - a.x, b.y - a.y) * 0.2) + entry.lane.rank * PARALLEL_SPACING);
        return { a, b, laneBow, along: entry.lane.along, touches: entry.touches };
      });
      group.shiftKey = chooseShift(members, obstaclesForRoutes, group.shiftKey);
      group.entries.forEach((entry, i) => {
        const m = members[i]!;
        const bow = memberBow(m, shiftOf(group.shiftKey!));
        curves.set(entry, bowCurve(m.a, m.b, bow));
        bowSides.set(entry, bow < 0 ? -1 : bow > 0 ? 1 : entry.lane.side);
      });
    }
    for (const entry of edgeLines.values()) {
      const { path, label, labelBg, edge } = entry;
      const opacity =
        edge.scope === "part-internal"
          ? revealFactors.get((edge.from.ref as { type: "part"; owner: string }).owner) ?? 0
          : 1;
      path.setAttribute("opacity", String(opacity));
      label?.setAttribute("opacity", String(opacity));
      labelBg?.setAttribute("opacity", String(opacity * 0.85));
      const curve = curves.get(entry)!;
      const { a, b } = curve;
      path.setAttribute("d", `M ${a.x} ${a.y} Q ${curve.c.x} ${curve.c.y} ${b.x} ${b.y}`);
      entry.lastAnchor = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
      if (opacity > 0) edgeLinesDrawn.push(curvePolyline(curve));
    }

    // Labels may still land on a node or across an edge when nothing near
    // their own edge is clear — staying readable as "this edge's label" wins.
    const obstacles = { labels: [] as Rect[], nodes: labelObstacleNodes(invZoom), edges: edgeLinesDrawn };
    const labelled = [...edgeLines.values()].filter((entry) => entry.label && entry.labelBg);
    const chordLength = (entry: EdgeLineEntry) => {
      const { a, b } = curves.get(entry)!;
      return Math.hypot(b.x - a.x, b.y - a.y);
    };
    // User-placed labels are fixed, so they go first as obstacles; then the
    // shortest edges, which have the least room to shift their label along.
    labelled.sort((p, q) => Number(q.labelOffset !== null) - Number(p.labelOffset !== null) || chordLength(p) - chordLength(q));
    for (const entry of labelled) {
      const label = entry.label!;
      const labelBg = entry.labelBg!;
      const box = labelBox(entry, invZoom);
      let rect: Rect;
      if (entry.labelOffset) {
        // Already clamped to MAX_LABEL_DRAG at drag time.
        const lx = entry.lastAnchor.x + entry.labelOffset.dx;
        const ly = entry.lastAnchor.y + entry.labelOffset.dy;
        rect = { x: lx - box.w / 2, y: ly - box.baseline, w: box.w, h: box.h };
      } else {
        const placement = placeLabel(curves.get(entry)!, bowSides.get(entry)!, box, invZoom, obstacles, entry.labelKey);
        entry.labelKey = placement.key;
        rect = placement.rect;
      }
      label.setAttribute("x", String(rect.x + rect.w / 2));
      label.setAttribute("y", String(rect.y + box.baseline));
      labelBg.setAttribute("x", String(rect.x));
      labelBg.setAttribute("y", String(rect.y));
      labelBg.setAttribute("width", String(rect.w));
      labelBg.setAttribute("height", String(rect.h));
      obstacles.labels.push(rect);
    }

    applyDimming();
  }

  function currentHighlightSet(): Set<string> | null {
    if (selection) {
      const keys = new Set<string>([keyOf(selection)]);
      // A selected part's own component frame must stay lit, or the group
      // opacity dims everything inside it — including the selected part.
      if (selection.type === "part") keys.add(keyOf({ type: "component", name: selection.owner }));
      for (const edge of world.edges) {
        if (edgeSideMatches(selection, edge.from)) keys.add(keyOf(edge.to.ref));
        if (edgeSideMatches(selection, edge.to)) keys.add(keyOf(edge.from.ref));
      }
      if (selection.type === "environment") {
        for (const env of world.environments) {
          if (env.name === selection.name) for (const c of env.components) keys.add(keyOf({ type: "component", name: c.name }));
        }
      }
      return keys;
    }
    if (searchQuery) {
      const keys = new Set<string>();
      for (const c of world.componentsByName.values()) {
        const matchedParts = c.parts.filter((p) => p.name.toLowerCase().includes(searchQuery));
        // A matched part's own component frame must stay lit too, or the
        // group opacity dims everything inside it — including the match.
        if (c.name.toLowerCase().includes(searchQuery) || matchedParts.length > 0) {
          keys.add(keyOf({ type: "component", name: c.name }));
        }
        for (const p of matchedParts) keys.add(keyOf({ type: "part", owner: p.owner, uid: p.uid }));
      }
      for (const e of world.externalsByName.values()) if (e.name.toLowerCase().includes(searchQuery)) keys.add(keyOf({ type: "external", name: e.name }));
      return keys;
    }
    return null;
  }

  function keyOf(ref: SelectableRef): string {
    if (ref.type === "part") return `part:${ref.uid}`;
    if (ref.type === "environment") return `environment:${ref.name}`;
    return `${ref.type}:${ref.name}`;
  }

  function applyDimming() {
    const set = currentHighlightSet();
    for (const [name, entry] of componentGroups) {
      entry.g.classList.toggle("dimmed", set !== null && !set.has(keyOf({ type: "component", name })));
      for (const [uid, { g: pg }] of entry.parts) {
        pg.classList.toggle("dimmed", set !== null && !set.has(`part:${uid}`) && !set.has(keyOf({ type: "component", name })));
        pg.classList.toggle("search-match", searchQuery !== "" && labelMatchesSearch(pg.dataset.name!));
        pg.classList.toggle("selected", selection?.type === "part" && selection.uid === uid);
      }
      entry.g.classList.toggle("search-match", searchQuery !== "" && labelMatchesSearch(name));
      entry.g.classList.toggle("selected", selection?.type === "component" && selection.name === name);
    }
    for (const [name, g] of externalGroups) {
      g.classList.toggle("dimmed", set !== null && !set.has(keyOf({ type: "external", name })));
      g.classList.toggle("search-match", searchQuery !== "" && labelMatchesSearch(name));
      g.classList.toggle("selected", selection?.type === "external" && selection.name === name);
    }
    for (const { path, label, labelBg, edge } of edgeLines.values()) {
      const relevant = set === null || (set.has(keyOf(edge.from.ref)) && set.has(keyOf(edge.to.ref)));
      // A Component's own internal Part-to-Part edges are part of what
      // selecting it is meant to explain, not traffic to dim away.
      const ownedByComponent =
        edge.scope === "part-internal" &&
        selection?.type === "component" &&
        (edge.from.ref as { type: "part"; owner: string }).owner === selection.name;
      const dim = selection
        ? !(edgeSideMatches(selection, edge.from) || edgeSideMatches(selection, edge.to) || ownedByComponent)
        : !relevant;
      path.classList.toggle("dimmed", dim);
      label?.classList.toggle("dimmed", dim);
      labelBg?.classList.toggle("dimmed", dim);
    }
  }

  function edgesTouching(ref: SelectableRef): WorldEdge[] {
    return world.edges.filter((e) => edgeSideMatches(ref, e.from) || edgeSideMatches(ref, e.to));
  }

  const handles: RenderHandles = {
    tick,
    setSelection(ref) {
      selection = ref;
      searchQuery = "";
      // Search feeds into tick()'s part-reveal opacity, not just
      // applyDimming()'s classes — re-run the whole frame so the effect shows
      // immediately, not on the next pan/zoom.
      tick();
    },
    setSearch(query) {
      selection = null;
      searchQuery = query.toLowerCase();
      tick();
    },
    edgesTouching,
  };

  tick();
  return handles;
}

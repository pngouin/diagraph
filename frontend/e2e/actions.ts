import type { Page } from "@playwright/test";
import fc from "fast-check";

export type NodeKind = "component" | "part" | "external" | "environment";

/** Screen positions are per-mille of the canvas so shrunk cases stay readable. */
export type Action =
  | { kind: "wheel"; x: number; y: number; deltaY: number }
  | { kind: "zoomAt"; node: NodeKind; index: number; deltaY: number }
  | { kind: "pan"; x: number; y: number; dx: number; dy: number }
  | { kind: "dblclick"; node: NodeKind; index: number }
  | { kind: "drag"; node: NodeKind; index: number; dx: number; dy: number }
  | { kind: "resize"; node: Exclude<NodeKind, "external">; index: number; dx: number; dy: number }
  | { kind: "search"; index: number; start: number; length: number }
  | { kind: "clearSearch" }
  | { kind: "select"; node: NodeKind; index: number }
  | { kind: "clickEmpty"; x: number; y: number };

/** A node the user just zoomed at, which must not stay faded. */
export type Focus = { type: "component" | "external"; name: string; via: "dblclick" | "wheel" } | null;

const permille = fc.integer({ min: 0, max: 1000 });
const delta = fc.integer({ min: -400, max: 400 });
const index = fc.nat({ max: 63 });
const node = fc.constantFrom<NodeKind>("component", "component", "part", "external", "environment");

export const actionArb: fc.Arbitrary<Action> = fc.oneof(
  { weight: 4, arbitrary: fc.record({ kind: fc.constant("wheel" as const), x: permille, y: permille, deltaY: fc.integer({ min: -1500, max: 1500 }) }) },
  { weight: 3, arbitrary: fc.record({ kind: fc.constant("zoomAt" as const), node, index, deltaY: fc.integer({ min: -1500, max: 1500 }) }) },
  { weight: 1, arbitrary: fc.record({ kind: fc.constant("pan" as const), x: permille, y: permille, dx: delta, dy: delta }) },
  { weight: 3, arbitrary: fc.record({ kind: fc.constant("dblclick" as const), node, index }) },
  { weight: 2, arbitrary: fc.record({ kind: fc.constant("drag" as const), node, index, dx: delta, dy: delta }) },
  {
    weight: 2,
    arbitrary: fc.record({
      kind: fc.constant("resize" as const),
      node: fc.constantFrom<Exclude<NodeKind, "external">>("component", "part", "environment"),
      index,
      dx: delta,
      dy: delta,
    }),
  },
  { weight: 1, arbitrary: fc.record({ kind: fc.constant("search" as const), index, start: index, length: fc.integer({ min: 1, max: 12 }) }) },
  { weight: 1, arbitrary: fc.constant({ kind: "clearSearch" as const }) },
  { weight: 1, arbitrary: fc.record({ kind: fc.constant("select" as const), node, index }) },
  { weight: 1, arbitrary: fc.record({ kind: fc.constant("clickEmpty" as const), x: permille, y: permille }) }
);

interface Target {
  x: number;
  y: number;
  hit: Focus;
}

/** Picks the `index`-th (mod count) on-screen node of a kind and a client point
 * on it — its resize handle when `handle` — plus what that point actually hits. */
async function locate(page: Page, kind: NodeKind, index: number, handle: boolean): Promise<Target | null> {
  return page.evaluate(
    ({ kind, index, handle }) => {
      const canvas = document.getElementById("canvas")!.getBoundingClientRect();
      const shapeOf: Record<string, string> = {
        component: ":scope > .component-frame",
        part: ":scope > .part-frame",
        external: ":scope > polygon",
        environment: ":scope > .environment-frame",
      };
      const candidates: { x: number; y: number }[] = [];
      for (const g of document.querySelectorAll<SVGGElement>(`.${kind}`)) {
        if (kind === "part" && g.style.pointerEvents === "none") continue;
        const shape = g.querySelector(handle ? ":scope > .resize-handle-group > .resize-handle" : shapeOf[kind]!);
        if (!shape) continue;
        const r = shape.getBoundingClientRect();
        const x0 = Math.max(r.left, canvas.left);
        const y0 = Math.max(r.top, canvas.top);
        const x1 = Math.min(r.right, canvas.right);
        const y1 = Math.min(r.bottom, canvas.bottom);
        if (x1 - x0 < 2 || y1 - y0 < 2) continue;
        const y = kind === "environment" && !handle ? y0 + Math.min(10, (y1 - y0) / 4) : (y0 + y1) / 2;
        candidates.push({ x: (x0 + x1) / 2, y });
      }
      if (candidates.length === 0) return null;
      const p = candidates[index % candidates.length]!;
      const hitEl = document.elementFromPoint(p.x, p.y)?.closest(".component, .part, .external, .environment");
      const owner = hitEl?.classList.contains("part") ? hitEl.closest(".component") : hitEl;
      const hit = owner?.classList.contains("component")
        ? { type: "component" as const, name: (owner as SVGGElement).dataset.name!, via: "dblclick" as const }
        : null;
      return { ...p, hit };
    },
    { kind, index, handle }
  );
}

async function nodeAt(page: Page, x: number, y: number): Promise<Focus> {
  return page.evaluate(
    ({ x, y }) => {
      const hitEl = document.elementFromPoint(x, y)?.closest(".component, .part, .external");
      const owner = hitEl?.classList.contains("part") ? hitEl.closest(".component") : hitEl;
      if (!owner) return null;
      const type = owner.classList.contains("external") ? ("external" as const) : ("component" as const);
      return { type, name: (owner as SVGGElement).dataset.name!, via: "wheel" as const };
    },
    { x, y }
  );
}

async function canvasPoint(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const r = await page.locator("#canvas").boundingBox();
  if (!r) throw new Error("canvas not laid out");
  return { x: r.x + (r.width * x) / 1000, y: r.y + (r.height * y) / 1000 };
}

async function dragFrom(page: Page, x: number, y: number, dx: number, dy: number) {
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx / 2, y + dy / 2, { steps: 3 });
  await page.mouse.move(x + dx, y + dy, { steps: 3 });
  await page.mouse.up();
}

/** Performs one action through real input events; returns the component a
 * double-click should have focused, if any. */
export async function perform(page: Page, action: Action): Promise<Focus> {
  switch (action.kind) {
    case "wheel": {
      const p = await canvasPoint(page, action.x, action.y);
      const hit = await nodeAt(page, p.x, p.y);
      await page.mouse.move(p.x, p.y);
      await page.mouse.wheel(0, action.deltaY);
      return hit;
    }
    case "zoomAt": {
      const t = await locate(page, action.node, action.index, false);
      if (!t) return null;
      const hit = await nodeAt(page, t.x, t.y);
      await page.mouse.move(t.x, t.y);
      await page.mouse.wheel(0, action.deltaY);
      return hit;
    }
    case "pan": {
      const p = await canvasPoint(page, action.x, action.y);
      await dragFrom(page, p.x, p.y, action.dx, action.dy);
      return null;
    }
    case "dblclick": {
      const t = await locate(page, action.node, action.index, false);
      if (!t) return null;
      await page.mouse.dblclick(t.x, t.y);
      return t.hit;
    }
    case "drag": {
      const t = await locate(page, action.node, action.index, false);
      if (t) await dragFrom(page, t.x, t.y, action.dx, action.dy);
      return null;
    }
    case "resize": {
      const t = await locate(page, action.node, action.index, true);
      if (t) await dragFrom(page, t.x, t.y, action.dx, action.dy);
      return null;
    }
    case "search": {
      const names = await page.evaluate(() =>
        [...document.querySelectorAll<SVGGElement>(".component, .part, .external")].map((g) => g.dataset.name ?? "")
      );
      const chars = [...(names[action.index % names.length] ?? "")];
      const start = chars.length === 0 ? 0 : action.start % chars.length;
      await page.fill("#search", chars.slice(start, start + action.length).join(""));
      return null;
    }
    case "clearSearch":
      await page.fill("#search", "");
      return null;
    case "select": {
      const t = await locate(page, action.node, action.index, false);
      if (t) await page.mouse.click(t.x, t.y);
      return null;
    }
    case "clickEmpty": {
      const p = await canvasPoint(page, action.x, action.y);
      await page.mouse.click(p.x, p.y);
      return null;
    }
  }
}

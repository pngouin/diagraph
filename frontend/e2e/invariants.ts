import type { Focus } from "./actions";

export interface InvariantArgs {
  focus: Focus;
  initial: boolean;
}

/** Runs inside the page (serialized by Playwright), so it must stay self-contained. */
export function pageInvariants({ focus, initial }: InvariantArgs): string[] {
  const out: string[] = [];
  const TOL = 1;
  const svg = document.getElementById("canvas")!;
  const fmt = (n: number) => n.toFixed(1);
  const num = (el: Element, attr: string) => parseFloat(el.getAttribute(attr) ?? "NaN");
  const translate = (el: Element) => {
    const m = /translate\(([^,]+),([^)]+)\)/.exec(el.getAttribute("transform") ?? "");
    return m ? { x: parseFloat(m[1]!), y: parseFloat(m[2]!) } : { x: NaN, y: NaN };
  };
  const inside = (inner: { x: number; y: number; w: number; h: number }, outer: typeof inner, tol: number) =>
    inner.x >= outer.x - tol &&
    inner.y >= outer.y - tol &&
    inner.x + inner.w <= outer.x + outer.w + tol &&
    inner.y + inner.h <= outer.y + outer.h + tol;
  const box = (r: { x: number; y: number; w: number; h: number }) =>
    `[${fmt(r.x)},${fmt(r.y)} ${fmt(r.w)}x${fmt(r.h)}]`;

  // Rule 4: no NaN / Infinity in geometry.
  for (const el of document.querySelectorAll("#viewport, #viewport *")) {
    for (const attr of ["transform", "d", "x", "y", "width", "height", "points", "opacity"]) {
      const v = el.getAttribute(attr);
      if (v && /NaN|Infinity/.test(v)) out.push(`rule4: <${el.tagName} class="${el.getAttribute("class")}"> ${attr}="${v}"`);
    }
  }

  // Rule 1: labels fit their box.
  // getBBox drifts once text renders below a few screen pixels, and nobody
  // reads a label that small anyway.
  const scale = parseFloat(/scale\(([^)]+)\)/.exec(document.getElementById("viewport")!.getAttribute("transform") ?? "")?.[1] ?? "1");
  const labelFits = (what: string, text: SVGTextElement | null, w: number, h: number) => {
    if (!text) return;
    if (parseFloat(getComputedStyle(text).fontSize) * scale < 4) return;
    const b = text.getBBox();
    if (b.width === 0) return;
    if (b.x < -TOL || b.x + b.width > w + TOL || b.y < -TOL || b.y + b.height > h + TOL) {
      out.push(`rule1: ${what} label ${JSON.stringify(text.textContent)} bbox ${box({ x: b.x, y: b.y, w: b.width, h: b.height })} outside box ${fmt(w)}x${fmt(h)}`);
    }
  };
  for (const g of document.querySelectorAll<SVGGElement>(".component")) {
    const frame = g.querySelector(":scope > .component-frame")!;
    labelFits(`component ${JSON.stringify(g.dataset.name)}`, g.querySelector(":scope > .component-label"), num(frame, "width"), num(frame, "height"));
  }
  for (const g of document.querySelectorAll<SVGGElement>(".part")) {
    const frame = g.querySelector(":scope > .part-frame")!;
    labelFits(`part ${JSON.stringify(g.dataset.name)}`, g.querySelector(":scope > .part-label"), num(frame, "width"), num(frame, "height"));
  }
  for (const g of document.querySelectorAll<SVGGElement>(".environment")) {
    const frame = g.querySelector(":scope > .environment-frame")!;
    labelFits(`environment ${JSON.stringify(g.dataset.name ?? null)}`, g.querySelector(":scope > .environment-label"), num(frame, "width"), num(frame, "height"));
  }

  // Rule 3: parts inside their component's interior, components inside their environment.
  const envRects = new Map<string | null, { x: number; y: number; w: number; h: number }>();
  for (const g of document.querySelectorAll<SVGGElement>(".environment")) {
    const t = translate(g);
    const frame = g.querySelector(":scope > .environment-frame")!;
    envRects.set(g.dataset.name ?? null, { x: t.x, y: t.y, w: num(frame, "width"), h: num(frame, "height") });
  }
  const componentRects: { name: string; env: string | null; rect: { x: number; y: number; w: number; h: number } }[] = [];
  for (const g of document.querySelectorAll<SVGGElement>(".component")) {
    const t = translate(g);
    const frame = g.querySelector(":scope > .component-frame")!;
    const rect = { x: t.x, y: t.y, w: num(frame, "width"), h: num(frame, "height") };
    const env = g.dataset.environment ?? null;
    componentRects.push({ name: g.dataset.name!, env, rect });
    const envRect = envRects.get(env);
    if (envRect && !inside(rect, envRect, 0.5)) {
      out.push(`rule3: component ${JSON.stringify(g.dataset.name)} ${box(rect)} outside environment ${JSON.stringify(env)} ${box(envRect)}`);
    }
    const interior = g.querySelector(":scope > .component-interior");
    if (!interior) continue;
    const inner = { x: num(interior, "x"), y: num(interior, "y"), w: num(interior, "width"), h: num(interior, "height") };
    for (const pg of g.querySelectorAll<SVGGElement>(":scope > .part")) {
      const pt = translate(pg);
      const pframe = pg.querySelector(":scope > .part-frame")!;
      const prect = { x: pt.x, y: pt.y, w: num(pframe, "width"), h: num(pframe, "height") };
      if (!inside(prect, inner, 0.5)) {
        out.push(`rule3: part ${JSON.stringify(pg.dataset.name)} ${box(prect)} outside interior of ${JSON.stringify(g.dataset.name)} ${box(inner)}`);
      }
    }
  }

  // Rule 7: no overlapping components within an environment, at initial layout.
  if (initial) {
    for (let i = 0; i < componentRects.length; i++) {
      for (let j = i + 1; j < componentRects.length; j++) {
        const a = componentRects[i]!;
        const b = componentRects[j]!;
        if (a.env !== b.env) continue;
        const overlap =
          a.rect.x < b.rect.x + b.rect.w - 0.5 && a.rect.x + a.rect.w > b.rect.x + 0.5 &&
          a.rect.y < b.rect.y + b.rect.h - 0.5 && a.rect.y + a.rect.h > b.rect.y + 0.5;
        if (overlap) out.push(`rule7: components ${JSON.stringify(a.name)} ${box(a.rect)} and ${JSON.stringify(b.name)} ${box(b.rect)} overlap`);
      }
    }
  }

  // Rule 2: what the user zoomed into is at full opacity. The `opacity`
  // attribute carries the ambient fade; the `.dimmed` class is selection/search
  // highlighting, which is only a bug when nothing is selected or searched.
  const noHighlight =
    (document.getElementById("search") as HTMLInputElement).value.trim() === "" &&
    document.querySelector("#inspector-body .inspector-empty") !== null;
  const fade = (el: Element) => {
    let o = 1;
    for (let n: Element | null = el; n && n !== svg; n = n.parentElement) o *= parseFloat(n.getAttribute("opacity") ?? "1");
    return o;
  };
  const checkFocused = (g: SVGGElement, why: string) => {
    const label = `${g.classList.contains("external") ? "external" : "component"} ${JSON.stringify(g.dataset.name)}`;
    const o = fade(g);
    if (o < 0.99) out.push(`rule2: ${label} (${why}) faded to opacity ${o.toFixed(2)}`);
    if (noHighlight && g.closest(".dimmed")) out.push(`rule2: ${label} (${why}) dimmed with no selection or search`);
  };
  const canvas = svg.getBoundingClientRect();
  const coverage = (g: SVGGElement) => {
    const r = g.querySelector(":scope > .component-frame, :scope > polygon")!.getBoundingClientRect();
    const w = Math.min(r.right, canvas.right) - Math.max(r.left, canvas.left);
    const h = Math.min(r.bottom, canvas.bottom) - Math.max(r.top, canvas.top);
    return w > 0 && h > 0 ? (w * h) / (canvas.width * canvas.height) : 0;
  };
  const nodes = [...document.querySelectorAll<SVGGElement>(".component, .external")];
  if (focus) {
    const g = nodes.find((n) => n.classList.contains(focus.type) && n.dataset.name === focus.name);
    // A wheel zoom only says what the user is looking at once the node fills a
    // real share of the screen; 8x max zoom caps a small component near 25%.
    if (g && focus.via === "dblclick") checkFocused(g, "double-clicked");
    if (g && focus.via === "wheel" && coverage(g) >= 0.15) checkFocused(g, `wheel-zoomed at, covers ${Math.round(coverage(g) * 100)}%`);
  }
  const covering = nodes.filter((g) => coverage(g) >= 0.5);
  if (covering.length === 1) checkFocused(covering[0]!, "covers ≥50% of the canvas");

  return out;
}

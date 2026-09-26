import { easeCubicInOut } from "d3-ease";
import { select } from "d3-selection";
import "d3-transition";
import { zoom as d3zoom, zoomIdentity, type ZoomBehavior, type ZoomTransform } from "d3-zoom";
import type { Rect } from "./geometry";

export interface ZoomController {
  getScale(): number;
  fitTo(rect: Rect, opts?: { animate?: boolean; padding?: number }): void;
  currentTransform(): ZoomTransform;
}

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

export function attachZoom(svg: SVGSVGElement, viewport: SVGGElement, onTransform: (t: ZoomTransform) => void): ZoomController {
  const selection = select(svg);
  let current: ZoomTransform = zoomIdentity;

  const behavior: ZoomBehavior<SVGSVGElement, unknown> = d3zoom<SVGSVGElement, unknown>()
    .scaleExtent([0.05, 8])
    .filter((event: MouseEvent | WheelEvent) => {
      if (event.type === "dblclick") return false;
      if (event.button === 2) return true;
      if (event.button) return false;
      if (event.ctrlKey && event.type !== "wheel") return false;
      const target = event.target as Element | null;
      if (event.type !== "wheel" && target?.closest?.(".component, .part, .external, .environment, .edge-label-group")) {
        return false;
      }
      return true;
    })
    .on("zoom", (event) => {
      current = event.transform;
      viewport.setAttribute("transform", current.toString());
      onTransform(current);
    });

  selection.call(behavior);
  svg.addEventListener("contextmenu", (ev) => ev.preventDefault());

  function fitTo(rect: Rect, opts: { animate?: boolean; padding?: number } = {}) {
    const padding = opts.padding ?? 56;
    const bounds = svg.getBoundingClientRect();
    const w = bounds.width || svg.clientWidth || 1;
    const h = bounds.height || svg.clientHeight || 1;
    const scale = Math.min(8, Math.max(0.05, Math.min((w - padding * 2) / rect.w, (h - padding * 2) / rect.h)));
    const tx = w / 2 - scale * (rect.x + rect.w / 2);
    const ty = h / 2 - scale * (rect.y + rect.h / 2);
    const target = zoomIdentity.translate(tx, ty).scale(scale);
    if (opts.animate === false || reducedMotion) {
      selection.call(behavior.transform, target);
    } else {
      selection.transition().duration(650).ease(easeCubicInOut).call(behavior.transform, target);
    }
  }

  return {
    getScale: () => current.k,
    fitTo,
    currentTransform: () => current,
  };
}

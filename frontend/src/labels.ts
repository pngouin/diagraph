import { curvePoint, curveTangent, polylineHitsRect, type Curve, type Polyline } from "./curves";
import type { Rect } from "./geometry";

export interface LabelObstacles {
  labels: Rect[];
  nodes: Rect[];
  edges: Polyline[];
}

export interface Placement {
  key: number;
  rect: Rect;
}

const TS = [0.5, 0.4, 0.6, 0.3, 0.7];
// In unzoomed screen units; scaled by --inv-zoom like the label itself.
const GAPS = [6, 20, 34, 48];
const LABEL_OVERLAP_WEIGHT = 1000;
const NODE_OVERLAP_WEIGHT = 100;
const EDGE_CROSSING_WEIGHT = 10;
const DISTANCE_WEIGHT = 2;
const INNER_SIDE_PENALTY = 0.5;
const SWITCH_PENALTY = 3;

function overlapFraction(r: Rect, o: Rect): number {
  const w = Math.min(r.x + r.w, o.x + o.w) - Math.max(r.x, o.x);
  const h = Math.min(r.y + r.h, o.y + o.h) - Math.max(r.y, o.y);
  if (w <= 0 || h <= 0) return 0;
  return (w * h) / Math.max(r.w * r.h, 1e-9);
}

/**
 * Picks where a `size` label sits along `q`: a few points near the middle of
 * the curve, either side, a few gaps out, scored by what each would cover.
 * `side` is the side the curve bows toward; `previous` is last tick's pick,
 * favored so labels don't hop between near-equal spots while nodes move.
 */
export function placeLabel(
  q: Curve,
  side: 1 | -1,
  size: { w: number; h: number },
  scale: number,
  obstacles: LabelObstacles,
  previous: number | null
): Placement {
  const mid = curvePoint(q, 0.5);
  let best: Placement | null = null;
  let bestScore = Infinity;
  let key = 0;
  for (const t of TS) {
    const p = curvePoint(q, t);
    const tan = curveTangent(q, t);
    const len = Math.hypot(tan.x, tan.y) || 1;
    for (const s of [side, -side]) {
      const nx = (-tan.y / len) * s;
      const ny = (tan.x / len) * s;
      const halfExtent = (Math.abs(nx) * size.w + Math.abs(ny) * size.h) / 2;
      for (const gap of GAPS) {
        const reach = gap * scale + halfExtent;
        const cx = p.x + nx * reach;
        const cy = p.y + ny * reach;
        const rect = { x: cx - size.w / 2, y: cy - size.h / 2, w: size.w, h: size.h };
        let score = DISTANCE_WEIGHT * (Math.hypot(cx - mid.x, cy - mid.y) / Math.max(size.h, 1e-9));
        if (s !== side) score += INNER_SIDE_PENALTY;
        if (previous !== null && key !== previous) score += SWITCH_PENALTY;
        for (const o of obstacles.labels) score += LABEL_OVERLAP_WEIGHT * overlapFraction(rect, o);
        for (const o of obstacles.nodes) score += NODE_OVERLAP_WEIGHT * overlapFraction(rect, o);
        if (score < bestScore) {
          for (const line of obstacles.edges) {
            if (polylineHitsRect(line, rect)) score += EDGE_CROSSING_WEIGHT;
            if (score >= bestScore) break;
          }
        }
        if (score < bestScore) {
          bestScore = score;
          best = { key, rect };
        }
        key++;
      }
    }
  }
  return best!;
}

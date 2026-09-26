import { curvePolyline, polylineHitsRect, type Curve } from "./curves";
import type { Point, Rect } from "./geometry";

export interface RouteObstacle {
  key: string;
  rect: Rect;
  weight: number;
}

/** One edge of a parallel group, in its own direction. */
export interface RouteMember {
  a: Point;
  b: Point;
  /** This edge's lane bow within the group, in its own perpendicular frame. */
  laneBow: number;
  /** +1 if this edge runs the same way as the group's first edge, -1 if reversed. */
  along: 1 | -1;
  /** Obstacle keys this edge is allowed to cross: its endpoints and what contains them. */
  touches: Set<string>;
}

// Sideways shift of the whole group, as a fraction of each edge's length, in
// the group's canonical perpendicular frame.
const SHIFTS = [0, 0.3, -0.3, 0.6, -0.6, 0.9, -0.9];
const SHIFT_WEIGHT = 2;
const SWITCH_PENALTY = 1;

export function memberBow(m: RouteMember, shift: number): number {
  return m.laneBow + m.along * shift * Math.hypot(m.b.x - m.a.x, m.b.y - m.a.y);
}

export function bowCurve(a: Point, b: Point, bow: number): Curve {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const dist = Math.hypot(dx, dy) || 1;
  const mx = (a.x + b.x) / 2;
  const my = (a.y + b.y) / 2;
  return { a, c: { x: mx - dy * (bow / dist), y: my + dx * (bow / dist) }, b };
}

/**
 * Picks one sideways shift for a whole parallel group, so its lanes move
 * together instead of collapsing onto the same detour, scored by the
 * unrelated nodes and frames each member's curve would pass through.
 * `previous` is last tick's pick, favored so routes don't flip while dragging.
 */
export function chooseShift(members: RouteMember[], obstacles: RouteObstacle[], previous: number | null): number {
  let best = 0;
  let bestScore = Infinity;
  SHIFTS.forEach((shift, key) => {
    let score = SHIFT_WEIGHT * Math.abs(shift);
    if (previous !== null && key !== previous) score += SWITCH_PENALTY;
    for (const m of members) {
      if (score >= bestScore) break;
      const line = curvePolyline(bowCurve(m.a, m.b, memberBow(m, shift)));
      for (const o of obstacles) {
        if (m.touches.has(o.key)) continue;
        if (polylineHitsRect(line, o.rect)) score += o.weight;
        if (score >= bestScore) break;
      }
    }
    if (score < bestScore) {
      bestScore = score;
      best = key;
    }
  });
  return best;
}

export function shiftOf(key: number): number {
  return SHIFTS[key]!;
}

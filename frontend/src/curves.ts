import { rectsOverlap, type Point, type Rect } from "./geometry";

/** Quadratic Bézier from `a` to `b` with control point `c`. */
export interface Curve {
  a: Point;
  c: Point;
  b: Point;
}

export interface Polyline {
  points: Point[];
  bounds: Rect;
}

export function curvePoint(q: Curve, t: number): Point {
  const u = 1 - t;
  return {
    x: u * u * q.a.x + 2 * u * t * q.c.x + t * t * q.b.x,
    y: u * u * q.a.y + 2 * u * t * q.c.y + t * t * q.b.y,
  };
}

export function curveTangent(q: Curve, t: number): Point {
  return {
    x: 2 * (1 - t) * (q.c.x - q.a.x) + 2 * t * (q.b.x - q.c.x),
    y: 2 * (1 - t) * (q.c.y - q.a.y) + 2 * t * (q.b.y - q.c.y),
  };
}

export function curvePolyline(q: Curve, segments = 8): Polyline {
  const points: Point[] = [];
  for (let i = 0; i <= segments; i++) points.push(curvePoint(q, i / segments));
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const x0 = Math.min(...xs);
  const y0 = Math.min(...ys);
  return { points, bounds: { x: x0, y: y0, w: Math.max(...xs) - x0, h: Math.max(...ys) - y0 } };
}

// Liang–Barsky clip: does the segment p→q enter the rectangle at all?
function segmentHitsRect(p: Point, q: Point, r: Rect): boolean {
  const dx = q.x - p.x;
  const dy = q.y - p.y;
  let t0 = 0;
  let t1 = 1;
  const edges: [number, number][] = [
    [-dx, p.x - r.x],
    [dx, r.x + r.w - p.x],
    [-dy, p.y - r.y],
    [dy, r.y + r.h - p.y],
  ];
  for (const [den, num] of edges) {
    if (den === 0) {
      if (num < 0) return false;
    } else {
      const t = num / den;
      if (den < 0) t0 = Math.max(t0, t);
      else t1 = Math.min(t1, t);
      if (t0 > t1) return false;
    }
  }
  return true;
}

export function polylineHitsRect(line: Polyline, r: Rect): boolean {
  if (!rectsOverlap(line.bounds, r)) return false;
  for (let i = 1; i < line.points.length; i++) {
    if (segmentHitsRect(line.points[i - 1]!, line.points[i]!, r)) return true;
  }
  return false;
}

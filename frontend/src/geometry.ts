export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Point {
  x: number;
  y: number;
}

export function rectCenter(r: Rect): Point {
  return { x: r.x + r.w / 2, y: r.y + r.h / 2 };
}

export function inflate(r: Rect, amount: number): Rect {
  return { x: r.x - amount, y: r.y - amount, w: r.w + amount * 2, h: r.h + amount * 2 };
}

export function union(a: Rect, b: Rect): Rect {
  const x0 = Math.min(a.x, b.x);
  const y0 = Math.min(a.y, b.y);
  const x1 = Math.max(a.x + a.w, b.x + b.w);
  const y1 = Math.max(a.y + a.h, b.y + b.h);
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

/** Where a ray from `rect`'s center toward `target` crosses the rectangle's border. */
export function rectBorderPoint(rect: Rect, target: Point): Point {
  const c = rectCenter(rect);
  const dx = target.x - c.x;
  const dy = target.y - c.y;
  if (dx === 0 && dy === 0) return c;
  const hw = rect.w / 2;
  const hh = rect.h / 2;
  const scale = 1 / Math.max(Math.abs(dx) / hw, Math.abs(dy) / hh);
  return { x: c.x + dx * scale, y: c.y + dy * scale };
}

/** Lays boxes left to right in one row, each vertically centered against the row's max height. */
export function packRow(
  sizes: { w: number; h: number }[],
  gap: number
): { positions: Point[]; size: { w: number; h: number } } {
  const maxH = sizes.reduce((m, s) => Math.max(m, s.h), 0);
  let x = 0;
  const positions = sizes.map((s) => {
    const p = { x, y: (maxH - s.h) / 2 };
    x += s.w + gap;
    return p;
  });
  return { positions, size: { w: Math.max(x - gap, 0), h: maxH } };
}

export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

export function lerpPoint(a: Point, b: Point, t: number): Point {
  return { x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t) };
}

/** 0 below `lo`, 1 above `hi`, smooth cubic ease between. */
export function smoothstep(lo: number, hi: number, value: number): number {
  if (lo === hi) return value < lo ? 0 : 1;
  const t = Math.min(1, Math.max(0, (value - lo) / (hi - lo)));
  return t * t * (3 - 2 * t);
}

export function clamp(value: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, value));
}

export function rectsOverlap(a: Rect, b: Rect): boolean {
  return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;
}

export function overlapArea(a: Rect, b: Rect): number {
  const w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
  const h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
  return w > 0 && h > 0 ? w * h : 0;
}

/** How far `rect` (anchored at its current x/y) can grow right/down before
 * touching a sibling that's roughly in line with it on the other axis. */
export function maxGrowth(rect: Rect, siblings: Rect[], margin: number): { w: number; h: number } {
  let maxW = Infinity;
  let maxH = Infinity;
  for (const s of siblings) {
    const verticalOverlap = rect.y < s.y + s.h && rect.y + rect.h > s.y;
    if (verticalOverlap && s.x >= rect.x) maxW = Math.min(maxW, s.x - rect.x - margin);
    const horizontalOverlap = rect.x < s.x + s.w && rect.x + rect.w > s.x;
    if (horizontalOverlap && s.y >= rect.y) maxH = Math.min(maxH, s.y - rect.y - margin);
  }
  return { w: maxW, h: maxH };
}

/** Packs same-sized-cell boxes into a grid; returns each box's top-left offset plus total content size. */
export function packGrid(
  sizes: { w: number; h: number }[],
  gap: number
): { positions: Point[]; size: { w: number; h: number } } {
  const n = sizes.length;
  if (n === 0) return { positions: [], size: { w: 0, h: 0 } };
  const cols = Math.max(1, Math.ceil(Math.sqrt(n)));
  const rows = Math.ceil(n / cols);
  const colWidths = new Array(cols).fill(0);
  const rowHeights = new Array(rows).fill(0);
  sizes.forEach((s, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    colWidths[col] = Math.max(colWidths[col], s.w);
    rowHeights[row] = Math.max(rowHeights[row], s.h);
  });
  const colX: number[] = [0];
  for (let c = 0; c < cols; c++) colX.push(colX[c]! + colWidths[c]! + gap);
  const rowY: number[] = [0];
  for (let r = 0; r < rows; r++) rowY.push(rowY[r]! + rowHeights[r]! + gap);

  const positions = sizes.map((s, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    const cellW = colWidths[col]!;
    const cellH = rowHeights[row]!;
    return {
      x: colX[col]! + (cellW - s.w) / 2,
      y: rowY[row]! + (cellH - s.h) / 2,
    };
  });

  const totalW = colX[cols]! - gap;
  const totalH = rowY[rows]! - gap;
  return { positions, size: { w: Math.max(totalW, 0), h: Math.max(totalH, 0) } };
}

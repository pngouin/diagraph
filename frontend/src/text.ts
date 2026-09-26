let ctx: CanvasRenderingContext2D | null = null;

function getCtx(): CanvasRenderingContext2D | null {
  if (ctx) return ctx;
  const canvas = document.createElement("canvas");
  ctx = canvas.getContext("2d");
  return ctx;
}

const MONO_STACK = 'ui-monospace, "SF Mono", "Cascadia Code", "Segoe UI Mono", Consolas, monospace';
const SANS_STACK = '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif';

// Must match the unscaled font declarations in template.html.
export const MONO_FONT = `10.5px ${MONO_STACK}`;
export const COMPONENT_FONT = `600 11px ${MONO_STACK}`;
export const ENVIRONMENT_FONT = `500 11px ${SANS_STACK}`;

export function measureWidth(text: string, font: string = MONO_FONT): number {
  const c = getCtx();
  if (!c) return text.length * 7.2;
  c.font = font;
  return c.measureText(text).width;
}

/** Longest prefix of `text` (plus an ellipsis) that fits in `maxWidth`. */
export function fitText(text: string, maxWidth: number, font: string = MONO_FONT): string {
  if (measureWidth(text, font) <= maxWidth) return text;
  const chars = [...text];
  let lo = 0;
  let hi = chars.length;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (measureWidth(chars.slice(0, mid).join("") + "…", font) <= maxWidth) lo = mid;
    else hi = mid - 1;
  }
  return lo === 0 ? "…" : chars.slice(0, lo).join("") + "…";
}

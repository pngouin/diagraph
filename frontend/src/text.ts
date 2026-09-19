let ctx: CanvasRenderingContext2D | null = null;

function getCtx(): CanvasRenderingContext2D | null {
  if (ctx) return ctx;
  const canvas = document.createElement("canvas");
  ctx = canvas.getContext("2d");
  return ctx;
}

export const MONO_FONT =
  '10.5px ui-monospace, "SF Mono", "Cascadia Code", "Segoe UI Mono", Consolas, monospace';

export function measureWidth(text: string, font: string = MONO_FONT): number {
  const c = getCtx();
  if (!c) return text.length * 7.2;
  c.font = font;
  return c.measureText(text).width;
}

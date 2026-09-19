// A curated technical palette — enough hue separation to tell environments
// apart at a glance without reading into a rainbow-chart cliché.
const PALETTE = ["#2f6f5e", "#b5652d", "#5b5ea6", "#9c3d54", "#3d6e8f", "#7a7530"];
const NEUTRAL = "#6b7570";

export function makeEnvironmentColor(environments: string[]): (env: string | null) => string {
  const index = new Map(environments.map((e, i) => [e, i]));
  return (env) => {
    if (env == null) return NEUTRAL;
    const i = index.get(env);
    return i == null ? NEUTRAL : PALETTE[i % PALETTE.length]!;
  };
}

export function withAlpha(hex: string, alpha: number): string {
  const n = parseInt(hex.slice(1), 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return `rgba(${r},${g},${b},${alpha})`;
}

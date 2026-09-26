import { makeEnvironmentColor } from "./colors";
import type { Rect } from "./geometry";
import { renderInspector } from "./inspector";
import { mount, type SelectableRef } from "./render";
import type { Payload } from "./types";
import { buildWorld, computeBounds } from "./world";
import { attachZoom } from "./zoom";

function boot() {
  const dataEl = document.getElementById("diagraph-data");
  if (!dataEl) return;
  const payload: Payload = JSON.parse(dataEl.textContent ?? "{}");

  const svg = document.getElementById("canvas") as unknown as SVGSVGElement;
  const viewport = document.getElementById("viewport") as unknown as SVGGElement;
  const searchInput = document.getElementById("search") as HTMLInputElement;
  const fitButton = document.getElementById("fit-view") as HTMLButtonElement;
  const themeToggle = document.getElementById("theme-toggle") as HTMLButtonElement;
  const inspector = document.getElementById("inspector-body") as HTMLElement;
  const legend = document.getElementById("legend-body") as HTMLElement;
  const emptyState = document.getElementById("empty-state") as HTMLElement;

  initTheme(themeToggle);

  if (payload.views.global.nodes.length === 0) {
    emptyState.style.display = "flex";
    document.getElementById("legend")!.style.display = "none";
    document.getElementById("inspector")!.style.display = "none";
    searchInput.disabled = true;
    fitButton.disabled = true;
    return;
  }

  const world = buildWorld(payload);
  const envColor = makeEnvironmentColor(payload.environments);

  const zoom = attachZoom(svg, viewport, () => handles.tick());

  function select(ref: SelectableRef | null) {
    searchInput.value = "";
    handles.setSelection(ref);
    renderInspector(inspector, ref, world, navigate);
  }

  const handles = mount(viewport, world, envColor, {
    onSelect: select,
    onFocusRequest(rect: Rect) {
      zoom.fitTo(rect, { padding: 64 });
    },
    getScale: () => zoom.getScale(),
    getVisibleRect: () => zoom.visibleRect(),
    requestTick: () => handles.tick(),
  });

  function navigate(ref: SelectableRef) {
    select(ref);
    if (ref.type === "component") {
      const c = world.componentsByName.get(ref.name)!;
      zoom.fitTo(c.rect, { padding: 64 });
    } else if (ref.type === "part") {
      const owner = world.componentsByName.get(ref.owner)!;
      zoom.fitTo(owner.rect, { padding: 64 });
    } else if (ref.type === "external") {
      const ext = world.externalsByName.get(ref.name)!;
      zoom.fitTo({ x: ext.rect.x - 200, y: ext.rect.y - 200, w: ext.rect.w + 400, h: ext.rect.h + 400 }, { padding: 64 });
    }
  }

  svg.addEventListener("click", () => select(null));

  searchInput.addEventListener("input", () => {
    handles.setSearch(searchInput.value.trim());
  });

  fitButton.addEventListener("click", () => zoom.fitTo(computeBounds(world)));

  buildLegend(legend, payload.environments, envColor, world.componentsByName.size > 0);
  renderInspector(inspector, null, world, navigate);
  zoom.fitTo(computeBounds(world), { animate: false });
}

const THEME_KEY = "diagraph:theme";

function currentTheme(): "light" | "dark" {
  const pinned = document.documentElement.dataset.theme;
  if (pinned === "light" || pinned === "dark") return pinned;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function initTheme(toggle: HTMLButtonElement) {
  const setLabel = () => {
    toggle.textContent = currentTheme() === "dark" ? "Light" : "Dark";
  };
  setLabel();
  toggle.addEventListener("click", () => {
    const next = currentTheme() === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    try {
      localStorage.setItem(THEME_KEY, next);
    } catch {
      // Private browsing / blocked storage — theme still applies for this session.
    }
    setLabel();
  });
}

function buildLegend(
  container: HTMLElement,
  environments: string[],
  envColor: (env: string | null) => string,
  hasComponents: boolean
) {
  container.textContent = "";
  const addRow = (swatchClass: string, color: string | null, text: string) => {
    const r = document.createElement("div");
    r.className = "legend-row";
    const s = document.createElement("span");
    s.className = `legend-swatch ${swatchClass}`;
    if (color) s.style.setProperty("--swatch-color", color);
    const t = document.createElement("span");
    t.textContent = text;
    r.append(s, t);
    container.appendChild(r);
  };
  for (const env of environments) addRow("legend-swatch-env", envColor(env), env);
  if (!hasComponents) return;
  addRow("legend-swatch-component", null, "component");
  addRow("legend-swatch-part", null, "part, nested inside its component");
  addRow("legend-swatch-external", null, "external target");
  addRow("legend-swatch-dashed", null, "crosses environments");
}

boot();

import type { World, WorldEdge } from "./world";
import type { SelectableRef } from "./render";

function refLabel(ref: SelectableRef, world: World): string {
  if (ref.type === "part") {
    const owner = world.componentsByName.get(ref.owner);
    return owner?.parts.find((p) => p.uid === ref.uid)?.name ?? ref.uid;
  }
  if (ref.type === "environment") return ref.name ?? "no environment";
  return ref.name;
}

function row(parent: HTMLElement, cls: string): HTMLDivElement {
  const d = document.createElement("div");
  d.className = cls;
  parent.appendChild(d);
  return d;
}

function heading(parent: HTMLElement, text: string, sub?: string) {
  const h = row(parent, "inspector-heading");
  const name = document.createElement("div");
  name.className = "inspector-name";
  name.textContent = text;
  h.appendChild(name);
  if (sub) {
    const s = document.createElement("div");
    s.className = "inspector-sub";
    s.textContent = sub;
    h.appendChild(s);
  }
}

function edgeRow(parent: HTMLElement, direction: "in" | "out", other: string, label: string | null, onClick: () => void) {
  const r = row(parent, "inspector-edge");
  const arrow = document.createElement("span");
  arrow.className = "inspector-edge-arrow";
  arrow.textContent = direction === "out" ? "→" : "←";
  const target = document.createElement("button");
  target.type = "button";
  target.className = "inspector-link";
  target.textContent = other;
  target.addEventListener("click", onClick);
  r.append(arrow, target);
  if (label) {
    const l = document.createElement("span");
    l.className = "inspector-edge-label";
    l.textContent = label;
    r.appendChild(l);
  }
}

export function renderInspector(
  container: HTMLElement,
  ref: SelectableRef | null,
  world: World,
  navigate: (ref: SelectableRef) => void
) {
  container.textContent = "";
  if (!ref) {
    const empty = row(container, "inspector-empty");
    empty.textContent = "Select a node to see what it talks to.";
    return;
  }

  const edges = world.edges.filter((e) => sideIs(e, "from", ref) || sideIs(e, "to", ref));

  if (ref.type === "component") {
    const c = world.componentsByName.get(ref.name)!;
    heading(container, c.name, c.environment ?? "no environment");
    if (c.hasParts) {
      const list = row(container, "inspector-section");
      list.textContent = "Parts";
      for (const p of c.parts) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "inspector-chip";
        btn.textContent = p.name;
        btn.addEventListener("click", () => navigate({ type: "part", owner: p.owner, uid: p.uid }));
        list.appendChild(btn);
      }
    }
    section(container, "Talks to", edges, ref, world, navigate);
  } else if (ref.type === "part") {
    const owner = world.componentsByName.get(ref.owner)!;
    const part = owner.parts.find((p) => p.uid === ref.uid)!;
    heading(container, part.name, `part of ${owner.name}`);
    section(container, "Connections", edges, ref, world, navigate);
  } else if (ref.type === "external") {
    heading(container, ref.name, "external target");
    section(container, "Called by", edges, ref, world, navigate);
  } else {
    const env = world.environments.find((e) => e.name === ref.name)!;
    heading(container, ref.name ?? "no environment", `${env.components.length} component${env.components.length === 1 ? "" : "s"}`);
    const list = row(container, "inspector-section");
    list.textContent = "Components";
    for (const c of env.components) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "inspector-chip";
      btn.textContent = c.name;
      btn.addEventListener("click", () => navigate({ type: "component", name: c.name }));
      list.appendChild(btn);
    }
  }
}

function sideIs(edge: WorldEdge, side: "from" | "to", ref: SelectableRef): boolean {
  const s = edge[side];
  const r = s.ref;
  if (ref.type === "component" && r.type === "component") return r.name === ref.name;
  if (ref.type === "external" && r.type === "external") return r.name === ref.name;
  if (ref.type === "part" && r.type === "part") return r.uid === ref.uid;
  // A boundary edge that targets a specific part (`to_part`/`from_part` in
  // diagram.toml) still keys its ref to the owning component — only the
  // anchor names the part — so a part selection has to match through that.
  if (ref.type === "part" && r.type === "component" && s.anchor.kind === "part") {
    return r.name === ref.owner && s.anchor.uid === ref.uid;
  }
  return false;
}

function section(
  container: HTMLElement,
  title: string,
  edges: WorldEdge[],
  ref: SelectableRef,
  world: World,
  navigate: (r: SelectableRef) => void
) {
  if (edges.length === 0) return;
  const sec = row(container, "inspector-section");
  sec.textContent = title;
  for (const e of edges) {
    const outgoing = sideIs(e, "from", ref);
    const other = outgoing ? e.to.ref : e.from.ref;
    const otherSelectable: SelectableRef =
      other.type === "part" ? { type: "part", owner: other.owner, uid: other.uid } : other;
    edgeRow(container, outgoing ? "out" : "in", refLabel(otherSelectable, world), e.label, () => navigate(otherSelectable));
  }
}

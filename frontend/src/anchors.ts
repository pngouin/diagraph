import { lerpPoint, rectBorderPoint, rectCenter, type Point, type Rect } from "./geometry";
import type { EndpointSide, PartNode, World } from "./world";

export function worldRectOfPart(ownerRect: Rect, part: PartNode): Rect {
  return { x: ownerRect.x + part.rect.x, y: ownerRect.y + part.rect.y, w: part.rect.w, h: part.rect.h };
}

function approxPoint(side: EndpointSide, world: World, revealFactors: Map<string, number>): Point {
  const ref = side.ref;
  if (ref.type === "external") return rectCenter(world.externalsByName.get(ref.name)!.rect);
  if (ref.type === "part") {
    const owner = world.componentsByName.get(ref.owner)!;
    const part = owner.parts.find((p) => p.uid === ref.uid)!;
    return rectCenter(worldRectOfPart(owner.rect, part));
  }
  const comp = world.componentsByName.get(ref.name)!;
  const compCenter = rectCenter(comp.rect);
  const anchor = side.anchor;
  if (anchor.kind === "part") {
    const part = comp.parts.find((p) => p.uid === anchor.uid);
    if (part) {
      const t = revealFactors.get(comp.name) ?? 0;
      return lerpPoint(compCenter, rectCenter(worldRectOfPart(comp.rect, part)), t);
    }
  }
  return compCenter;
}

function precisePoint(side: EndpointSide, world: World, revealFactors: Map<string, number>, target: Point): Point {
  const ref = side.ref;
  if (ref.type === "external") return rectBorderPoint(world.externalsByName.get(ref.name)!.rect, target);
  if (ref.type === "part") {
    const owner = world.componentsByName.get(ref.owner)!;
    const part = owner.parts.find((p) => p.uid === ref.uid)!;
    return rectBorderPoint(worldRectOfPart(owner.rect, part), target);
  }
  const comp = world.componentsByName.get(ref.name)!;
  const outer = rectBorderPoint(comp.rect, target);
  const anchor = side.anchor;
  if (anchor.kind === "part") {
    const part = comp.parts.find((p) => p.uid === anchor.uid);
    if (part) {
      const inner = rectBorderPoint(worldRectOfPart(comp.rect, part), target);
      const t = revealFactors.get(comp.name) ?? 0;
      return lerpPoint(outer, inner, t);
    }
  }
  return outer;
}

export function edgeEndpoints(
  from: EndpointSide,
  to: EndpointSide,
  world: World,
  revealFactors: Map<string, number>
): { a: Point; b: Point } {
  const aApprox = approxPoint(from, world, revealFactors);
  const bApprox = approxPoint(to, world, revealFactors);
  return {
    a: precisePoint(from, world, revealFactors, bApprox),
    b: precisePoint(to, world, revealFactors, aApprox),
  };
}

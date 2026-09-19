/** Adjacency weights keyed by item key; symmetric (bumping a->b also bumps b->a). */
export type Adjacency = Map<string, Map<string, number>>;

export function buildAdjacency(pairs: { a: string; b: string }[]): Adjacency {
  const adjacency: Adjacency = new Map();
  const bump = (x: string, y: string) => {
    let neighbors = adjacency.get(x);
    if (!neighbors) {
      neighbors = new Map();
      adjacency.set(x, neighbors);
    }
    neighbors.set(y, (neighbors.get(y) ?? 0) + 1);
  };
  for (const { a, b } of pairs) {
    if (a === b) continue;
    bump(a, b);
    bump(b, a);
  }
  return adjacency;
}

function relocate<T>(order: T[], from: number, to: number): T[] {
  const copy = [...order];
  const [item] = copy.splice(from, 1);
  copy.splice(to, 0, item as T);
  return copy;
}

/**
 * Hill-climbs `seed` toward a local minimum of `cost` by moving one item to
 * a different position (not just swapping adjacent pairs — that gets stuck
 * far more easily: a hub whose neighbors should split across both its sides
 * needs a multi-slot move, not a swap, to reach that arrangement). Runs
 * until no single relocation helps.
 */
function localSearch<T>(seed: T[], cost: (order: T[]) => number): T[] {
  let current = [...seed];
  let currentCost = cost(current);
  let improved = true;
  while (improved) {
    improved = false;
    for (let from = 0; from < current.length; from++) {
      for (let to = 0; to < current.length; to++) {
        if (from === to) continue;
        const next = relocate(current, from, to);
        const nextCost = cost(next);
        if (nextCost < currentCost) {
          current = next;
          currentCost = nextCost;
          improved = true;
        }
      }
    }
  }
  return current;
}

function arrangementCost(order: string[], adjacency: Adjacency): number {
  const index = new Map(order.map((key, i) => [key, i]));
  let cost = 0;
  for (const [key, neighbors] of adjacency) {
    const ki = index.get(key);
    if (ki === undefined) continue;
    for (const [other, weight] of neighbors) {
      const oi = index.get(other);
      if (oi === undefined) continue;
      cost += weight * Math.abs(ki - oi);
    }
  }
  return cost;
}

function byDescendingDegree(keys: string[], adjacency: Adjacency): string[] {
  const degree = (key: string) => {
    const neighbors = adjacency.get(key);
    if (!neighbors) return 0;
    let sum = 0;
    for (const weight of neighbors.values()) sum += weight;
    return sum;
  };
  return [...keys].sort((a, b) => degree(b) - degree(a) || a.localeCompare(b));
}

/**
 * Orders `keys` so items connected by heavier edges end up closer together
 * (minimizing total weighted distance between connected pairs) instead of
 * the plain alphabetical order this replaces, which scatters connected
 * components/environments and forces manual repositioning to untangle.
 * Local search can still settle in a local optimum depending on where it
 * starts, so a few different seed orders are tried and the best kept; cheap
 * at the scale these diagrams run at (a handful of environments, or
 * components per environment).
 */
export function orderByMinimizingCrossings(keys: string[], adjacency: Adjacency): string[] {
  const seeds = [
    [...keys].sort((a, b) => a.localeCompare(b)),
    [...keys].sort((a, b) => b.localeCompare(a)),
    byDescendingDegree(keys, adjacency),
  ];
  let best = seeds[0]!;
  let bestCost = Infinity;
  for (const seed of seeds) {
    const candidate = localSearch(seed, (order) => arrangementCost(order, adjacency));
    const cost = arrangementCost(candidate, adjacency);
    if (cost < bestCost) {
      bestCost = cost;
      best = candidate;
    }
  }
  return best;
}

function arrangementCostWithinGroup(
  order: string[],
  thisGroupIndex: number,
  groupOf: (key: string) => string,
  groupOrderIndex: (group: string) => number,
  adjacency: Adjacency
): number {
  const n = order.length;
  const index = new Map(order.map((key, i) => [key, i]));
  let cost = 0;
  for (const key of order) {
    const ki = index.get(key)!;
    const neighbors = adjacency.get(key);
    if (!neighbors) continue;
    for (const [other, weight] of neighbors) {
      const oi = index.get(other);
      const target = oi ?? (groupOrderIndex(groupOf(other)) < thisGroupIndex ? 0 : n - 1);
      cost += weight * Math.abs(ki - target);
    }
  }
  return cost;
}

/**
 * Same idea as `orderByMinimizingCrossings`, for items nested inside a fixed
 * outer ordering (e.g. components within their environment's place in the
 * environment row). A same-group neighbor pulls toward its actual position;
 * a neighbor in another group only contributes a direction — toward index 0
 * if its group sits earlier in `groupOrderIndex`, otherwise toward the last
 * index — since its exact position in a different group isn't comparable.
 */
export function orderWithinGroupMinimizingCrossings(
  keys: string[],
  thisGroupIndex: number,
  groupOf: (key: string) => string,
  groupOrderIndex: (group: string) => number,
  adjacency: Adjacency
): string[] {
  if (keys.length <= 2) return [...keys];
  const cost = (order: string[]) =>
    arrangementCostWithinGroup(order, thisGroupIndex, groupOf, groupOrderIndex, adjacency);
  const seeds = [[...keys].sort((a, b) => a.localeCompare(b)), [...keys].sort((a, b) => b.localeCompare(a))];
  let best = seeds[0]!;
  let bestCost = Infinity;
  for (const seed of seeds) {
    const candidate = localSearch(seed, cost);
    const candidateCost = cost(candidate);
    if (candidateCost < bestCost) {
      bestCost = candidateCost;
      best = candidate;
    }
  }
  return best;
}

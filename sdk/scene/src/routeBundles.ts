import type { PlacedRoute, Point, SharedRouteCorridor } from "./geometry";

type Member = { readonly route: PlacedRoute; readonly segment: number };
type Branch = {
  readonly points: readonly Point[];
  readonly members: Member[];
  readonly children: Map<string, Branch>;
};

/** @internal
 * Index exact directed segments outward from a selected common port.
 * The caller supplies validated routes and at most two selections per route.
 * A branch is a shared corridor only when at least two routes have real extent.
 * Its points are existing ELK geometry, not a new spline or an inferred connection.
 *
 * Each selected segment is visited once while indexing. Member storage and the
 * traversal are bounded by those visits; output ranges retain each original route.
 * Frames and line/cubic kinds never mix. Equality is exact, not tolerance-based.
 */
export function sharedRouteCorridors(routes: readonly PlacedRoute[], end: "source" | "target"): readonly SharedRouteCorridor[] {
  const roots = new Map<string, Map<string, Branch>>();
  for (const route of routes) {
    const frameKey = JSON.stringify([route.containerId, route.kind]);
    let branches: Map<string, Branch> = roots.get(frameKey) ?? new Map();
    roots.set(frameKey, branches);
    const stride = route.kind === "line" ? 1 : 3;
    const count = (route.points.length - 1) / stride;
    for (let offset = 0; offset < count; offset++) {
      const segment = end === "source" ? offset : count - offset - 1;
      const points = route.points.slice(segment * stride, (segment + 1) * stride + 1);
      const key = JSON.stringify(points.map(point => [point.x, point.y]));
      let branch: Branch | undefined = branches.get(key);
      if (!branch) { branch = { points, members: [], children: new Map() }; branches.set(key, branch); }
      branch.members.push({ route, segment });
      branches = branch.children;
    }
  }
  const corridors: SharedRouteCorridor[] = [];
  // Reverse the stack insertion to retain first-member input order.
  const pending = [...roots.values()].flatMap(branches => [...branches.values()]).reverse();
  while (pending.length !== 0) {
    const first = pending.pop()!;
    if (first.members.length < 2) continue;
    const chain = [first];
    let last = first;
    // A maximal run ends where membership changes, not at an arbitrary bend.
    while (last.children.size === 1) {
      const child = last.children.values().next().value!;
      if (child.members.length !== last.members.length) break;
      chain.push(child); last = child;
    }
    for (const child of [...last.children.values()].reverse()) pending.push(child);
    const directed = end === "source" ? chain : chain.reverse();
    const points: Point[] = [];
    for (const branch of directed) points.push(...branch.points.slice(points.length === 0 ? 0 : 1));
    const origin = points[0]!;
    // Point-only runs remain on the original routes but do not claim a corridor.
    if (!points.some(point => point.x !== origin.x || point.y !== origin.y)) continue;
    const base = first.members[0]!.route;
    corridors.push({ containerId: base.containerId, routeKind: base.kind, points,
      members: first.members.map(member => {
        const firstSegment = end === "source" ? member.segment : member.segment - chain.length + 1;
        return { routeId: member.route.id, firstSegment, endSegmentExclusive: firstSegment + chain.length };
      }) });
  }
  return corridors;
}

// Visit caller-supplied edges once, preserving the selected discovery order.
export function dependencyClosure(roots, dependencies, { depthFirst = false } = {}) {
  const pending = [...roots];
  if (depthFirst) pending.reverse();
  const selected = new Set();
  let cursor = 0;

  while (cursor < pending.length) {
    const name = depthFirst ? pending.pop() : pending[cursor++];
    if (selected.has(name)) continue;
    selected.add(name);
    const edges = dependencies(name);
    for (const dependency of depthFirst ? [...edges].reverse() : edges) {
      pending.push(dependency);
    }
  }
  return selected;
}

export function generateWeb(nodes: any[], maxDist = 4, maxPerNode = 4): { source: string; target: string }[] {
  const edges: { source: string; target: string }[] = [];
  const buckets = new Map<string, number>();   // track how many each node already has

  nodes.forEach((a, i) => {
    // build a sorted list of nearest neighbours
    const neighbours = nodes
      .filter((_, j) => i !== j)
      .map(b => ({
        node: b,
        d: a.position?.distanceTo(b.position) || 0
      }))
      .filter(n => n.d < maxDist)
      .sort((p, q) => p.d - q.d);              // closest first

    for (const { node: b } of neighbours) {
      const countA = buckets.get(a.id) ?? 0;
      const countB = buckets.get(b.id) ?? 0;
      if (countA >= maxPerNode || countB >= maxPerNode) continue;

      // connect only if neither endpoint is saturated
      edges.push({ source: a.id, target: b.id });
      buckets.set(a.id, countA + 1);
      buckets.set(b.id, countB + 1);
      if ((buckets.get(a.id) ?? 0) >= maxPerNode) break; // A is full
    }
  });

  return edges;
}
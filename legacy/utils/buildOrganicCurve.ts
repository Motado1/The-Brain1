import * as THREE from 'three';

// Simple seeded random function for deterministic curves
function seededRand(seed: string, index: number): number {
  let hash = 0;
  const str = seed + index;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash; // Convert to 32-bit integer
  }
  // Convert to 0-1 range
  return Math.abs(Math.sin(hash * 12.9898 + 78.233)) % 1;
}

/**
 * Returns a wiggly CatmullRomCurve3 between two points.
 *  – segments: number of internal control points
 *  – amp:      max lateral jitter as fraction of total distance
 *  – seed:     optional seed for deterministic randomness
 */
export function buildOrganicCurve(
  start: THREE.Vector3,
  end: THREE.Vector3,
  segments = 5,
  amp = 0.25,
  seed?: string
) {
  const points: THREE.Vector3[] = [start];
  const dir   = new THREE.Vector3().subVectors(end, start);
  const len   = dir.length();
  const right = new THREE.Vector3().crossVectors(dir, new THREE.Vector3(0, 0, 1)).normalize();
  const up    = new THREE.Vector3().crossVectors(dir, right).normalize();

  for (let i = 1; i < segments; i++) {
    const t   = i / segments;
    const p   = start.clone().lerp(end, t);

    if (i === 1) {
      // first control point pulls slightly above the start ➜ bundling
      p.addScaledVector(up, amp * len * 0.5);
    }

    // jitter in two perpendicular directions
    const rightRand = seed ? (seededRand(seed, i * 2) - 0.5) : (Math.random() - 0.5);
    const upRand = seed ? (seededRand(seed, i * 2 + 1) - 0.5) : (Math.random() - 0.5);
    
    const jitter =
      right.clone().multiplyScalar(rightRand * amp * len)
        .add(up.clone().multiplyScalar(upRand * amp * len));

    p.add(jitter);
    points.push(p);
  }
  points.push(end);
  return new THREE.CatmullRomCurve3(points);
}
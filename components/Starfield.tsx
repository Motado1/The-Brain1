'use client';

import { useMemo } from 'react';
import * as THREE from 'three';

export default function Starfield() {
  // Generate 700 random positions for stars
  const positions = useMemo(() => {
    const pos = new Float32Array(700 * 3);
    for (let i = 0; i < 700; i++) {
      pos[i * 3] = (Math.random() - 0.5) * 10000; // x ∈ [-5000, 5000]
      pos[i * 3 + 1] = (Math.random() - 0.5) * 10000; // y ∈ [-5000, 5000]
      pos[i * 3 + 2] = -5000 + Math.random() * 4000; // z ∈ [-5000, -1000]
    }
    return pos;
  }, []);

  return (
    <points>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          count={700}
          array={positions}
          itemSize={3}
        />
      </bufferGeometry>
      <pointsMaterial
        size={1.5}
        color="#ffffff"
        transparent={true}
        opacity={0.6}
        blending={THREE.AdditiveBlending}
        sizeAttenuation={false}
        vertexColors={false}
        fog={false}
      />
    </points>
  );
}
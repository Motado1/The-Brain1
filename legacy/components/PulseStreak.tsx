'use client';

import { useFrame } from '@react-three/fiber';
import { useRef } from 'react';
import { TubeGeometry } from 'three';
import { EdgePulseMaterial } from '@/materials/EdgePulseMaterial';
import * as THREE from 'three';

export function PulseStreak({ curve, color, tOffset }: { 
  curve: THREE.CatmullRomCurve3; 
  color: string; 
  tOffset: number;
}) {
  const ref = useRef<THREE.Mesh>(null!);

  // Build a thin tube once
  const geometry = new TubeGeometry(curve, 64, 0.01, 8, false);

  useFrame(({ clock }) => {
    if (ref.current) {
      const mat = ref.current.material as any;           // EdgePulseMaterial instance
      mat.uniforms.uTime.value  = clock.elapsedTime * 0.5 + tOffset;
      mat.uniforms.uColor.value.set(color);
    }
  });

  return (
    <mesh ref={ref} geometry={geometry}>
      <edgePulseMaterial
        attach="material"
        uWidth={0.05}
        // uColor and uTime are set in useFrame
      />
    </mesh>
  );
}
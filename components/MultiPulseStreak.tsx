'use client';

import { useFrame } from '@react-three/fiber';
import { useRef } from 'react';
import { TubeGeometry } from 'three';
import '@/materials/MultiPulseMaterial';
import * as THREE from 'three';

export function MultiPulseStreak({ 
  curve, 
  color = '#ff6600', 
  thickness = 0.01 
}: { 
  curve: THREE.CatmullRomCurve3; 
  color?: string; 
  thickness?: number;
}) {
  const ref = useRef<THREE.Mesh>(null!);

  // Build tube geometry
  const geometry = new TubeGeometry(curve, 64, thickness, 8, false);

  useFrame(({ clock }) => {
    if (ref.current) {
      const mat = ref.current.material as any;
      mat.uniforms.time.value = clock.getElapsedTime();
      
      // Set hotColor (orange pulse) - use the passed color
      const colorObj = new THREE.Color(color);
      mat.uniforms.hotColor.value.copy(colorObj);
      
      // baseColor is already set to white in the material definition
    }
  });

  return (
    <mesh ref={ref} geometry={geometry}>
      {/* @ts-ignore */}
      <multiPulseMaterial attach="material" />
    </mesh>
  );
}
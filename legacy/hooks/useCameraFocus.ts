import { useEffect } from 'react';
import { useThree } from '@react-three/fiber';
import * as THREE from 'three';
import gsap from 'gsap';

export function useCameraFocus(selectedNode: any) {
  const { camera } = useThree();
  
  useEffect(() => {
    if (!selectedNode) return;
    
    const target = new THREE.Vector3(selectedNode.x, selectedNode.y, selectedNode.z);
    
    gsap.to(camera.position, {
      duration: 1.2,
      x: target.x + 2,
      y: target.y + 2,
      z: target.z + 2,
      onUpdate: () => camera.lookAt(target)
    });
  }, [selectedNode, camera]);
}
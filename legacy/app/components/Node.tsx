"use client";
import { useState, useMemo, useRef, useEffect } from 'react';
import { useBrainStore } from '@/lib/store';
import { NodeData } from '@/lib/types';
import * as THREE from 'three';

// Shared geometries and materials at module level for performance
const sharedGeometry = new THREE.SphereGeometry(1, 16, 16);
const defaultMaterial = new THREE.MeshStandardMaterial({ color: '#229EE6' });
const hoverMaterial = new THREE.MeshStandardMaterial({ color: '#ffaa33' });
const selectedMaterial = new THREE.MeshStandardMaterial({ color: '#ffff00' });

interface NodeProps {
  node: NodeData;
}

export default function Node({ node }: NodeProps) {
  const [hovered, setHovered] = useState(false);
  const [isNew, setIsNew] = useState(true);
  const meshRef = useRef<THREE.Mesh>(null);
  const selectNode = useBrainStore(state => state.setSelectedNode);
  const selectedNodeId = useBrainStore(state => state.selectedNode);
  const isSelected = selectedNodeId === node.id;

  // Animation for new nodes
  useEffect(() => {
    if (isNew && meshRef.current) {
      // Start with scale 0 and animate to full size
      meshRef.current.scale.setScalar(0);
      
      // Simple scale animation using requestAnimationFrame
      let startTime: number;
      const duration = 500; // 500ms animation
      
      const animate = (currentTime: number) => {
        if (!startTime) startTime = currentTime;
        const elapsed = currentTime - startTime;
        const progress = Math.min(elapsed / duration, 1);
        
        // Easing function (easeOutBack for a bouncy effect)
        const easeProgress = 1 + (progress - 1) ** 3;
        
        if (meshRef.current) {
          const targetScale = (hovered ? 1.2 : 1.0) * (node.scale ?? 1);
          meshRef.current.scale.setScalar(easeProgress * targetScale);
        }
        
        if (progress < 1) {
          requestAnimationFrame(animate);
        } else {
          setIsNew(false);
        }
      };
      
      requestAnimationFrame(animate);
    }
  }, [isNew, node.scale, hovered]);

  // Update scale when not animating in
  useEffect(() => {
    if (!isNew && meshRef.current) {
      const targetScale = (hovered ? 1.2 : 1.0) * (node.scale ?? 1);
      meshRef.current.scale.setScalar(targetScale);
    }
  }, [hovered, node.scale, isNew]);

  // Create material based on node's custom color or use shared materials
  const material = useMemo(() => {
    if (node.color && node.color !== '#229EE6') {
      // Create custom material for nodes with specific colors
      return new THREE.MeshStandardMaterial({ color: node.color });
    }
    // Use shared materials for performance
    return isSelected ? selectedMaterial : (hovered ? hoverMaterial : defaultMaterial);
  }, [node.color, isSelected, hovered]);

  return (
    <mesh
      ref={meshRef}
      position={[node.x ?? 0, node.y ?? 0, node.z ?? 0]}
      geometry={sharedGeometry}
      material={material}
      onPointerOver={() => { 
        setHovered(true);
        document.body.style.cursor = 'pointer';
      }}
      onPointerOut={() => {
        setHovered(false);
        document.body.style.cursor = 'default';
      }}
      onClick={() => selectNode(node.id)}
    />
  );
}
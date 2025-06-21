'use client';

import { useRef, useState, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Mesh, MeshBasicMaterial, Group } from 'three';
import * as THREE from 'three';
import { useLoader } from '@react-three/fiber';
import { TextureLoader } from 'three';
import { useGraphStore } from '../stores/graph';
import { useBrainStore } from '../lib/store';
import { isConnectedTo } from '../utils/adjacency';
import { Dendrites } from './Dendrites';

export interface VisualNodeData {
  id: string;
  name: string;
  entity_type: string;
  is_pillar: boolean;
  size: number;
  position?: THREE.Vector3; // Optional Vector3 object
  x?: number; // Coordinate fallback
  y?: number;
  z?: number;
}

interface VisualNodeProps {
  node: VisualNodeData;
  simulationPosition?: { x: number; y: number; z?: number };
  onClick?: (node: VisualNodeData) => void;
}

export default function VisualNode({ 
  node, 
  simulationPosition, 
  onClick 
}: VisualNodeProps) {
  const meshRef = useRef<Group>(null);
  const [hovered, setHovered] = useState(false);
  const selectNode = useGraphStore(s => s.selectNode);
  const selectedNodeId = useGraphStore(s => s.selectedNodeId);
  const isNodeHighlighted = useBrainStore(s => s.isNodeHighlighted);
  
  // Fading logic based on selection
  const faded = selectedNodeId && !isConnectedTo(selectedNodeId, node.id);
  
  // Highlight logic for pulse arrivals
  const highlighted = isNodeHighlighted(node.id);
  const highlightedNodes = useBrainStore(s => s.highlightedNodes);
  
  // Calculate flash intensity based on time remaining
  const flashIntensity = useMemo(() => {
    const expiry = highlightedNodes.get(node.id);
    if (!expiry) return 1.0;
    
    const now = Date.now();
    const timeRemaining = expiry - now;
    const totalDuration = 2000; // 2 seconds
    
    if (timeRemaining <= 0) return 1.0;
    
    // Create a pulsing effect that's strongest at the beginning
    const progress = timeRemaining / totalDuration;
    const pulse = Math.sin(now * 0.01) * 0.5 + 0.5; // Oscillate between 0-1
    return 1.0 + (progress * 2.0 * (1.0 + pulse)); // Range: 1.0 to 4.0 with pulsing
  }, [highlighted, highlightedNodes, node.id]);
  
  // Load halo texture (temporarily disabled for debugging)
  // const haloTexture = useLoader(TextureLoader, '/assets/halo.png');
  
  // Color logic for neural centers
  const baseColor = node.is_pillar ? '#00d8ff' :
                    node.entity_type === 'task' ? '#ff9500' :
                    node.entity_type === 'project' ? '#4ec5ff' :
                    node.entity_type === 'idea' ? '#ffaa00' :
                    '#b47dff';
  
  // Use simulationPosition if available, otherwise use node coordinates
  const position: [number, number, number] = [
    simulationPosition?.x ?? node.x ?? 0,
    simulationPosition?.y ?? node.y ?? 0,
    simulationPosition?.z ?? node.z ?? 0
  ];

  // Simple hover animation and fading logic
  useFrame((state) => {
    if (meshRef.current) {
      const targetScale = hovered ? 1.4 : 1.0;
      meshRef.current.scale.x += (targetScale - meshRef.current.scale.x) * 0.1;
      meshRef.current.scale.y += (targetScale - meshRef.current.scale.y) * 0.1;
      meshRef.current.scale.z += (targetScale - meshRef.current.scale.z) * 0.1;
      
      // Handle fading and visibility
      const targetOpacity = faded ? 0.05 : 1;
      meshRef.current.visible = true; // Always visible, rely on opacity for fading
      
      // Performance optimization: set render order for proper bloom
      const isSelected = node.id === selectedNodeId;
      const isConnected = selectedNodeId && isConnectedTo(selectedNodeId, node.id);
      meshRef.current.renderOrder = (isSelected || isConnected) ? 1 : 0;
      
      // Update material opacity for all child meshes
      meshRef.current.traverse((child) => {
        if (child instanceof THREE.Mesh && child.material) {
          const material = child.material as THREE.Material & { opacity?: number };
          if (material.opacity !== undefined) {
            material.opacity = targetOpacity;
          }
        }
      });
    }
  });

  const handleClick = () => {
    if (onClick) {
      onClick(node);
    }
  };

  const handlePointerOver = () => {
    setHovered(true);
    document.body.style.cursor = 'pointer';
  };

  const handlePointerOut = () => {
    setHovered(false);
    document.body.style.cursor = 'default';
  };

  // Debug log for pillar nodes
  if (node.is_pillar) {
    console.log('🌟 Rendering pillar node:', {
      name: node.name,
      is_pillar: node.is_pillar,
      entity_type: node.entity_type,
      position: node.position,
      x: node.x,
      y: node.y,
      z: node.z,
      simulationPosition
    });
  }

  // Simplified approach - let bloom do the work
  return (
    <group
      ref={meshRef}
      position={position}
      onClick={handleClick}
      onPointerOver={handlePointerOver}
      onPointerOut={handlePointerOut}
    >
      {/* Main sphere */}
      <mesh onPointerDown={(e) => { e.stopPropagation(); selectNode(node.id); }}>
        <sphereGeometry args={[node.is_pillar ? 1.0 : 0.4, 32, 32]} />
        <meshStandardMaterial
          color={baseColor}
          emissive={baseColor}
          emissiveIntensity={(node.is_pillar ? 2.5 : 1.5) * flashIntensity}  // Reduced emissive intensity
          toneMapped={false}
        />
      </mesh>
      
      {/* Outer glow sphere (optional) */}
      <mesh scale={1.5}>
        <sphereGeometry args={[node.is_pillar ? 1.0 : 0.4, 16, 16]} />
        <meshBasicMaterial
          color={baseColor}
          transparent
          opacity={0.2}
          blending={THREE.AdditiveBlending}
          side={THREE.BackSide}
        />
      </mesh>
      
    </group>
  );
}
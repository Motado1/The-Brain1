"use client";
import { useState } from 'react';
import { Line } from '@react-three/drei';
import { useBrainStore } from '@/lib/store';
import { EdgeData } from '@/lib/types';

interface EdgeProps {
  edge: EdgeData;
  onEdit?: (edge: EdgeData) => void;
}

export default function Edge({ edge, onEdit }: EdgeProps) {
  const [hovered, setHovered] = useState(false);
  const nodes = useBrainStore(state => state.nodes);
  const sourceNode = nodes.find(n => n.id === edge.source);
  const targetNode = nodes.find(n => n.id === edge.target);
  
  if (!sourceNode || !targetNode) {
    return null;
  }
  
  const start = [sourceNode.x ?? 0, sourceNode.y ?? 0, sourceNode.z ?? 0] as [number, number, number];
  const end = [targetNode.x ?? 0, targetNode.y ?? 0, targetNode.z ?? 0] as [number, number, number];

  const handleClick = () => {
    if (onEdit) {
      onEdit(edge);
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

  // Calculate line width based on strength and hover state
  const baseWidth = (edge.strength || 1.0) * 1.5;
  const lineWidth = hovered ? baseWidth * 2 : baseWidth;
  
  // Color based on edge type and hover state
  const getEdgeColor = () => {
    if (hovered) return '#2563eb'; // Blue when hovered
    
    switch (edge.edge_type) {
      case 'hierarchy': return '#dc2626'; // Red for hierarchy
      case 'dependency': return '#ea580c'; // Orange for dependency
      case 'reference': return '#7c3aed'; // Purple for reference
      case 'related': return '#059669'; // Green for related
      case 'contains': return '#0891b2'; // Cyan for contains
      case 'supports': return '#be123c'; // Rose for supports
      default: return '#6b7280'; // Gray for default connection
    }
  };

  return (
    <Line 
      points={[start, end]}
      color={getEdgeColor()}
      lineWidth={lineWidth}
      transparent 
      opacity={hovered ? 1.0 : 0.7}
      onClick={handleClick}
      onPointerOver={handlePointerOver}
      onPointerOut={handlePointerOut}
    />
  );
}
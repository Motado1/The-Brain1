'use client';

import { useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { useBrainStore } from '@/lib/store';
import { MultiPulseStreak } from './MultiPulseStreak';

interface PulsesProps {
  simulationNodes?: any[];
}

export default function Pulses({ simulationNodes = [] }: PulsesProps) {
  const pulses = useBrainStore(state => state.pulses);
  const edges = useBrainStore(state => state.edges);
  const nodes = useBrainStore(state => state.nodes);
  const tickPulses = useBrainStore(state => state.tickPulses);
  
  // Create curves for all edges using simulation positions
  const edgeCurves = useMemo(() => {
    const curves = new Map<string, THREE.CatmullRomCurve3>();
    
    edges.forEach(edge => {
      const sourceNode = nodes.find(n => n.id === edge.source);
      const targetNode = nodes.find(n => n.id === edge.target);
      
      if (sourceNode && targetNode) {
        // Get simulation positions (same as VisualNode and Synapse components)
        const sourceSimNode = simulationNodes.find(n => n.id === edge.source);
        const targetSimNode = simulationNodes.find(n => n.id === edge.target);
        
        const sourcePos = [
          sourceSimNode?.x || sourceNode.x || 0,
          sourceSimNode?.y || sourceNode.y || 0,
          sourceSimNode?.z || sourceNode.z || 0
        ];
        
        const targetPos = [
          targetSimNode?.x || targetNode.x || 0,
          targetSimNode?.y || targetNode.y || 0,
          targetSimNode?.z || targetNode.z || 0
        ];
        
        // Use exact same curve calculation as Synapse component
        const sourceVec = new THREE.Vector3(sourcePos[0], sourcePos[1], sourcePos[2]);
        const targetVec = new THREE.Vector3(targetPos[0], targetPos[1], targetPos[2]);
        
        // Calculate midpoint with slight Y offset for natural curve (same as Synapse)
        const midVec = new THREE.Vector3()
          .addVectors(sourceVec, targetVec)
          .multiplyScalar(0.5);
        
        // Add slight upward curve (same as Synapse)
        const distance = sourceVec.distanceTo(targetVec);
        midVec.y += distance * 0.15; // 15% of distance as curve height
        
        // Create curved path (same as Synapse)
        const curve = new THREE.CatmullRomCurve3([sourceVec, midVec, targetVec]);
        curves.set(edge.id, curve);
      }
    });
    
    return curves;
  }, [edges, nodes, simulationNodes]);
  
  // Update pulse positions
  useFrame((state, delta) => {
    tickPulses(delta);
  });
  
  return (
    <group>
      {pulses.map(pulse => {
        const curve = edgeCurves.get(pulse.edgeId);
        if (!curve) return null;
        
        // Determine color based on pulse type
        const color = pulse.cascaded ? '#ff6600' : '#4ec5ff';
        
        return (
          <MultiPulseStreak
            key={pulse.edgeId}
            curve={curve}
            color={color}
            thickness={0.008}
          />
        );
      })}
    </group>
  );
}
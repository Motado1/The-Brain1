import * as d3 from 'd3-force';
import * as THREE from 'three';

export interface SimulationNode {
  id: string;
  parentId?: string;
  is_pillar: boolean;
  size: number;
  entity_type: string;
  x?: number;
  y?: number;
  z?: number;
  pos?: THREE.Vector3; // Single mutable Vector3 for position
}

export interface PillarPosition {
  id: string;
  x: number;
  y: number;
  z?: number;
}

export function createNeuralOrbitSystem(
  nodes: SimulationNode[]
) {
  // Group nodes by their pillar parents
  const pillars = nodes.filter(n => n.is_pillar);
  const satellites = nodes.filter(n => !n.is_pillar);
  
  console.log(`🧠 Orbital System: ${pillars.length} pillars, ${satellites.length} satellites`);
  
  // Position each satellite in a clean orbit around its parent pillar
  const positionedNodes = nodes.map(node => {
    if (node.is_pillar) {
      // Keep pillar positions (they should already have fixed positions)
      console.log(`🏛️ Pillar: ${node.id} at (${node.x}, ${node.y})`);
      const position = new THREE.Vector3(node.x || 0, node.y || 0, node.z || 0);
      return {
        ...node,
        position,
        pos: position, // Keep for backward compatibility
        x: node.x || 0,
        y: node.y || 0,
        z: node.z || 0,
        fx: node.x, // Fix position
        fy: node.y,
        fz: node.z
      };
    } else {
      // Find parent pillar
      const parentPillar = pillars.find(p => p.id === node.parentId);
      if (!parentPillar) {
        console.warn(`⚠️ No parent pillar found for ${node.id}, placing at center`);
        const position = new THREE.Vector3(0, 0, 0);
        return { ...node, position, pos: position, x: 0, y: 0, z: 0 };
      }
      
      // Get all siblings (nodes with same parent)
      const siblings = satellites.filter(s => s.parentId === node.parentId);
      const nodeIndex = siblings.findIndex(s => s.id === node.id);
      
      // Calculate orbital position
      const totalSiblings = siblings.length;
      const angle = (nodeIndex / totalSiblings) * 2 * Math.PI;
      
      // Different orbital distances based on entity type
      const orbitRadius = getOrbitRadius(node.entity_type, totalSiblings);
      
      // Calculate position around parent pillar
      const x = (parentPillar.x || 0) + Math.cos(angle) * orbitRadius;
      const y = (parentPillar.y || 0) + Math.sin(angle) * orbitRadius;
      const z = (parentPillar.z || 0) + (Math.random() - 0.5) * 10; // Small Z variation
      
      console.log(`🛰️ ${node.entity_type} "${node.id}" orbiting ${parentPillar.id} at (${x.toFixed(1)}, ${y.toFixed(1)})`);
      
      // Create mutable Vector3 position as single source of truth
      const position = new THREE.Vector3(x, y, z);
      
      return {
        ...node,
        position,
        pos: position, // Keep for backward compatibility
        x,
        y, 
        z
      };
    }
  });

  return positionedNodes;
}

function getOrbitRadius(entityType: string, siblingCount: number): number {
  // Base radius by entity type
  const baseRadius = {
    'idea': 25,
    'project': 30, 
    'task': 20,
    'artifact': 22
  }[entityType] || 25;
  
  // Increase radius if there are many siblings to prevent overlap
  const crowdingFactor = Math.max(1, siblingCount / 4);
  
  return baseRadius * crowdingFactor;
}

// Simplified pillar positions
export function getDefaultPillarPositions(): PillarPosition[] {
  return [
    { id: 'knowledge-core', x: 0, y: 0, z: 0 },
    { id: 'idea-project-hub', x: 100, y: 0, z: 0 },
    { id: 'action-task-dashboard', x: 50, y: 86, z: 0 },
    { id: 'learning-ledger', x: -50, y: 86, z: 0 },
    { id: 'ai-assistant-layer', x: -100, y: 0, z: 0 },
    { id: 'workbench', x: -50, y: -86, z: 0 },
  ];
}

// Create a gentle orbital animation
export function createOrbitalSimulation(nodes: SimulationNode[]) {
  const simulationNodes = nodes.map(node => ({ ...node }));
  
  const simulation = d3.forceSimulation(simulationNodes as any)
    .force('collision', d3.forceCollide().radius((d: any) => 
      d.size * 5 + 2 // Smaller collision radius for tighter clusters
    ))
    .alpha(0.1) // Low energy for subtle movement
    .alphaDecay(0.01) // Slow decay for gentle motion
    .on('tick', () => {
      // Synchronize d3 positions with Vector3 position during simulation
      simulationNodes.forEach((node: any) => {
        if (node.position && node.position instanceof THREE.Vector3) {
          // Update the Vector3 directly using set() for consistency
          node.position.set(
            node.x, 
            node.y, 
            node.z || node.position.z // Keep Z if d3 doesn't modify it
          );
        }
        // Also update legacy pos reference for backward compatibility
        if (node.pos && node.pos instanceof THREE.Vector3) {
          node.pos.copy(node.position);
        }
      });
    });
    
  return simulation;
}
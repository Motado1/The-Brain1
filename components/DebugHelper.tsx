'use client';

import * as THREE from 'three';

interface DebugHelperProps {
  nodes: any[];
  edges: any[];
  simulationNodes?: any[];
}

export default function DebugHelper({ nodes, edges, simulationNodes }: DebugHelperProps) {
  console.log('🔍 Debug Helper Data:', {
    nodes: nodes.length,
    edges: edges.length,
    simulationNodes: simulationNodes?.length || 0,
    sampleNode: nodes[0],
    sampleSimNode: simulationNodes?.[0]
  });

  return (
    <group>
      {/* Show node positions as red spheres */}
      {nodes.map(node => {
        const pos = [node.x || 0, node.y || 0, node.z || 0];
        console.log(`🔴 Node ${node.id}: [${pos.join(', ')}]`);
        return (
          <mesh key={`debug-node-${node.id}`} position={pos}>
            <sphereGeometry args={[0.5, 8, 8]} />
            <meshBasicMaterial color="red" />
          </mesh>
        );
      })}
      
      {/* Show simulation node positions as yellow spheres */}
      {simulationNodes?.map(node => {
        const pos = [node.x || 0, node.y || 0, node.z || 0];
        console.log(`🟡 SimNode ${node.id}: [${pos.join(', ')}]`);
        return (
          <mesh key={`debug-sim-${node.id}`} position={pos}>
            <sphereGeometry args={[0.3, 8, 8]} />
            <meshBasicMaterial color="yellow" />
          </mesh>
        );
      })}
      
      {/* Show edge endpoints as blue/green spheres */}
      {edges.map((edge, i) => {
        const sourceNode = nodes.find(n => n.id === edge.source);
        const targetNode = nodes.find(n => n.id === edge.target);
        
        if (!sourceNode || !targetNode) {
          console.warn(`⚠️ Edge ${i}: Missing nodes`, { edge, sourceNode, targetNode });
          return null;
        }
        
        const sourcePos = [sourceNode.x || 0, sourceNode.y || 0, sourceNode.z || 0];
        const targetPos = [targetNode.x || 0, targetNode.y || 0, targetNode.z || 0];
        
        console.log(`🔵➡️🟢 Edge ${i}: ${sourcePos.join(',')} → ${targetPos.join(',')}`);
        
        return (
          <group key={`debug-edge-${i}`}>
            {/* Source point - blue */}
            <mesh position={sourcePos}>
              <sphereGeometry args={[0.2, 8, 8]} />
              <meshBasicMaterial color="blue" />
            </mesh>
            {/* Target point - green */}
            <mesh position={targetPos}>
              <sphereGeometry args={[0.2, 8, 8]} />
              <meshBasicMaterial color="green" />
            </mesh>
            {/* Debug line between them */}
            <line>
              <bufferGeometry>
                <bufferAttribute
                  attach="attributes-position"
                  count={2}
                  array={new Float32Array([
                    sourcePos[0], sourcePos[1], sourcePos[2],
                    targetPos[0], targetPos[1], targetPos[2]
                  ])}
                  itemSize={3}
                />
              </bufferGeometry>
              <lineBasicMaterial color="white" />
            </line>
          </group>
        );
      })}
    </group>
  );
}
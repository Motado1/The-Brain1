"use client";
import { Canvas, useThree } from '@react-three/fiber';
import { OrbitControls, Stats } from '@react-three/drei';
import { useBrainStore } from '@/lib/store';
import { supabase } from '@/lib/supabaseClient';
import { mockNodes, mockEdges } from '@/lib/mockData';
import { startRealTimeSync, stopRealTimeSync } from '@/lib/realtime';
import { useEffect, useRef, useState, useMemo } from 'react';
import * as THREE from 'three';
import { EdgeData } from '@/lib/types';
import Node from './Node';
import Edge from './Edge';
import PillarConnection from './PillarConnection';
import GraphToolbar from '@/components/GraphToolbar';
import EdgeFormModal from '@/components/EdgeFormModal';
import VisualNode from '@/components/VisualNode';
import { Synapse } from '@/components/Synapse';
import SceneComposer from '@/components/SceneComposer';
import Starfield from '@/components/Starfield';
import Pulses from '@/components/Pulses';
import DebugHelper from '@/components/DebugHelper';
import { createNeuralOrbitSystem, createOrbitalSimulation } from '@/utils/simulation';
import { generateWeb } from '@/utils/generateWeb';
import { setGlobalAdjacencyMap } from '@/utils/adjacency';
import { Dendrites } from '@/components/Dendrites';
import { NeuroLink } from '@/components/NeuroLink';
// Import material to register it with R3F
import '@/materials/EdgePulseMaterial';

// Camera controller to handle zoom to selected node
function CameraController() {
  const { camera } = useThree();
  const controlsRef = useRef<any>();
  const selectedNodeId = useBrainStore(state => state.selectedNode);
  const nodes = useBrainStore(state => state.nodes);

  useEffect(() => {
    if (selectedNodeId && controlsRef.current) {
      const selectedNode = nodes.find(n => n.id === selectedNodeId);
      if (selectedNode && selectedNode.x !== undefined) {
        const targetPosition = new THREE.Vector3(
          selectedNode.x || 0, 
          selectedNode.y || 0, 
          selectedNode.z || 0
        );
        
        // Animate camera to look at the selected node
        const distance = 50; // Distance from the node
        const cameraPosition = targetPosition.clone().add(new THREE.Vector3(distance, distance, distance));
        
        // Smooth transition
        if (controlsRef.current.object) {
          controlsRef.current.object.position.copy(cameraPosition);
          controlsRef.current.target.copy(targetPosition);
          controlsRef.current.update();
        }
      }
    }
  }, [selectedNodeId, nodes, camera]);

  return (
    <OrbitControls 
      ref={controlsRef}
      enableDamping={true}
      dampingFactor={0.1}
      minDistance={5}
      maxDistance={500}
      enablePan={true}
      autoRotate={false}
    />
  );
}

export default function CanvasScene(props: { className?: string }) {
  const nodes = useBrainStore(state => state.nodes);
  const edges = useBrainStore(state => state.edges);
  const setNodes = useBrainStore(state => state.setNodes);
  const setEdges = useBrainStore(state => state.setEdges);
  const setSelectedNode = useBrainStore(state => state.setSelectedNode);
  const startAmbientFiring = useBrainStore(state => state.startAmbientFiring);
  
  // Edge editing state
  const [editingEdge, setEditingEdge] = useState<EdgeData | null>(null);
  const [showEdgeEditModal, setShowEdgeEditModal] = useState(false);
  
  // Simulation state
  const [simulationNodes, setSimulationNodes] = useState<any[]>([]);
  const [simulation, setSimulation] = useState<any>(null);
  const connectionsGenerated = useRef(false);
  const lastNodeCount = useRef(0);
  
  // Group nodes by their parent pillar
  const nodesByPillar = useMemo(() => {
    const groups = new Map();
    nodes.forEach(node => {
      if (!node.is_pillar && node.parent_id) {
        if (!groups.has(node.parent_id)) {
          groups.set(node.parent_id, []);
        }
        groups.get(node.parent_id).push(node);
      }
    });
    return groups;
  }, [nodes]);
  
  // Visible tube connection component - REMOVED (was causing blue lines)
  
  // Render ethereal tendril connections - REMOVED (unused function)
  
  const handleEdgeEdit = (edge: EdgeData) => {
    setEditingEdge(edge);
    setShowEdgeEditModal(true);
  };
  
  const handleEdgeEditClose = () => {
    setEditingEdge(null);
    setShowEdgeEditModal(false);
  };

  useEffect(() => {
    console.log('🚀 CanvasScene useEffect: Starting initialization...');
    // Start real-time sync
    const stopSync = startRealTimeSync();
    console.log('🔄 CanvasScene: Real-time sync started');
    
    // Fetch all visual nodes from local Supabase
    supabase.from('visual_nodes').select('*')
      .then(res => {
        if (res.error) {
          console.error('Error loading visual_nodes:', res.error);
          console.log('Falling back to mock data');
          setNodes(mockNodes);
        } else {
          console.log('Loaded', res.data?.length, 'nodes from Supabase');
          console.log('Raw node data:', res.data);
          
          // Process nodes and assign positions to those without coordinates
          // First pass: process pillars and create binary star system
          let processedNodes = res.data?.map((node, index) => {
            if (node.entity_type === 'pillar') {
              let pillarNode = {
                ...node,
                x: (node.x || 0) * 1.8,  // Reduce spread by 55% total (4 * 0.45 = 1.8)
                y: (node.y || 0) * 1.8,  // Reduce spread by 55% total
                z: (node.z || 0),
                scale: (node.scale || 2.0) * 10  // 10x the pillar size
              };
              
              // Create binary star system: move Idea Hub and Task Dashboard closer
              if (node.name === 'Idea & Project Hub' || node.name === 'Action & Task Dashboard') {
                // Find both pillars
                const ideaPillar = res.data?.find(n => n.entity_type === 'pillar' && n.name === 'Idea & Project Hub');
                const taskPillar = res.data?.find(n => n.entity_type === 'pillar' && n.name === 'Action & Task Dashboard');
                
                if (ideaPillar && taskPillar) {
                  // Calculate midpoint between original positions
                  const midX = ((ideaPillar.x || 0) + (taskPillar.x || 0)) / 2 * 1.8;
                  const midY = ((ideaPillar.y || 0) + (taskPillar.y || 0)) / 2 * 1.8;
                  const midZ = ((ideaPillar.z || 0) + (taskPillar.z || 0)) / 2;
                  
                  // Place them close together (27 units apart - 55% reduction total)
                  if (node.name === 'Idea & Project Hub') {
                    pillarNode.x = midX - 13.5;
                    pillarNode.y = midY;
                    pillarNode.z = midZ;
                  } else if (node.name === 'Action & Task Dashboard') {
                    pillarNode.x = midX + 13.5;
                    pillarNode.y = midY;
                    pillarNode.z = midZ;
                  }
                }
              }
              
              return pillarNode;
            }
            return node;
          }) || [];
          
          // Second pass: position ideas/projects around binary star center and scale them
          processedNodes = processedNodes.map((node, index) => {
            if (node.entity_type === 'idea' || node.entity_type === 'project') {
              // Scale based on type: projects 0.7x pillar size, ideas 0.5x pillar size
              const baseScale = node.entity_type === 'project' ? 7 : 5; // 0.7x or 0.5x of 10x pillar scale
              let positioned = { ...node, scale: (node.scale || 1.0) * baseScale };
              
              // Position around the binary star center if it doesn't have coordinates
              if (node.x === null || node.x === undefined) {
                // Find both binary star pillars
                const ideaPillar = processedNodes.find(n => n.entity_type === 'pillar' && n.name === 'Idea & Project Hub');
                const taskPillar = processedNodes.find(n => n.entity_type === 'pillar' && n.name === 'Action & Task Dashboard');
                
                if (ideaPillar && taskPillar) {
                  // Calculate the center point between the binary stars
                  const centerX = ((ideaPillar.x || 0) + (taskPillar.x || 0)) / 2;
                  const centerY = ((ideaPillar.y || 0) + (taskPillar.y || 0)) / 2;
                  const centerZ = ((ideaPillar.z || 0) + (taskPillar.z || 0)) / 2;
                  
                  // Get all ideas and projects that will orbit this binary center
                  const orbiterNodes = res.data?.filter(n => 
                    (n.entity_type === 'idea' || n.entity_type === 'project') && 
                    (n.x === null || n.x === undefined)
                  ) || [];
                  const nodeIndex = orbiterNodes.findIndex(n => n.id === node.id);
                  const angle = (nodeIndex / Math.max(orbiterNodes.length, 1)) * 2 * Math.PI;
                  const radius = 100; // Orbit around the binary center
                  const height = (Math.random() - 0.5) * 20;
                  
                  positioned = {
                    ...positioned,
                    x: centerX + Math.cos(angle) * radius,
                    y: centerY + Math.sin(angle) * radius,
                    z: centerZ + height
                  };
                }
              }
              return positioned;
            }
            return node;
          });
          
          // Third pass: position tasks and parented ideas around their parents
          processedNodes = processedNodes.map((node, index) => {
            // Skip pillars and orphaned ideas/projects - they're already processed
            if (node.entity_type === 'pillar' || 
                ((node.entity_type === 'idea' || node.entity_type === 'project') && !node.parent_id)) {
              return node;
            }
            
            // Scale nodes based on type
            let processedNode = { ...node };
            if (node.entity_type === 'task') {
              processedNode.scale = (node.scale || 1.0) * 2; // 0.2x of 10x pillar scale
            } else if (node.entity_type === 'idea' || node.entity_type === 'project') {
              // Scale ideas/projects with parents
              const baseScale = node.entity_type === 'project' ? 7 : 5;
              processedNode.scale = (node.scale || 1.0) * baseScale;
            }
            
            // Position nodes around their parent (tasks, parented ideas/projects)
            if (node.x === null || node.x === undefined || node.parent_id) {
              let parentNode = null;
              let radius = 80; // Default radius
              let angle, height = 0;
              
              // Find parent node
              if (node.parent_id) {
                // First try to find parent idea or project node
                parentNode = processedNodes.find(n => 
                  n.id === node.parent_id && 
                  (n.entity_type === 'idea' || n.entity_type === 'project')
                );
                if (parentNode) {
                  radius = 12; // Medium-close orbit around parent idea/project
                } else {
                  // Check for pillar parent
                  parentNode = processedNodes.find(n => 
                    n.id === node.parent_id && n.entity_type === 'pillar'
                  );
                  if (parentNode) {
                    // Different radii based on node type
                    if (node.entity_type === 'idea') {
                      radius = 50; // Ideas orbit closer to pillars
                    } else if (node.entity_type === 'project') {
                      radius = 55; // Projects orbit slightly further
                    } else {
                      radius = 40; // Tasks and other nodes
                    }
                  }
                }
              }
              
              // If no parent idea/project found, assign to appropriate pillar
              if (!parentNode) {
                switch (node.entity_type) {
                  case 'ai_persona':
                    parentNode = processedNodes.find(n => n.entity_type === 'pillar' && n.name === 'AI Assistant Layer');
                    break;
                  case 'task':
                    // Orphaned tasks go to Task Dashboard pillar
                    parentNode = processedNodes.find(n => n.entity_type === 'pillar' && n.name === 'Action & Task Dashboard');
                    radius = 60; // Larger orbit around pillar for orphaned tasks
                    break;
                  case 'artifact':
                    parentNode = processedNodes.find(n => n.entity_type === 'pillar' && n.name === 'Knowledge Core');
                    break;
                  default:
                    // Fallback to Workbench for any unassigned nodes
                    parentNode = processedNodes.find(n => n.entity_type === 'pillar' && n.name === 'Workbench');
                }
              }
              
              if (parentNode && parentNode.x !== null && parentNode.x !== undefined) {
                // Use parent's already-processed coordinates
                const parentX = parentNode.x || 0;
                const parentY = parentNode.y || 0;
                const parentZ = parentNode.z || 0;
                
                // Find all sibling nodes that should be around this parent
                const siblingNodes = res.data?.filter(n => {
                  // For any node with the same parent_id
                  if (node.parent_id && n.parent_id === node.parent_id) return true;
                  // For orphaned tasks around pillars, match by entity type and no parent
                  if (node.entity_type === 'task' && !node.parent_id && n.entity_type === 'task' && !n.parent_id) return true;
                  // For other orphaned nodes around pillars, match by entity type
                  if (node.entity_type !== 'task' && !node.parent_id && n.entity_type === node.entity_type && !n.parent_id) return true;
                  return false;
                }) || [];
                
                const nodeIndex = siblingNodes.findIndex(n => n.id === node.id);
                angle = (nodeIndex / Math.max(siblingNodes.length, 1)) * 2 * Math.PI;
                
                // Position around the parent with minimal height variation for close orbits
                height = (Math.random() - 0.5) * (radius < 30 ? 10 : 20); // Less variation for close moons
                
                return {
                  ...processedNode,
                  x: parentX + Math.cos(angle) * radius,
                  y: parentY + Math.sin(angle) * radius,
                  z: parentZ + height
                };
              } else {
                // Fallback: place in outer ring if no parent found
                radius = 250;
                const orphanNodes = res.data?.filter(n => 
                  (n.x === null || n.x === undefined) && 
                  !n.parent_id && 
                  n.entity_type !== 'ai_persona'
                ) || [];
                const nodeIndex = orphanNodes.findIndex(n => n.id === node.id);
                angle = (nodeIndex / Math.max(orphanNodes.length, 1)) * 2 * Math.PI;
                height = (Math.random() - 0.5) * 30;
                
                return {
                  ...processedNode,
                  x: Math.cos(angle) * radius,
                  y: Math.sin(angle) * radius,
                  z: height
                };
              }
            }
            return processedNode;
          }) || [];
          
          console.log('Processed nodes:', processedNodes.length);
          console.log('Processed node data:', processedNodes);
          setNodes(processedNodes);
        }
      });
    
    // Fetch all visual edges from local Supabase
    supabase.from('visual_edges').select('*')
      .then(res => {
        if (res.error) {
          console.error('Error loading visual_edges:', res.error);
          console.log('Falling back to mock data');
          setEdges(mockEdges);
          setGlobalAdjacencyMap(mockEdges); // Initialize adjacency map with mock data
        } else {
          console.log('Loaded', res.data?.length, 'edges from Supabase');
          const loadedEdges = res.data || [];
          setEdges(loadedEdges);
          setGlobalAdjacencyMap(loadedEdges); // Initialize adjacency map
        }
      });
    
    // Cleanup function to stop real-time sync when component unmounts
    return () => {
      console.log('🛑 CanvasScene: Cleaning up real-time sync');
      stopSync();
    };
  }, [setNodes, setEdges]);
  
  // Initialize neural orbit system when nodes change
  useEffect(() => {
    if (nodes.length > 0) {
      const simNodes = nodes.map(node => ({
        id: node.id,
        parentId: node.parent_id,
        is_pillar: node.is_pillar || node.entity_type === 'pillar',
        size: node.size || (node.entity_type === 'pillar' ? 3.0 : 1.0),
        entity_type: node.entity_type,
        x: node.x,
        y: node.y,
        z: node.z,
        // Initialize position Vector3 if node doesn't already have one
        position: node.pos || new THREE.Vector3(node.x || 0, node.y || 0, node.z || 0)
      }));
      
      // Create clean orbital positions
      const orbitalNodes = createNeuralOrbitSystem(simNodes);
      setSimulationNodes(orbitalNodes);
      
          // Proximity connections temporarily disabled to prevent conflicts
      // TODO: Re-enable with better duplicate prevention
      // if (orbitalNodes.length > 0 && 
      //     (!connectionsGenerated.current || lastNodeCount.current !== orbitalNodes.length)) {
      //   connectionsGenerated.current = true;
      //   lastNodeCount.current = orbitalNodes.length;
      //   console.log(`🧠 Generating proximity connections for ${orbitalNodes.length} nodes`);
      //   generateProximityConnections(orbitalNodes);
      // }
      
      // Optional: Add gentle simulation for subtle movement
      const sim = createOrbitalSimulation(orbitalNodes);
      setSimulation(sim);
      
      // Update positions from gentle simulation
      sim.on('tick', () => {
        const updatedNodes = sim.nodes();
        setSimulationNodes([...updatedNodes]);
      });
      
      // Ensure edge generation happens after simulation converges
      sim.on('end', () => {
        console.log('🧠 Simulation converged, finalizing positions for edge generation');
        // Force final tick to guarantee positions are synchronized
        sim.tick();
        sim.stop();
        
        // Now positions are final, generate edges using web algorithm
        if (!connectionsGenerated.current || lastNodeCount.current !== orbitalNodes.length) {
          connectionsGenerated.current = true;
          lastNodeCount.current = orbitalNodes.length;
          console.log(`🧠 Generating web connections for ${orbitalNodes.length} nodes`);
          generateWebConnections(sim.nodes());
        }
      });
    }
  }, [nodes]);
  
  // Start ambient pulse firing to make The Brain feel alive
  useEffect(() => {
    if (edges.length > 0) {
      startAmbientFiring(1200); // Fire pulses every 1.2 seconds
    }
  }, [edges.length, startAmbientFiring]);
  
  // Generate web connections using the new algorithm
  const generateWebConnections = async (positionedNodes: any[]) => {
    // Clear existing web connections first to see new generation
    console.log('🧹 Clearing existing edges to regenerate web connections');
    try {
      // Delete all existing edges
      await fetch('/api/edges', { method: 'DELETE' });
      setEdges([]);
      setGlobalAdjacencyMap([]);
    } catch (error) {
      console.warn('Failed to clear existing edges:', error);
    }

    console.log('🕸️ Generating web connections with distance-based algorithm');
    const webEdges = generateWeb(positionedNodes, 50, 4); // maxDist=50 (increased), maxPerNode=4
    
    console.log(`🕸️ Generated ${webEdges.length} web connections`);
    
    // Create edges via API
    const createdEdges = [];
    for (const edge of webEdges) {
      try {
        const response = await fetch('/api/edges', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            source: edge.source,
            target: edge.target,
            edge_type: 'web_connection',
            strength: 0.8
          })
        });
        
        if (response.ok) {
          const createdEdge = await response.json();
          createdEdges.push(createdEdge);
        } else {
          console.warn('Failed to create web connection:', await response.text());
        }
      } catch (error) {
        console.warn('Failed to create web connection:', error);
      }
    }
    
    // Update adjacency map with newly created edges
    if (createdEdges.length > 0) {
      console.log(`🕸️ Successfully created ${createdEdges.length} web connections`);
      // Refresh edges from store and update adjacency map
      const allEdges = [...edges, ...createdEdges];
      setGlobalAdjacencyMap(allEdges);
    }
  };

  // Generate automatic neural connections based on proximity
  const generateProximityConnections = async (positionedNodes: any[]) => {
    // Early exit if we already have many edges (indicating connections were already generated)
    if (edges.length > positionedNodes.length) {
      console.log('🔄 Skipping proximity connections - already have sufficient edges');
      return;
    }
    
    const CLOSE_PROXIMITY_THRESHOLD = 15; // Very close nodes
    const MEDIUM_PROXIMITY_THRESHOLD = 35; // Medium distance nodes
    const newConnections: any[] = [];
    
    // Check each pair of nodes for proximity
    for (let i = 0; i < positionedNodes.length; i++) {
      for (let j = i + 1; j < positionedNodes.length; j++) {
        const nodeA = positionedNodes[i];
        const nodeB = positionedNodes[j];
        
        // Skip if either is a pillar (pillars don't auto-connect to everything)
        if (nodeA.is_pillar || nodeB.is_pillar) continue;
        
        // Calculate distance
        const dx = (nodeA.x || 0) - (nodeB.x || 0);
        const dy = (nodeA.y || 0) - (nodeB.y || 0);
        const dz = (nodeA.z || 0) - (nodeB.z || 0);
        const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);
        
        let shouldConnect = false;
        let connectionStrength = 0;
        let isProximityConnection = false;
        
        // Check for different types of connections
        if (nodeA.parentId === nodeB.parentId && nodeA.parentId) {
          // Same parent - strong connection
          shouldConnect = true;
          connectionStrength = 1.0;
        } else if (distance <= CLOSE_PROXIMITY_THRESHOLD) {
          // Very close proximity - medium connection
          shouldConnect = true;
          connectionStrength = 0.6;
          isProximityConnection = true;
        } else if (distance <= MEDIUM_PROXIMITY_THRESHOLD) {
          // Medium proximity - weak connection
          shouldConnect = true;
          connectionStrength = 0.3;
          isProximityConnection = true;
        }
        
        if (shouldConnect) {
          const existingConnection = edges.find(edge => 
            (edge.source === nodeA.id && edge.target === nodeB.id) ||
            (edge.source === nodeB.id && edge.target === nodeA.id)
          );
          
          // Also check if we've already added this connection in current batch
          const alreadyAdded = newConnections.find(conn =>
            (conn.source === nodeA.id && conn.target === nodeB.id) ||
            (conn.source === nodeB.id && conn.target === nodeA.id)
          );
          
          if (!existingConnection && !alreadyAdded) {
            // Determine connection type based on entity types
            const connectionType = getConnectionType(nodeA.entity_type, nodeB.entity_type);
            
            newConnections.push({
              source: nodeA.id,
              target: nodeB.id,
              edge_type: connectionType,
              strength: connectionStrength,
              is_proximity: isProximityConnection,
              opacity: isProximityConnection ? 0.4 : 1.0
            });
          }
        }
      }
    }
    
    // Create the new connections in database
    for (const connection of newConnections) {
      try {
        const response = await fetch('/api/edges', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(connection)
        });
        
        if (!response.ok && response.status !== 409) {
          // Log errors except 409 (conflict) which are expected for duplicates
          console.warn(`Failed to create proximity connection (${response.status}):`, connection);
        }
      } catch (error) {
        console.warn('Failed to create proximity connection:', error);
      }
    }
    
    if (newConnections.length > 0) {
      console.log(`🧠 Generated ${newConnections.length} proximity-based neural connections`);
    }
  };
  
  // Determine connection type based on entity types
  const getConnectionType = (typeA: string, typeB: string): string => {
    // Ideas connecting to projects
    if ((typeA === 'idea' && typeB === 'project') || (typeA === 'project' && typeB === 'idea')) {
      return 'hierarchy';
    }
    // Tasks connecting to projects  
    if ((typeA === 'task' && typeB === 'project') || (typeA === 'project' && typeB === 'task')) {
      return 'dependency';
    }
    // Ideas connecting to ideas
    if (typeA === 'idea' && typeB === 'idea') {
      return 'related';
    }
    // Artifacts connecting to anything
    if (typeA === 'artifact' || typeB === 'artifact') {
      return 'reference';
    }
    // Default
    return 'related';
  };
  
  const getEdgeColor = (edgeType?: string) => {
    switch (edgeType) {
      case 'hierarchy': return '#dc2626'; // Red
      case 'dependency': return '#ea580c'; // Orange
      case 'reference': return '#7c3aed'; // Purple
      case 'related': return '#059669'; // Green
      case 'contains': return '#0891b2'; // Cyan
      case 'supports': return '#be123c'; // Rose
      default: return '#00d8ff'; // Cyan blue
    }
  };
  
  return (
    <div className={props.className}>
      {/* Graph Toolbar */}
      <GraphToolbar className="absolute top-4 left-4 z-10" />
      
      {/* Edge Edit Modal */}
      <EdgeFormModal
        isOpen={showEdgeEditModal}
        onClose={handleEdgeEditClose}
        mode="edit"
        edge={editingEdge ? {
          id: editingEdge.id,
          sourceId: editingEdge.source,
          targetId: editingEdge.target,
          label: editingEdge.label
        } : undefined}
      />
      
      <Canvas 
        camera={{ position: [0, 0, 200], fov: 75, near: 0.1, far: 100000 }}
        style={{ width: '100%', height: '100%', background: '#0a0a0a' }}
        onPointerMissed={() => setSelectedNode(null)}
        gl={{
          antialias: true,
          toneMapping: THREE.ACESFilmicToneMapping,
          toneMappingExposure: 1.2
        }}
      >
        <SceneComposer>
        <ambientLight intensity={0.3} color="#ffffff" />
        <directionalLight position={[10, 10, 5]} intensity={0.3} />
        <pointLight position={[0, 0, 0]} intensity={0.5} color="#00ffff" />
        <CameraController />
        <Stats />
        
        {/* Cosmic starfield background */}
        <Starfield />
        
        
        {/* Pulses now integrated into NeuroLink components */}
        
        {/* Legacy connections for reference - can be removed later */}
        
        
        {/* Render nodes with neural-cosmic aesthetic */}
        {nodes.map(node => {
          const simulationNode = simulationNodes.find(n => n.id === node.id);
          const isPillar = node.is_pillar || node.entity_type === 'pillar';
          
          return (
            <group key={node.id}>
              <VisualNode 
                node={{
                  ...node,
                  is_pillar: isPillar,
                  size: node.size || (isPillar ? 3.0 : 1.0)
                }}
                simulationPosition={simulationNode ? {
                  x: simulationNode.x || node.x || 0,
                  y: simulationNode.y || node.y || 0,
                  z: simulationNode.z || node.z || 0
                } : {
                  x: node.x || 0,
                  y: node.y || 0, 
                  z: node.z || 0
                }}
                onClick={(clickedNode) => setSelectedNode(clickedNode.id)}
              />
              
            </group>
          );
        })}
        
        {/* NeuroLink connections with unified pulse system */}
        {edges.map(edge => {
          // Use simulationNodes for accurate positioning
          const sourceNode = simulationNodes.find(n => n.id === edge.source) || nodes.find(n => n.id === edge.source)
          const targetNode = simulationNodes.find(n => n.id === edge.target) || nodes.find(n => n.id === edge.target)
          
          if (!sourceNode || !targetNode) return null
          
          return (
            <NeuroLink
              key={edge.id}
              start={new THREE.Vector3(sourceNode.x || 0, sourceNode.y || 0, sourceNode.z || 0)}
              end={new THREE.Vector3(targetNode.x || 0, targetNode.y || 0, targetNode.z || 0)}
              sourceNode={sourceNode}
              color="#4488dd"
              radius={0.008}
              showParticles={false}
            />
          )
        })}
        
        </SceneComposer>
      </Canvas>
    </div>
  );
}
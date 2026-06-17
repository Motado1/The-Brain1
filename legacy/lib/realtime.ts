import { supabase } from './supabaseClient';
import { useBrainStore } from './store';
import { NodeData, EdgeData } from './types';

type RealtimePayload = {
  eventType: 'INSERT' | 'UPDATE' | 'DELETE';
  new: any;
  old: any;
  errors: any;
};

let nodeSubscription: any = null;
let edgeSubscription: any = null;
let pollingInterval: NodeJS.Timeout | null = null;
let lastFetchTime: Date = new Date();

export const startRealTimeSync = () => {
  console.log('🔄 Starting real-time sync...');
  console.log('📊 Current store state before sync:', useBrainStore.getState().isRealTimeConnected);
  
  // Clean up any existing subscriptions/polling first
  stopRealTimeSync();
  
  // Try WebSocket real-time first, fallback to polling
  attemptRealtimeConnection();

  return () => {
    stopRealTimeSync();
  };
};

const attemptRealtimeConnection = () => {
  console.log('🔄 Attempting WebSocket real-time connection...');
  
  // Subscribe to visual_nodes changes
  nodeSubscription = supabase
    .channel('visual_nodes_changes')
    .on(
      'postgres_changes',
      { 
        event: '*', 
        schema: 'public', 
        table: 'visual_nodes' 
      },
      (payload: RealtimePayload) => {
        console.log('📡 Node change received:', payload);
        handleNodeChange(payload);
      }
    )
    .subscribe((status) => {
      console.log('📡 Node subscription status:', status);
      if (status === 'SUBSCRIBED') {
        console.log('✅ WebSocket real-time connected');
        useBrainStore.getState().setRealTimeConnected(true);
        useBrainStore.getState().setRealTimeMode('websocket');
      } else if (status === 'CHANNEL_ERROR') {
        console.error('❌ Node subscription error - falling back to polling');
        fallbackToPolling();
      } else if (status === 'TIMED_OUT') {
        console.error('⏰ Node subscription timed out - falling back to polling');
        fallbackToPolling();
      } else if (status === 'CLOSED') {
        console.log('🔒 Node subscription closed');
        useBrainStore.getState().setRealTimeConnected(false);
        useBrainStore.getState().setRealTimeMode('disconnected');
      }
    });

  // Subscribe to visual_edges changes
  edgeSubscription = supabase
    .channel('visual_edges_changes')
    .on(
      'postgres_changes',
      { 
        event: '*', 
        schema: 'public', 
        table: 'visual_edges' 
      },
      (payload: RealtimePayload) => {
        console.log('📡 Edge change received:', payload);
        handleEdgeChange(payload);
      }
    )
    .subscribe((status) => {
      console.log('📡 Edge subscription status:', status);
      if (status === 'CHANNEL_ERROR' || status === 'TIMED_OUT') {
        console.error('❌ Edge subscription failed');
      }
    });

  // Set a timeout to fallback to polling if WebSocket doesn't connect
  setTimeout(() => {
    if (!useBrainStore.getState().isRealTimeConnected) {
      console.log('⏰ WebSocket timeout - falling back to polling');
      fallbackToPolling();
    }
  }, 5000);
};

const fallbackToPolling = () => {
  console.log('🔄 Starting polling fallback...');
  
  // Clean up WebSocket subscriptions
  if (nodeSubscription) {
    supabase.removeChannel(nodeSubscription);
    nodeSubscription = null;
  }
  if (edgeSubscription) {
    supabase.removeChannel(edgeSubscription);
    edgeSubscription = null;
  }
  
  // Start polling for changes
  lastFetchTime = new Date();
  pollingInterval = setInterval(pollForChanges, 3000); // Poll every 3 seconds
  
  // Set connected state
  useBrainStore.getState().setRealTimeConnected(true);
  useBrainStore.getState().setRealTimeMode('polling');
  console.log('✅ Polling fallback active');
};

const pollForChanges = async () => {
  try {
    // Fetch nodes updated since last check
    const { data: newNodes, error: nodesError } = await supabase
      .from('visual_nodes')
      .select('*')
      .gte('updated_at', lastFetchTime.toISOString())
      .order('updated_at', { ascending: true });

    if (nodesError) {
      console.error('Polling error for nodes:', nodesError);
      return;
    }

    // Fetch edges updated since last check
    const { data: newEdges, error: edgesError } = await supabase
      .from('visual_edges')
      .select('*')
      .gte('created_at', lastFetchTime.toISOString())
      .order('created_at', { ascending: true });

    if (edgesError) {
      console.error('Polling error for edges:', edgesError);
      return;
    }

    // Process new/updated nodes
    if (newNodes && newNodes.length > 0) {
      console.log(`📊 Polling found ${newNodes.length} node changes`);
      newNodes.forEach(node => {
        const existing = useBrainStore.getState().nodes.find(n => n.id === node.id);
        if (existing) {
          // Update existing node
          useBrainStore.getState().updateNode(node.id, node);
        } else {
          // Add new node
          useBrainStore.getState().addNode(processNewNode(node));
        }
      });
    }

    // Process new edges
    if (newEdges && newEdges.length > 0) {
      console.log(`📊 Polling found ${newEdges.length} edge changes`);
      newEdges.forEach(edge => {
        const existing = useBrainStore.getState().edges.find(e => e.id === edge.id);
        if (!existing) {
          useBrainStore.getState().addEdge(edge);
        }
      });
    }

    // Update last fetch time
    lastFetchTime = new Date();
  } catch (error) {
    console.error('Polling error:', error);
  }
};

export const stopRealTimeSync = () => {
  console.log('🛑 Stopping real-time sync...');
  console.log('📊 Store state before cleanup:', useBrainStore.getState().isRealTimeConnected);
  
  // Clean up WebSocket subscriptions
  if (nodeSubscription) {
    supabase.removeChannel(nodeSubscription);
    nodeSubscription = null;
  }
  
  if (edgeSubscription) {
    supabase.removeChannel(edgeSubscription);
    edgeSubscription = null;
  }
  
  // Clean up polling
  if (pollingInterval) {
    clearInterval(pollingInterval);
    pollingInterval = null;
  }
  
  useBrainStore.getState().setRealTimeConnected(false);
  useBrainStore.getState().setRealTimeMode('disconnected');
};

const handleNodeChange = (payload: RealtimePayload) => {
  const store = useBrainStore.getState();
  
  switch (payload.eventType) {
    case 'INSERT':
      if (payload.new) {
        console.log('➕ Adding new node:', payload.new.name);
        // Process the new node with positioning logic (similar to CanvasScene)
        const processedNode = processNewNode(payload.new as NodeData);
        store.addNode(processedNode);
        
        // Show a subtle notification
        showNodeNotification('added', payload.new.name);
      }
      break;
      
    case 'UPDATE':
      if (payload.new) {
        console.log('✏️ Updating node:', payload.new.name);
        store.updateNode(payload.new.id, payload.new as Partial<NodeData>);
        
        // Show a subtle notification
        showNodeNotification('updated', payload.new.name);
      }
      break;
      
    case 'DELETE':
      if (payload.old) {
        console.log('🗑️ Removing node:', payload.old.name);
        store.removeNode(payload.old.id);
        
        // Show a subtle notification
        showNodeNotification('removed', payload.old.name);
      }
      break;
  }
};

const handleEdgeChange = (payload: RealtimePayload) => {
  const store = useBrainStore.getState();
  
  switch (payload.eventType) {
    case 'INSERT':
      if (payload.new) {
        console.log('➕ Adding new edge');
        store.addEdge(payload.new as EdgeData);
      }
      break;
      
    case 'UPDATE':
      if (payload.new) {
        console.log('✏️ Updating edge');
        store.updateEdge(payload.new.id, payload.new as Partial<EdgeData>);
      }
      break;
      
    case 'DELETE':
      if (payload.old) {
        console.log('🗑️ Removing edge');
        store.removeEdge(payload.old.id);
      }
      break;
  }
};

// Process new nodes with the same positioning logic as CanvasScene
const processNewNode = (node: NodeData): NodeData => {
  const store = useBrainStore.getState();
  
  // Apply size scaling based on entity type
  let processedNode = { ...node };
  
  if (node.entity_type === 'pillar') {
    processedNode.scale = (node.scale || 2.0) * 10; // 10x pillar size
    processedNode.x = (node.x || 0) * 4; // 4x spread
    processedNode.y = (node.y || 0) * 4; // 4x spread
  } else if (node.entity_type === 'idea' || node.entity_type === 'project') {
    const baseScale = node.entity_type === 'project' ? 7 : 5; // 0.7x or 0.5x pillar size
    processedNode.scale = (node.scale || 1.0) * baseScale;
    
    // Position around binary center if no coordinates
    if (node.x === null || node.x === undefined) {
      const ideaPillar = store.nodes.find(n => n.entity_type === 'pillar' && n.name === 'Idea & Project Hub');
      const taskPillar = store.nodes.find(n => n.entity_type === 'pillar' && n.name === 'Action & Task Dashboard');
      
      if (ideaPillar && taskPillar) {
        const centerX = ((ideaPillar.x || 0) + (taskPillar.x || 0)) / 2;
        const centerY = ((ideaPillar.y || 0) + (taskPillar.y || 0)) / 2;
        const centerZ = ((ideaPillar.z || 0) + (taskPillar.z || 0)) / 2;
        
        // Random position around binary center
        const angle = Math.random() * 2 * Math.PI;
        const radius = 100;
        const height = (Math.random() - 0.5) * 20;
        
        processedNode.x = centerX + Math.cos(angle) * radius;
        processedNode.y = centerY + Math.sin(angle) * radius;
        processedNode.z = centerZ + height;
      }
    }
  } else if (node.entity_type === 'task') {
    processedNode.scale = (node.scale || 1.0) * 2; // 0.2x pillar size
    
    // Position around parent idea/project if no coordinates
    if (node.x === null || node.x === undefined && node.parent_id) {
      const parentNode = store.nodes.find(n => n.id === node.parent_id);
      if (parentNode) {
        const angle = Math.random() * 2 * Math.PI;
        const radius = 12; // Close orbit
        const height = (Math.random() - 0.5) * 10;
        
        processedNode.x = (parentNode.x || 0) + Math.cos(angle) * radius;
        processedNode.y = (parentNode.y || 0) + Math.sin(angle) * radius;
        processedNode.z = (parentNode.z || 0) + height;
      }
    }
  }
  
  return processedNode;
};

// Simple notification system (can be enhanced with toast library)
const showNodeNotification = (action: 'added' | 'updated' | 'removed', nodeName: string) => {
  const emoji = action === 'added' ? '➕' : action === 'updated' ? '✏️' : '🗑️';
  console.log(`${emoji} Node ${action}: ${nodeName}`);
  
  // You could integrate with a toast notification library here
  // For now, we'll just log to console and could show in UI status bar
};
import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { NodeData, EdgeData, KnowledgeItem } from './types';

type Pulse = {
  id: string;
  edgeId: string;
  t: number;
  speed: number;
  color: string;
};

interface BrainState {
  nodes: NodeData[];
  edges: EdgeData[];
  selectedNode: string | null;
  isRealTimeConnected: boolean;
  realTimeMode: 'websocket' | 'polling' | 'disconnected';
  knowledgeItems: KnowledgeItem[];
  loading: boolean;
  error: string | null;
  pulses: Pulse[];
  _ambientTimer?: NodeJS.Timeout;
  
  // Batch operations
  setNodes: (nodes: NodeData[]) => void;
  setEdges: (edges: EdgeData[]) => void;
  setSelectedNode: (nodeId: string | null) => void;
  setRealTimeConnected: (connected: boolean) => void;
  setRealTimeMode: (mode: 'websocket' | 'polling' | 'disconnected') => void;
  
  // Real-time operations for nodes
  addNode: (node: NodeData) => void;
  updateNode: (id: string, updates: Partial<NodeData>) => void;
  removeNode: (id: string) => void;
  
  // Real-time operations for edges
  addEdge: (edge: EdgeData) => void;
  updateEdge: (id: string, updates: Partial<EdgeData>) => void;
  removeEdge: (id: string) => void;
  
  // CRUD API operations for nodes
  fetchNodes: () => Promise<void>;
  createNode: (data: Partial<NodeData>) => Promise<NodeData | null>;
  updateNodeApi: (id: string, data: Partial<NodeData>) => Promise<NodeData | null>;
  deleteNode: (id: string) => Promise<boolean>;
  
  // CRUD API operations for edges
  fetchEdges: () => Promise<void>;
  createEdge: (data: Partial<EdgeData>) => Promise<EdgeData | null>;
  updateEdgeApi: (id: string, data: Partial<EdgeData>) => Promise<EdgeData | null>;
  deleteEdge: (id: string) => Promise<boolean>;
  
  // Knowledge management
  addKnowledgeItem: (item: KnowledgeItem) => void;
  updateKnowledgeItem: (id: string, updates: Partial<KnowledgeItem>) => void;
  removeKnowledgeItem: (id: string) => void;
  setKnowledgeItems: (items: KnowledgeItem[]) => void;
  
  // Pulse management
  addPulse: (edgeId: string, color: string) => void;
  tickPulses: (dt: number) => void;
  spawnRandomPulse: () => void;
  startAmbientFiring: (intervalMs?: number) => void;
  stopAmbientFiring: () => void;
  
  // Error handling
  clearError: () => void;
}

function generateId(): string {
  return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
}

export const useBrainStore = create<BrainState>()(
  subscribeWithSelector((set, get) => ({
    nodes: [],
    edges: [],
    selectedNode: null,
    isRealTimeConnected: false,
    realTimeMode: 'disconnected',
    knowledgeItems: [],
    loading: false,
    error: null,
    pulses: [],
    _ambientTimer: undefined,
    
    // Batch operations
    setNodes: (nodes) => set({ nodes }),
    setEdges: (edges) => set({ edges }),
    setSelectedNode: (nodeId) => set({ selectedNode: nodeId }),
    setRealTimeConnected: (connected) => set({ isRealTimeConnected: connected }),
    setRealTimeMode: (mode) => set({ realTimeMode: mode }),
    
    // Real-time node operations
    addNode: (node) => set((state) => ({
      nodes: [...state.nodes, node]
    })),
    
    updateNode: (id, updates) => set((state) => ({
      nodes: state.nodes.map(node => 
        node.id === id ? { ...node, ...updates } : node
      )
    })),
    
    removeNode: (id) => set((state) => ({
      nodes: state.nodes.filter(node => node.id !== id),
      // Clear selection if the selected node is being removed
      selectedNode: state.selectedNode === id ? null : state.selectedNode
    })),
    
    // Real-time edge operations
    addEdge: (edge) => set((state) => ({
      edges: [...state.edges, edge]
    })),
    
    updateEdge: (id, updates) => set((state) => ({
      edges: state.edges.map(edge => 
        edge.id === id ? { ...edge, ...updates } : edge
      )
    })),
    
    removeEdge: (id) => set((state) => ({
      edges: state.edges.filter(edge => edge.id !== id)
    })),

    // Knowledge management operations
    addKnowledgeItem: (item) => set((state) => ({
      knowledgeItems: [...state.knowledgeItems, item]
    })),

    updateKnowledgeItem: (id, updates) => set((state) => ({
      knowledgeItems: state.knowledgeItems.map(item =>
        item.id === id ? { ...item, ...updates } : item
      )
    })),

    removeKnowledgeItem: (id) => set((state) => ({
      knowledgeItems: state.knowledgeItems.filter(item => item.id !== id)
    })),

    setKnowledgeItems: (items) => set({ knowledgeItems: items }),

    // Pulse management
    addPulse: (edgeId: string, color: string) => set((state) => {
      // Limit pulses to prevent performance issues
      if (state.pulses.length > 200) {
        state.pulses.shift(); // Remove oldest pulse
      }
      
      const newPulse: Pulse = {
        id: generateId(),
        edgeId,
        t: 0,
        speed: (0.4 + Math.random() * 0.3) * 0.85, // 15% slower
        color
      };
      
      return {
        pulses: [...state.pulses, newPulse]
      };
    }),

    tickPulses: (dt: number) => set((state) => ({
      pulses: state.pulses
        .map(pulse => ({ ...pulse, t: pulse.t + pulse.speed * dt }))
        .filter(pulse => pulse.t <= 1) // Remove completed pulses
    })),

    spawnRandomPulse: () => {
      const { edges, addPulse, nodes } = get();
      if (!edges.length) return;
      
      // Get random edge
      const edge = edges[Math.floor(Math.random() * edges.length)];
      
      // Determine color based on source node type
      const sourceNode = nodes.find(n => n.id === edge.source);
      const color = sourceNode?.entity_type === 'idea' ? '#4ec5ff' : 
                   sourceNode?.entity_type === 'task' ? '#ff9500' : 
                   sourceNode?.entity_type === 'project' ? '#4ec5ff' : '#9e7dff';
      
      addPulse(edge.id, color);
    },

    startAmbientFiring: (intervalMs = 1200) => {
      const { _ambientTimer, spawnRandomPulse } = get();
      if (_ambientTimer) return; // Already running
      
      const timerId = setInterval(spawnRandomPulse, intervalMs);
      set({ _ambientTimer: timerId });
    },

    stopAmbientFiring: () => {
      const { _ambientTimer } = get();
      if (_ambientTimer) {
        clearInterval(_ambientTimer);
        set({ _ambientTimer: undefined });
      }
    },

    // CRUD API operations for nodes
    fetchNodes: async () => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch('/api/nodes');
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to fetch nodes');
        }
        
        set({ 
          nodes: result.data || [],
          loading: false 
        });
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to fetch nodes';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error fetching nodes:', error);
      }
    },

    createNode: async (data: Partial<NodeData>) => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch('/api/nodes', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(data),
        });
        
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to create node');
        }
        
        const newNode = result.data;
        
        set(state => ({
          nodes: [...state.nodes, newNode],
          loading: false
        }));
        
        return newNode;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to create node';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error creating node:', error);
        return null;
      }
    },

    updateNodeApi: async (id: string, data: Partial<NodeData>) => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch(`/api/nodes/${id}`, {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(data),
        });
        
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to update node');
        }
        
        const updatedNode = result.data;
        
        set(state => ({
          nodes: state.nodes.map(node => 
            node.id === id ? updatedNode : node
          ),
          loading: false
        }));
        
        return updatedNode;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to update node';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error updating node:', error);
        return null;
      }
    },

    deleteNode: async (id: string) => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch(`/api/nodes/${id}`, {
          method: 'DELETE',
        });
        
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to delete node');
        }
        
        set(state => ({
          nodes: state.nodes.filter(node => node.id !== id),
          selectedNode: state.selectedNode === id ? null : state.selectedNode,
          loading: false
        }));
        
        return true;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to delete node';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error deleting node:', error);
        return false;
      }
    },

    // CRUD API operations for edges
    fetchEdges: async () => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch('/api/edges');
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to fetch edges');
        }
        
        set({ 
          edges: result.data || [],
          loading: false 
        });
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to fetch edges';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error fetching edges:', error);
      }
    },

    createEdge: async (data: Partial<EdgeData>) => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch('/api/edges', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(data),
        });
        
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to create edge');
        }
        
        const newEdge = result.data;
        
        set(state => ({
          edges: [...state.edges, newEdge],
          loading: false
        }));
        
        return newEdge;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to create edge';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error creating edge:', error);
        return null;
      }
    },

    updateEdgeApi: async (id: string, data: Partial<EdgeData>) => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch(`/api/edges/${id}`, {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(data),
        });
        
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to update edge');
        }
        
        const updatedEdge = result.data;
        
        set(state => ({
          edges: state.edges.map(edge => 
            edge.id === id ? updatedEdge : edge
          ),
          loading: false
        }));
        
        return updatedEdge;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to update edge';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error updating edge:', error);
        return null;
      }
    },

    deleteEdge: async (id: string) => {
      set({ loading: true, error: null });
      
      try {
        const response = await fetch(`/api/edges/${id}`, {
          method: 'DELETE',
        });
        
        const result = await response.json();
        
        if (!response.ok) {
          throw new Error(result.error || 'Failed to delete edge');
        }
        
        set(state => ({
          edges: state.edges.filter(edge => edge.id !== id),
          loading: false
        }));
        
        return true;
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to delete edge';
        set({ 
          error: errorMessage,
          loading: false 
        });
        console.error('Error deleting edge:', error);
        return false;
      }
    },

    // Error handling
    clearError: () => {
      set({ error: null });
    }
  }))
);
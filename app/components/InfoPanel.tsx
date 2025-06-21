"use client";
import { useBrainStore } from '@/lib/store';
import { useState, useEffect } from 'react';
import { supabase } from '@/lib/supabaseClient';
import AddKnowledgeModal from './AddKnowledgeModal';
import NodeFormModal from '../../components/NodeFormModal';

export default function InfoPanel() {
  const selectedNodeId = useBrainStore(state => state.selectedNode);
  const nodes = useBrainStore(state => state.nodes);
  const setSelectedNode = useBrainStore(state => state.setSelectedNode);
  const isRealTimeConnected = useBrainStore(state => state.isRealTimeConnected);
  const realTimeMode = useBrainStore(state => state.realTimeMode);
  const selectedNode = nodes.find(n => n.id === selectedNodeId);
  const [showNodeList, setShowNodeList] = useState(false);
  const [showAddKnowledgeModal, setShowAddKnowledgeModal] = useState(false);
  const [showNodeFormModal, setShowNodeFormModal] = useState(false);
  const [nodeFormEntityType, setNodeFormEntityType] = useState<string>('idea');
  const [nodeFormPosition, setNodeFormPosition] = useState<{ x: number; y: number; z: number }>({ x: 0, y: 0, z: 0 });

  // Check if the selected node is the Knowledge Core pillar
  const isKnowledgeCoreSelected = selectedNode?.name === 'Knowledge Core' && selectedNode?.entity_type === 'pillar';
  
  // Debug logging
  console.log('Selected node:', selectedNode);
  console.log('Is Knowledge Core selected:', isKnowledgeCoreSelected);

  // Group nodes by type for better organization
  const nodesByType = nodes.reduce((acc, node) => {
    if (!acc[node.entity_type]) acc[node.entity_type] = [];
    acc[node.entity_type].push(node);
    return acc;
  }, {} as Record<string, typeof nodes>);

  const handleNodeClick = (nodeId: string) => {
    setSelectedNode(nodeId);
  };

  const getNodeColor = (node: any) => {
    return node.color || '#229EE6';
  };

  const handleAddNode = (entityType: string) => {
    setNodeFormEntityType(entityType);
    
    // Calculate position once when opening modal
    if (selectedNode && selectedNode.entity_type === 'pillar') {
      const angle = Math.random() * 2 * Math.PI;
      const radius = 50;
      const height = (Math.random() - 0.5) * 20;
      
      setNodeFormPosition({
        x: (selectedNode.x || 0) + Math.cos(angle) * radius,
        y: (selectedNode.y || 0) + Math.sin(angle) * radius,
        z: (selectedNode.z || 0) + height
      });
    } else {
      setNodeFormPosition({ x: 0, y: 0, z: 0 });
    }
    
    setShowNodeFormModal(true);
  };

  return (
    <div className="p-4 text-sm overflow-y-auto h-full">
      {/* Real-time connection status */}
      <div className="mb-4 flex items-center">
        <div className={`w-2 h-2 rounded-full mr-2 ${
          isRealTimeConnected ? 
            (realTimeMode === 'websocket' ? 'bg-green-500' : 'bg-yellow-500') : 
            'bg-red-500'
        }`}></div>
        <span className="text-xs text-gray-400">
          {isRealTimeConnected ? 
            (realTimeMode === 'websocket' ? 'Live sync (WebSocket)' : 
             realTimeMode === 'polling' ? 'Live sync (Polling)' : 'Live sync active') : 
            'Sync disconnected'}
        </span>
      </div>
      {selectedNode ? (
        <div>
          <h2 className="text-lg font-bold mb-2">{selectedNode.name}</h2>
          
          {isKnowledgeCoreSelected ? (
            <KnowledgeManagementPanel onAddKnowledge={() => setShowAddKnowledgeModal(true)} />
          ) : selectedNode?.entity_type === 'pillar' ? (
            <div>
              <p className="text-sm text-gray-400 mb-4">
                This is the {selectedNode.name} pillar.
              </p>
              
              {/* Pillar Action Buttons */}
              <div className="mb-4 space-y-2">
                <h4 className="text-sm font-semibold text-white">Add to this Pillar</h4>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={() => handleAddNode('idea')}
                    className="px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white text-xs rounded transition-colors"
                  >
                    💡 Add Idea
                  </button>
                  <button
                    onClick={() => handleAddNode('project')}
                    className="px-3 py-2 bg-green-600 hover:bg-green-700 text-white text-xs rounded transition-colors"
                  >
                    📋 Add Project
                  </button>
                  <button
                    onClick={() => handleAddNode('task')}
                    className="px-3 py-2 bg-yellow-600 hover:bg-yellow-700 text-white text-xs rounded transition-colors"
                  >
                    ✅ Add Task
                  </button>
                  <button
                    onClick={() => handleAddNode('artifact')}
                    className="px-3 py-2 bg-purple-600 hover:bg-purple-700 text-white text-xs rounded transition-colors"
                  >
                    📄 Add Artifact
                  </button>
                </div>
              </div>
              
              {selectedNode.name === 'Knowledge Core' && (
                <KnowledgeManagementPanel onAddKnowledge={() => setShowAddKnowledgeModal(true)} />
              )}
              <div>
                <p className="mb-1"><span className="font-semibold">Type:</span> {selectedNode.entity_type}</p>
                <p className="mb-1"><span className="font-semibold">Layer:</span> {selectedNode.layer}</p>
                <p className="mb-1"><span className="font-semibold">Position:</span> ({selectedNode.x?.toFixed(1)}, {selectedNode.y?.toFixed(1)}, {selectedNode.z?.toFixed(1)})</p>
                {selectedNode.scale && (
                  <p className="mb-1"><span className="font-semibold">Scale:</span> {selectedNode.scale}</p>
                )}
                <div className="flex items-center mb-2">
                  <span className="font-semibold mr-2">Color:</span>
                  <div 
                    className="w-4 h-4 rounded border border-gray-400"
                    style={{ backgroundColor: getNodeColor(selectedNode) }}
                  ></div>
                  <span className="ml-2 text-xs">{getNodeColor(selectedNode)}</span>
                </div>
              </div>
            </div>
          ) : (
            <div>
              <p className="mb-1"><span className="font-semibold">Type:</span> {selectedNode.entity_type}</p>
              <p className="mb-1"><span className="font-semibold">Layer:</span> {selectedNode.layer}</p>
              <p className="mb-1"><span className="font-semibold">Position:</span> ({selectedNode.x?.toFixed(1)}, {selectedNode.y?.toFixed(1)}, {selectedNode.z?.toFixed(1)})</p>
              {selectedNode.scale && (
                <p className="mb-1"><span className="font-semibold">Scale:</span> {selectedNode.scale}</p>
              )}
              <div className="flex items-center mb-2">
                <span className="font-semibold mr-2">Color:</span>
                <div 
                  className="w-4 h-4 rounded border border-gray-400"
                  style={{ backgroundColor: getNodeColor(selectedNode) }}
                ></div>
                <span className="ml-2 text-xs">{getNodeColor(selectedNode)}</span>
              </div>
            </div>
          )}
          
          <button 
            onClick={() => setSelectedNode(null)} 
            className="mt-3 px-2 py-1 bg-gray-700 rounded text-xs hover:bg-gray-600"
          >
            Clear Selection
          </button>
        </div>
      ) : (
        <div className="text-gray-400">
          <p className="mb-3"><em>Select a node to see its details here.</em></p>
          
          {/* Temporary test button */}
          <button
            onClick={() => setShowAddKnowledgeModal(true)}
            className="bg-purple-600 hover:bg-purple-700 text-white py-2 px-4 rounded-lg transition-colors"
          >
            🧪 Test Add Knowledge
          </button>
        </div>
      )}

      <div className="mt-6 border-t border-gray-600 pt-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="font-semibold">All Nodes ({nodes.length})</h3>
          <button
            onClick={() => setShowNodeList(!showNodeList)}
            className="text-xs px-2 py-1 bg-gray-700 rounded hover:bg-gray-600"
          >
            {showNodeList ? 'Hide' : 'Show'}
          </button>
        </div>
        
        {showNodeList && (
          <div className="space-y-3 max-h-96 overflow-y-auto">
            {Object.entries(nodesByType).map(([type, typeNodes]) => (
              <div key={type} className="border border-gray-600 rounded p-2">
                <h4 className="font-medium mb-2 capitalize text-xs text-gray-300">
                  {type} ({typeNodes.length})
                </h4>
                <div className="space-y-1">
                  {typeNodes.map(node => (
                    <button
                      key={node.id}
                      onClick={() => handleNodeClick(node.id)}
                      className={`w-full text-left p-1 rounded text-xs hover:bg-gray-600 transition-colors flex items-center ${
                        selectedNodeId === node.id ? 'bg-gray-600' : ''
                      }`}
                    >
                      <div 
                        className="w-3 h-3 rounded-full mr-2 border border-gray-400"
                        style={{ backgroundColor: getNodeColor(node) }}
                      ></div>
                      <span className="truncate flex-1">{node.name}</span>
                      <span className="text-gray-400 ml-1">L{node.layer}</span>
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      
      {/* Add Knowledge Modal */}
      <AddKnowledgeModal 
        isOpen={showAddKnowledgeModal} 
        onClose={() => setShowAddKnowledgeModal(false)} 
      />
      
      {/* Node Form Modal */}
      <NodeFormModal
        isOpen={showNodeFormModal}
        onClose={() => setShowNodeFormModal(false)}
        mode="create"
        entityType={nodeFormEntityType}
        parentId={selectedNode?.entity_type === 'pillar' ? selectedNode.id : undefined}
        defaultPosition={nodeFormPosition}
      />
    </div>
  );
}

// Knowledge Management Panel Component
function KnowledgeManagementPanel({ onAddKnowledge }: { onAddKnowledge: () => void }) {
  const [artifacts, setArtifacts] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState('');
  const [queryResult, setQueryResult] = useState<{ answer: string; sources: Array<{ id: string; snippet: string }> } | null>(null);
  const [querying, setQuerying] = useState(false);

  // Fetch artifacts on mount
  useEffect(() => {
    fetchArtifacts();
  }, []);

  // Real-time subscription for artifact updates
  useEffect(() => {
    console.log('🔄 Setting up real-time subscription for knowledge panel...');

    const channel = supabase
      .channel('knowledge_panel_artifacts')
      .on(
        'postgres_changes',
        {
          event: '*', // Listen to all events (INSERT, UPDATE, DELETE)
          schema: 'public',
          table: 'artifacts'
        },
        (payload) => {
          console.log('📡 Knowledge panel artifact change:', payload);
          handleArtifactChange(payload);
        }
      )
      .subscribe((status) => {
        console.log('📡 Knowledge panel subscription status:', status);
      });

    return () => {
      console.log('🛑 Cleaning up knowledge panel subscription');
      supabase.removeChannel(channel);
    };
  }, []);

  const handleArtifactChange = (payload: any) => {
    const { eventType, new: newRecord, old: oldRecord } = payload;

    setArtifacts(prev => {
      switch (eventType) {
        case 'INSERT':
          console.log('➕ New artifact added:', newRecord.name);
          return [newRecord, ...prev];
          
        case 'UPDATE':
          console.log('📝 Artifact updated:', newRecord.name, 'Status:', newRecord.status);
          return prev.map(artifact => 
            artifact.id === newRecord.id ? newRecord : artifact
          );
          
        case 'DELETE':
          console.log('🗑️ Artifact deleted:', oldRecord.name);
          return prev.filter(artifact => artifact.id !== oldRecord.id);
          
        default:
          return prev;
      }
    });
  };

  const fetchArtifacts = async () => {
    try {
      const response = await fetch('/api/artifacts');
      if (response.ok) {
        const data = await response.json();
        setArtifacts(data.data || []);
      }
    } catch (error) {
      console.error('Failed to fetch artifacts:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleQuery = async () => {
    if (!query.trim() || querying) return;
    
    setQuerying(true);
    setQueryResult(null);
    
    try {
      const response = await fetch('/api/query', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ question: query.trim() }),
      });
      
      const result = await response.json();
      
      if (response.ok) {
        setQueryResult(result);
      } else {
        console.error('Query failed:', result.error);
        setQueryResult({
          answer: `Error: ${result.error}`,
          sources: []
        });
      }
    } catch (error) {
      console.error('Query request failed:', error);
      setQueryResult({
        answer: 'Failed to process query. Please try again.',
        sources: []
      });
    } finally {
      setQuerying(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleQuery();
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'indexed': return 'text-green-400';
      case 'processing': return 'text-yellow-400';
      case 'failed': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'indexed': return '✅';
      case 'processing': return '⚙️';
      case 'failed': return '❌';
      default: return '📄';
    }
  };

  return (
    <div>
      <p className="text-sm text-gray-400 mb-4">
        Manage your knowledge repository. Upload documents, notes, and files to make them searchable in The Brain.
      </p>
      
      {/* Add Knowledge Button */}
      <button
        onClick={onAddKnowledge}
        className="w-full bg-purple-600 hover:bg-purple-700 text-white py-2 px-4 rounded-lg mb-4 transition-colors flex items-center justify-center space-x-2"
      >
        <span>📁</span>
        <span>Add Knowledge</span>
      </button>

      {/* Query Interface */}
      <div className="mb-4 p-3 bg-gray-700 rounded-lg">
        <h4 className="text-sm font-semibold text-white mb-2">🔍 Ask The Brain</h4>
        <div className="flex space-x-2">
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="Ask a question about your knowledge..."
            className="flex-1 px-3 py-2 text-sm bg-gray-600 text-white rounded border border-gray-500 focus:outline-none focus:border-blue-400"
            disabled={querying}
          />
          <button
            onClick={handleQuery}
            disabled={!query.trim() || querying}
            className="px-3 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded text-sm transition-colors"
          >
            {querying ? '⏳' : '🔍'}
          </button>
        </div>
        
        {queryResult && (
          <div className="mt-3 p-3 bg-gray-600 rounded text-sm">
            <div className="text-white mb-2">
              <strong>Answer:</strong>
            </div>
            <div className="text-gray-200 mb-3 leading-relaxed">
              {queryResult.answer}
            </div>
            
            {queryResult.sources.length > 0 && (
              <div>
                <div className="text-white mb-2">
                  <strong>Sources ({queryResult.sources.length}):</strong>
                </div>
                <div className="space-y-1 max-h-32 overflow-y-auto">
                  {queryResult.sources.map((source, index) => (
                    <div key={source.id} className="text-xs text-gray-300 bg-gray-700 p-2 rounded">
                      <div className="font-medium">#{index + 1}</div>
                      <div className="truncate">{source.snippet}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Knowledge Statistics */}
      <div className="grid grid-cols-2 gap-2 mb-4">
        <div className="bg-gray-700 rounded p-2 text-center">
          <div className="text-lg font-bold text-white">{artifacts.length}</div>
          <div className="text-xs text-gray-400">Total Items</div>
        </div>
        <div className="bg-gray-700 rounded p-2 text-center">
          <div className="text-lg font-bold text-green-400">
            {artifacts.filter(a => a.status === 'indexed').length}
          </div>
          <div className="text-xs text-gray-400">Indexed</div>
        </div>
      </div>

      {/* Recent Knowledge Items */}
      <div className="mb-4">
        <h4 className="text-sm font-semibold text-white mb-2">Recent Knowledge</h4>
        {loading ? (
          <div className="text-xs text-gray-400">Loading...</div>
        ) : artifacts.length === 0 ? (
          <div className="text-xs text-gray-400 italic">
            No knowledge items yet. Click "Add Knowledge" to get started!
          </div>
        ) : (
          <div className="space-y-2 max-h-40 overflow-y-auto">
            {artifacts.slice(0, 5).map((artifact) => (
              <div key={artifact.id} className="bg-gray-700 rounded p-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2 flex-1 min-w-0">
                    <span className="text-xs">{getStatusIcon(artifact.status)}</span>
                    <span className="text-xs text-white truncate">{artifact.name}</span>
                  </div>
                  <span className={`text-xs ${getStatusColor(artifact.status)}`}>
                    {artifact.status}
                  </span>
                </div>
                <div className="text-xs text-gray-400 mt-1">
                  {artifact.type} • {new Date(artifact.created_at).toLocaleDateString()}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Quick Actions */}
      <div className="space-y-2">
        <button
          onClick={fetchArtifacts}
          className="w-full text-xs bg-gray-700 hover:bg-gray-600 text-white py-1 px-2 rounded transition-colors"
        >
          🔄 Refresh
        </button>
      </div>
    </div>
  );
}
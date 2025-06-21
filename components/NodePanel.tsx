'use client';

import { useGraphStore } from '../stores/graph';
import { useBrainStore } from '../lib/store';

export default function NodePanel() {
  const selectedNodeId = useGraphStore(s => s.selectedNodeId);
  const nodes = useBrainStore(s => s.nodes);
  
  const selectedNode = selectedNodeId ? nodes.find(n => n.id === selectedNodeId) : null;
  
  if (!selectedNode) {
    return null;
  }
  
  return (
    <div className="absolute top-4 right-4 bg-black/80 text-white p-4 rounded-lg max-w-sm">
      <h3 className="font-bold text-lg mb-2">{selectedNode.name}</h3>
      <p className="text-sm opacity-80 mb-1">Type: {selectedNode.entity_type}</p>
      {selectedNode.entity_id && (
        <p className="text-xs opacity-60">ID: {selectedNode.entity_id}</p>
      )}
    </div>
  );
}
'use client';

import { useState, useEffect } from 'react';
import { useBrainStore } from '@/lib/store';
import { NodeData } from '@/lib/types';

interface ParentNodeSelectorProps {
  currentParentId?: string;
  onParentChange: (parentId: string | null) => void;
  entityType: string;
}

export default function ParentNodeSelector({ 
  currentParentId, 
  onParentChange, 
  entityType 
}: ParentNodeSelectorProps) {
  const nodes = useBrainStore(state => state.nodes);
  const [selectedParentId, setSelectedParentId] = useState<string>(currentParentId || '');

  // Filter potential parent nodes based on entity type
  const getPotentialParents = () => {
    switch (entityType) {
      case 'idea':
        // Ideas can have pillar parents or other ideas as parents
        return nodes.filter(node => 
          node.entity_type === 'pillar' || 
          node.entity_type === 'idea'
        );
      case 'project':
        // Projects can have pillar parents or idea parents
        return nodes.filter(node => 
          node.entity_type === 'pillar' || 
          node.entity_type === 'idea'
        );
      case 'task':
        // Tasks can have project or idea parents
        return nodes.filter(node => 
          node.entity_type === 'project' || 
          node.entity_type === 'idea'
        );
      case 'artifact':
        // Artifacts can have any type as parent
        return nodes.filter(node => 
          node.entity_type === 'pillar' || 
          node.entity_type === 'idea' || 
          node.entity_type === 'project'
        );
      default:
        return nodes.filter(node => node.entity_type === 'pillar');
    }
  };

  const potentialParents = getPotentialParents();

  useEffect(() => {
    setSelectedParentId(currentParentId || '');
  }, [currentParentId]);

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    setSelectedParentId(value);
    onParentChange(value || null);
  };

  return (
    <div>
      <select
        value={selectedParentId}
        onChange={handleChange}
        className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
      >
        <option value="">No parent (top-level)</option>
        {potentialParents.map(node => (
          <option key={node.id} value={node.id}>
            {node.name} ({node.entity_type})
          </option>
        ))}
      </select>
      
      {/* Show context info */}
      <div className="mt-1 text-xs text-gray-500">
        {entityType === 'idea' && 'Ideas can be parented to pillars or other ideas'}
        {entityType === 'project' && 'Projects can be parented to pillars or ideas'}
        {entityType === 'task' && 'Tasks can be parented to projects or ideas'}
        {entityType === 'artifact' && 'Artifacts can be parented to pillars, ideas, or projects'}
      </div>
    </div>
  );
}
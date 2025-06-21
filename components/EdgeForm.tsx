'use client';

import { useState, useEffect } from 'react';
import { useBrainStore } from '../lib/store';
import { EdgeData } from '../lib/types';

interface EdgeFormProps {
  mode: 'create' | 'edit';
  edge?: {
    id: string;
    sourceId: string;
    targetId: string;
    label?: string;
  };
  onSave: (data: Partial<EdgeData>) => void;
  onDelete?: (id: string) => void;
  onCancel: () => void;
  loading?: boolean;
}

export default function EdgeForm({ 
  mode, 
  edge, 
  onSave, 
  onDelete, 
  onCancel, 
  loading = false 
}: EdgeFormProps) {
  const nodes = useBrainStore(state => state.nodes);
  
  const [formData, setFormData] = useState({
    source: edge?.sourceId || '',
    target: edge?.targetId || '',
    edge_type: edge?.label || 'connection',
    strength: 1.0,
  });

  useEffect(() => {
    if (mode === 'edit' && edge) {
      setFormData({
        source: edge.sourceId || '',
        target: edge.targetId || '',
        edge_type: edge.label || 'connection',
        strength: 1.0,
      });
    }
  }, [mode, edge]);

  const handleInputChange = (field: string, value: any) => {
    setFormData(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!formData.source || !formData.target) {
      return;
    }

    if (formData.source === formData.target) {
      return;
    }

    const finalData = {
      ...formData,
      ...(mode === 'edit' && edge && { id: edge.id })
    };

    onSave(finalData);
  };

  const handleDelete = () => {
    if (mode === 'edit' && edge && onDelete) {
      if (confirm('Are you sure you want to delete this connection?')) {
        onDelete(edge.id);
      }
    }
  };

  const getNodeDisplayName = (nodeId: string) => {
    const node = nodes.find(n => n.id === nodeId);
    if (!node) return nodeId;
    
    const typeLabel = node.entity_type.charAt(0).toUpperCase() + node.entity_type.slice(1);
    return `${node.name} (${typeLabel})`;
  };

  const getNodeIcon = (nodeType: string) => {
    switch (nodeType) {
      case 'pillar': return '🏛️';
      case 'idea': return '💡';
      case 'project': return '📋';
      case 'task': return '✅';
      case 'artifact': return '📄';
      default: return '⚪';
    }
  };

  const sourceOptions = nodes;
  const targetOptions = nodes.filter(node => node.id !== formData.source);

  return (
    <div className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-4">
        {mode === 'create' ? 'Create Connection' : 'Edit Connection'}
      </h3>
      
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Source Node */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Source Node *
          </label>
          <select
            value={formData.source}
            onChange={(e) => handleInputChange('source', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            required
            disabled={loading}
          >
            <option value="">Select source node...</option>
            {sourceOptions.map((node) => (
              <option key={node.id} value={node.id}>
                {getNodeIcon(node.entity_type)} {getNodeDisplayName(node.id)}
              </option>
            ))}
          </select>
        </div>

        {/* Target Node */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Target Node *
          </label>
          <select
            value={formData.target}
            onChange={(e) => handleInputChange('target', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            required
            disabled={loading}
          >
            <option value="">Select target node...</option>
            {targetOptions.map((node) => (
              <option key={node.id} value={node.id}>
                {getNodeIcon(node.entity_type)} {getNodeDisplayName(node.id)}
              </option>
            ))}
          </select>
        </div>

        {/* Relationship Label */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Relationship Type
          </label>
          <select
            value={formData.edge_type}
            onChange={(e) => handleInputChange('edge_type', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            disabled={loading}
          >
            <option value="connection">Connection</option>
            <option value="hierarchy">Hierarchy</option>
            <option value="dependency">Dependency</option>
            <option value="reference">Reference</option>
            <option value="related">Related</option>
            <option value="contains">Contains</option>
            <option value="supports">Supports</option>
          </select>
        </div>

        {/* Custom Label */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Custom Label (optional)
          </label>
          <input
            type="text"
            value={formData.edge_type === 'connection' ? '' : formData.edge_type}
            onChange={(e) => handleInputChange('edge_type', e.target.value || 'connection')}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            placeholder="Enter custom relationship label..."
            disabled={loading}
          />
        </div>

        {/* Connection Strength */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Connection Strength
          </label>
          <div className="flex items-center space-x-4">
            <input
              type="range"
              min="0.1"
              max="2.0"
              step="0.1"
              value={formData.strength}
              onChange={(e) => handleInputChange('strength', parseFloat(e.target.value))}
              className="flex-1"
              disabled={loading}
            />
            <span className="text-sm text-gray-600 min-w-[3rem]">
              {formData.strength.toFixed(1)}
            </span>
          </div>
          <p className="text-xs text-gray-500 mt-1">
            Controls visual thickness and importance of the connection
          </p>
        </div>

        {/* Preview */}
        {formData.source && formData.target && (
          <div className="p-3 bg-blue-50 border border-blue-200 rounded-lg">
            <h4 className="text-sm font-medium text-blue-800 mb-2">Preview</h4>
            <div className="text-sm text-blue-700">
              <strong>{getNodeDisplayName(formData.source)}</strong>
              <span className="mx-2">→</span>
              <em>"{formData.edge_type}"</em>
              <span className="mx-2">→</span>
              <strong>{getNodeDisplayName(formData.target)}</strong>
            </div>
          </div>
        )}

        {/* Validation Messages */}
        {formData.source === formData.target && formData.source && (
          <div className="p-3 bg-red-50 border border-red-200 rounded-lg">
            <p className="text-sm text-red-700">
              Source and target nodes cannot be the same.
            </p>
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex gap-3 pt-4">
          <button
            type="submit"
            disabled={loading || !formData.source || !formData.target || formData.source === formData.target}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? 'Saving...' : mode === 'create' ? 'Create Connection' : 'Update Connection'}
          </button>
          
          {mode === 'edit' && edge && onDelete && (
            <button
              type="button"
              onClick={handleDelete}
              disabled={loading}
              className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              Delete Connection
            </button>
          )}
          
          <button
            type="button"
            onClick={onCancel}
            disabled={loading}
            className="px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
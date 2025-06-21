'use client';

import { useState, useEffect } from 'react';
import { NodeData } from '../lib/types';
import ParentNodeSelector from './ParentNodeSelector';

interface NodeFormProps {
  mode: 'create' | 'edit';
  node?: NodeData;
  onSubmit: (data: Partial<NodeData>) => void;
  onCancel: () => void;
  loading?: boolean;
  entityType?: string;
  parentId?: string;
  defaultPosition?: { x: number; y: number; z: number };
}

export default function NodeForm({ mode, node, onSubmit, onCancel, loading = false, entityType, parentId, defaultPosition }: NodeFormProps) {
  const [formData, setFormData] = useState<Partial<NodeData>>({
    name: '',
    entity_type: entityType || 'idea',
    entity_id: '',
    x: defaultPosition?.x || 0,
    y: defaultPosition?.y || 0,
    z: defaultPosition?.z || 0,
    scale: 1.0,
    color: '#3b82f6',
    parent_id: parentId || undefined,
    layer: 0,
    type: '',
    pillar: undefined,
    url: undefined,
    activity_level: 'normal',
    visual_state: undefined,
  });

  const [entityData, setEntityData] = useState<any>({
    // Common fields
    description: '',
    status: '',
    priority: 'medium',
    tags: '',
    
    // Idea-specific
    validation_notes: '',
    
    // Project-specific
    idea_id: '',
    
    // Task-specific
    title: '',
    parent_type: 'idea',
    estimated_hours: '',
    due_date: '',
  });

  useEffect(() => {
    if (mode === 'edit' && node) {
      setFormData({
        name: node.name || '',
        entity_type: node.entity_type || 'idea',
        entity_id: node.entity_id || '',
        x: node.x || 0,
        y: node.y || 0,
        z: node.z || 0,
        scale: node.scale || 1.0,
        color: node.color || '#3b82f6',
        parent_id: node.parent_id || undefined,
        layer: node.layer || 0,
        type: node.type || '',
        pillar: node.pillar || undefined,
        url: node.url || undefined,
        activity_level: node.activity_level || 'normal',
        visual_state: node.visual_state || undefined,
      });
    } else if (mode === 'create') {
      // Reset form for create mode
      setFormData({
        name: '',
        entity_type: entityType || 'idea',
        entity_id: '',
        x: defaultPosition?.x || 0,
        y: defaultPosition?.y || 0,
        z: defaultPosition?.z || 0,
        scale: 1.0,
        color: '#3b82f6',
        parent_id: parentId || undefined,
        layer: 0,
        type: '',
        pillar: undefined,
        url: undefined,
        activity_level: 'normal',
        visual_state: undefined,
      });
    }
  }, [mode, node, entityType, parentId, defaultPosition?.x, defaultPosition?.y, defaultPosition?.z]);

  const handleInputChange = (field: keyof NodeData, value: any) => {
    setFormData(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const handleEntityDataChange = (field: string, value: any) => {
    setEntityData((prev: any) => ({
      ...prev,
      [field]: value
    }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    // API will generate entity_id automatically for new nodes
    const finalData = {
      ...formData,
      // Remove entity_id for new nodes, keep it for edits
      ...(mode === 'edit' && { entity_id: formData.entity_id })
    };

    onSubmit(finalData);
  };

  const getStatusOptions = () => {
    switch (formData.entity_type) {
      case 'idea':
        return ['spark', 'validation', 'approved', 'active', 'completed', 'archived'];
      case 'project':
        return ['planning', 'active', 'completed', 'on_hold'];
      case 'task':
        return ['pending', 'in_progress', 'completed', 'blocked'];
      default:
        return ['active', 'inactive'];
    }
  };

  const renderTypeSpecificFields = () => {
    switch (formData.entity_type) {
      case 'idea':
        return (
          <>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Description
              </label>
              <textarea
                value={entityData.description}
                onChange={(e) => handleEntityDataChange('description', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                rows={3}
                placeholder="Describe your idea..."
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Validation Notes
              </label>
              <textarea
                value={entityData.validation_notes}
                onChange={(e) => handleEntityDataChange('validation_notes', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                rows={2}
                placeholder="Notes on idea validation..."
              />
            </div>
          </>
        );

      case 'project':
        return (
          <>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Description
              </label>
              <textarea
                value={entityData.description}
                onChange={(e) => handleEntityDataChange('description', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                rows={3}
                placeholder="Describe your project..."
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Parent Idea ID
              </label>
              <input
                type="text"
                value={entityData.idea_id}
                onChange={(e) => handleEntityDataChange('idea_id', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                placeholder="idea_123456789"
              />
            </div>
          </>
        );

      case 'task':
        return (
          <>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Title
              </label>
              <input
                type="text"
                value={entityData.title}
                onChange={(e) => handleEntityDataChange('title', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                placeholder="Task title..."
                required
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Description
              </label>
              <textarea
                value={entityData.description}
                onChange={(e) => handleEntityDataChange('description', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                rows={3}
                placeholder="Describe the task..."
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Parent Type
              </label>
              <select
                value={entityData.parent_type}
                onChange={(e) => handleEntityDataChange('parent_type', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
              >
                <option value="idea">Idea</option>
                <option value="project">Project</option>
              </select>
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Parent ID
              </label>
              <input
                type="text"
                value={formData.parent_id || ''}
                onChange={(e) => handleInputChange('parent_id', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                placeholder="Parent idea or project ID"
                required
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Estimated Hours
              </label>
              <input
                type="number"
                value={entityData.estimated_hours}
                onChange={(e) => handleEntityDataChange('estimated_hours', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
                placeholder="0"
                min="0"
                step="0.5"
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Due Date
              </label>
              <input
                type="date"
                value={entityData.due_date}
                onChange={(e) => handleEntityDataChange('due_date', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
              />
            </div>
          </>
        );

      default:
        return null;
    }
  };

  return (
    <div className="bg-white p-6 rounded-lg border border-gray-200 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-4">
        {mode === 'create' ? 'Create New Node' : 'Edit Node'}
      </h3>
      
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Common Fields */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Node Name *
          </label>
          <input
            type="text"
            value={formData.name}
            onChange={(e) => handleInputChange('name', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            placeholder="Enter node name..."
            required
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Entity Type *
          </label>
          <select
            value={formData.entity_type}
            onChange={(e) => handleInputChange('entity_type', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            disabled={mode === 'edit'}
          >
            <option value="idea">Idea</option>
            <option value="project">Project</option>
            <option value="task">Task</option>
            <option value="artifact">Artifact</option>
            <option value="pillar">Pillar</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Status
          </label>
          <select
            value={entityData.status}
            onChange={(e) => handleEntityDataChange('status', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
          >
            <option value="">Select status...</option>
            {getStatusOptions().map(status => (
              <option key={status} value={status}>
                {status.charAt(0).toUpperCase() + status.slice(1)}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Priority
          </label>
          <select
            value={entityData.priority}
            onChange={(e) => handleEntityDataChange('priority', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
          >
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            Tags
          </label>
          <input
            type="text"
            value={entityData.tags}
            onChange={(e) => handleEntityDataChange('tags', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            placeholder="tag1, tag2, tag3"
          />
        </div>

        {/* Parent/Dependencies Selection */}
        {formData.entity_type !== 'pillar' && (
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Parent Node (optional)
            </label>
            <ParentNodeSelector
              currentParentId={formData.parent_id}
              onParentChange={(parentId) => handleInputChange('parent_id', parentId)}
              entityType={formData.entity_type || 'idea'}
            />
          </div>
        )}

        {/* Position Fields */}
        <div className="grid grid-cols-3 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">X</label>
            <input
              type="number"
              value={formData.x}
              onChange={(e) => handleInputChange('x', parseFloat(e.target.value) || 0)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
              step="0.1"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Y</label>
            <input
              type="number"
              value={formData.y}
              onChange={(e) => handleInputChange('y', parseFloat(e.target.value) || 0)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
              step="0.1"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Z</label>
            <input
              type="number"
              value={formData.z}
              onChange={(e) => handleInputChange('z', parseFloat(e.target.value) || 0)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
              step="0.1"
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Scale</label>
            <input
              type="number"
              value={formData.scale}
              onChange={(e) => handleInputChange('scale', parseFloat(e.target.value) || 1.0)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
              min="0.1"
              max="10"
              step="0.1"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Color</label>
            <input
              type="color"
              value={formData.color}
              onChange={(e) => handleInputChange('color', e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            />
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            URL (optional)
          </label>
          <input
            type="url"
            value={formData.url || ''}
            onChange={(e) => handleInputChange('url', e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 bg-white"
            placeholder="https://..."
          />
        </div>

        {/* Type-specific Fields */}
        {renderTypeSpecificFields()}

        {/* Action Buttons */}
        <div className="flex gap-3 pt-4">
          <button
            type="submit"
            disabled={loading}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? 'Saving...' : mode === 'create' ? 'Create Node' : 'Update Node'}
          </button>
          
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
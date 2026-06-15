'use client';

import { useState } from 'react';
import Modal from './Modal';
import NodeForm from './NodeForm';
import { NodeData } from '../lib/types';
import { useBrainStore } from '../lib/store';

interface NodeFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  mode: 'create' | 'edit';
  node?: NodeData;
  entityType?: string;
  parentId?: string;
  defaultPosition?: { x: number; y: number; z: number };
}

export default function NodeFormModal({ 
  isOpen, 
  onClose, 
  mode, 
  node, 
  entityType,
  parentId,
  defaultPosition 
}: NodeFormModalProps) {
  const createNode = useBrainStore(state => state.createNode);
  const updateNodeApi = useBrainStore(state => state.updateNodeApi);
  const clearError = useBrainStore(state => state.clearError);
  const loading = useBrainStore(state => state.loading);
  const error = useBrainStore(state => state.error);

  const handleSubmit = async (data: Partial<NodeData>) => {
    // Apply defaults for new nodes
    const finalData = {
      ...data,
      entity_type: entityType || data.entity_type,
      parent_id: parentId || data.parent_id,
      x: defaultPosition?.x ?? data.x ?? 0,
      y: defaultPosition?.y ?? data.y ?? 0,
      z: defaultPosition?.z ?? data.z ?? 0,
    };

    let success = false;
    
    if (mode === 'create') {
      const result = await createNode(finalData);
      success = result !== null;
    } else if (mode === 'edit' && node) {
      const result = await updateNodeApi(node.id, finalData);
      success = result !== null;
    }
    
    if (success) {
      onClose();
    }
  };

  const handleCancel = () => {
    clearError();
    onClose();
  };

  const getModalTitle = () => {
    if (mode === 'edit') {
      return `Edit ${node?.entity_type || 'Node'}`;
    }
    
    if (entityType) {
      return `Create ${entityType.charAt(0).toUpperCase() + entityType.slice(1)}`;
    }
    
    return 'Create Node';
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleCancel}
      title={getModalTitle()}
      size="xl"
    >
      <NodeForm
        mode={mode}
        node={node}
        onSubmit={handleSubmit}
        onCancel={handleCancel}
        loading={loading}
        entityType={entityType}
        parentId={parentId}
        defaultPosition={defaultPosition}
      />
      
      {error && (
        <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-red-700 text-sm">{error}</p>
        </div>
      )}
    </Modal>
  );
}
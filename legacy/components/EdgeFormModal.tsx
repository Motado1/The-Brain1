'use client';

import { useState } from 'react';
import Modal from './Modal';
import EdgeForm from './EdgeForm';
import { EdgeData } from '../lib/types';
import { useBrainStore } from '../lib/store';

interface EdgeFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  mode: 'create' | 'edit';
  edge?: {
    id: string;
    sourceId: string;
    targetId: string;
    label?: string;
  };
  preselectedSource?: string;
  preselectedTarget?: string;
}

export default function EdgeFormModal({ 
  isOpen, 
  onClose, 
  mode, 
  edge,
  preselectedSource,
  preselectedTarget 
}: EdgeFormModalProps) {
  const createEdge = useBrainStore(state => state.createEdge);
  const updateEdgeApi = useBrainStore(state => state.updateEdgeApi);
  const deleteEdge = useBrainStore(state => state.deleteEdge);
  const clearError = useBrainStore(state => state.clearError);
  const loading = useBrainStore(state => state.loading);
  const error = useBrainStore(state => state.error);

  const handleSave = async (data: Partial<EdgeData>) => {
    let success = false;
    
    if (mode === 'create') {
      const result = await createEdge(data);
      success = result !== null;
    } else if (mode === 'edit' && edge) {
      const result = await updateEdgeApi(edge.id, data);
      success = result !== null;
    }
    
    if (success) {
      onClose();
    }
  };

  const handleDelete = async (id: string) => {
    const success = await deleteEdge(id);
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
      return 'Edit Connection';
    }
    return 'Create Connection';
  };

  // Convert edge data format if needed
  const formEdge = edge ? {
    id: edge.id,
    sourceId: edge.sourceId,
    targetId: edge.targetId,
    label: edge.label
  } : preselectedSource && preselectedTarget ? {
    id: '',
    sourceId: preselectedSource,
    targetId: preselectedTarget,
    label: undefined
  } : undefined;

  return (
    <Modal
      isOpen={isOpen}
      onClose={handleCancel}
      title={getModalTitle()}
      size="lg"
    >
      <EdgeForm
        mode={mode}
        edge={formEdge}
        onSave={handleSave}
        onDelete={mode === 'edit' ? handleDelete : undefined}
        onCancel={handleCancel}
        loading={loading}
      />
      
      {error && (
        <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-red-700 text-sm">{error}</p>
        </div>
      )}
    </Modal>
  );
}
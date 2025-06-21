'use client';

import { useState } from 'react';
import EdgeFormModal from './EdgeFormModal';

interface GraphToolbarProps {
  className?: string;
}

export default function GraphToolbar({ className = '' }: GraphToolbarProps) {
  const [showEdgeModal, setShowEdgeModal] = useState(false);

  return (
    <>
      <div className={`bg-white/90 backdrop-blur-sm border border-gray-200 rounded-lg shadow-sm p-3 ${className}`}>
        <div className="flex items-center space-x-3">
          <h3 className="text-sm font-medium text-gray-900">Graph Tools</h3>
          
          <button
            onClick={() => setShowEdgeModal(true)}
            className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-md transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1"
          >
            🔗 Add Connection
          </button>
        </div>
      </div>

      <EdgeFormModal
        isOpen={showEdgeModal}
        onClose={() => setShowEdgeModal(false)}
        mode="create"
      />
    </>
  );
}
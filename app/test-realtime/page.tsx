"use client";
import { useState } from 'react';
import { createEntityWithVisualization } from '@/lib/api';

export default function TestRealtimePage() {
  const [isCreating, setIsCreating] = useState(false);
  const [results, setResults] = useState<any[]>([]);

  const addResult = (action: string, success: boolean, data?: any) => {
    setResults(prev => [...prev, { 
      action, 
      success, 
      data, 
      timestamp: new Date().toLocaleTimeString() 
    }]);
  };

  const createTestIdea = async () => {
    setIsCreating(true);
    try {
      const randomId = Math.floor(Math.random() * 1000);
      const result = await createEntityWithVisualization.idea({
        name: `Real-time Test Idea ${randomId}`,
        description: `This idea was created to test real-time sync at ${new Date().toLocaleTimeString()}`,
        status: 'spark',
        priority: 'medium'
      });

      if (result.error) {
        addResult('Create Idea', false, result.error);
      } else {
        addResult('Create Idea', true, result.data);
      }
    } catch (error) {
      addResult('Create Idea', false, error instanceof Error ? error.message : 'Unknown error');
    }
    setIsCreating(false);
  };

  const createTestTask = async () => {
    setIsCreating(true);
    try {
      // First get the Neural Knowledge Interface idea ID
      const response = await fetch('/api/visual-nodes?entity_type=idea');
      const visualNodes = await response.json();
      const neuralKnowledgeNode = visualNodes.data?.find((n: any) => 
        n.name === 'Neural Knowledge Interface'
      );

      if (!neuralKnowledgeNode) {
        addResult('Create Task', false, 'Neural Knowledge Interface not found');
        setIsCreating(false);
        return;
      }

      const randomId = Math.floor(Math.random() * 1000);
      const result = await createEntityWithVisualization.task({
        title: `Real-time Test Task ${randomId}`,
        description: `This task was created to test real-time sync at ${new Date().toLocaleTimeString()}`,
        parent_type: 'idea',
        parent_id: neuralKnowledgeNode.entity_id,
        status: 'pending',
        priority: 'high'
      }, {
        parent_id: neuralKnowledgeNode.id // Visual parent for positioning
      });

      if (result.error) {
        addResult('Create Task', false, result.error);
      } else {
        addResult('Create Task', true, result.data);
      }
    } catch (error) {
      addResult('Create Task', false, error instanceof Error ? error.message : 'Unknown error');
    }
    setIsCreating(false);
  };

  const clearResults = () => {
    setResults([]);
  };

  return (
    <div className="min-h-screen bg-gray-900 text-white p-8">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-3xl font-bold mb-8">Real-time Sync Test</h1>
        
        <div className="mb-8 p-6 bg-gray-800 rounded-lg">
          <h2 className="text-xl font-semibold mb-4">Test Instructions</h2>
          <ol className="list-decimal list-inside space-y-2 text-gray-300">
            <li>Open the main Brain visualization in another tab: <a href="/" className="text-blue-400 underline">Go to Brain</a></li>
            <li>Click the buttons below to create new nodes</li>
            <li>Watch the main visualization update in real-time!</li>
            <li>New nodes should appear with smooth scale-in animations</li>
          </ol>
        </div>

        <div className="mb-8 space-x-4">
          <button
            onClick={createTestIdea}
            disabled={isCreating}
            className="bg-pink-600 hover:bg-pink-700 disabled:bg-gray-600 px-6 py-3 rounded-lg font-semibold"
          >
            {isCreating ? 'Creating...' : 'Create Test Idea'}
          </button>
          
          <button
            onClick={createTestTask}
            disabled={isCreating}
            className="bg-orange-600 hover:bg-orange-700 disabled:bg-gray-600 px-6 py-3 rounded-lg font-semibold"
          >
            {isCreating ? 'Creating...' : 'Create Test Task'}
          </button>
          
          <button
            onClick={clearResults}
            className="bg-gray-600 hover:bg-gray-700 px-6 py-3 rounded-lg font-semibold"
          >
            Clear Results
          </button>
        </div>

        <div className="space-y-4">
          <h3 className="text-lg font-semibold">Results ({results.length})</h3>
          {results.length === 0 ? (
            <p className="text-gray-400 italic">No actions performed yet</p>
          ) : (
            results.map((result, index) => (
              <div
                key={index}
                className={`p-4 rounded-lg border ${
                  result.success 
                    ? 'bg-green-900 border-green-600' 
                    : 'bg-red-900 border-red-600'
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <h4 className="font-semibold">{result.action}</h4>
                  <div className="flex items-center space-x-2">
                    <span className="text-xs text-gray-300">{result.timestamp}</span>
                    <span className={`px-2 py-1 rounded text-sm ${
                      result.success ? 'bg-green-600' : 'bg-red-600'
                    }`}>
                      {result.success ? 'SUCCESS' : 'FAILED'}
                    </span>
                  </div>
                </div>
                <pre className="bg-gray-800 p-2 rounded text-xs overflow-x-auto">
                  {JSON.stringify(result.data, null, 2)}
                </pre>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
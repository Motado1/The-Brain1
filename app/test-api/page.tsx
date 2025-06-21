"use client";
import { useState } from 'react';

export default function TestAPIPage() {
  const [results, setResults] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  const addResult = (test: string, success: boolean, data?: any) => {
    setResults(prev => [...prev, { test, success, data, timestamp: new Date().toISOString() }]);
  };

  const runTests = async () => {
    setLoading(true);
    setResults([]);

    try {
      // Test GET /api/ideas
      console.log('Testing GET /api/ideas...');
      const ideasResponse = await fetch('/api/ideas');
      const ideasData = await ideasResponse.json();
      addResult('GET /api/ideas', ideasResponse.ok, ideasData);

      // Test POST /api/ideas
      console.log('Testing POST /api/ideas...');
      const newIdea = {
        name: 'Test API Idea',
        description: 'This is a test idea created via API',
        status: 'spark',
        priority: 'medium'
      };
      const createResponse = await fetch('/api/ideas', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(newIdea)
      });
      const createData = await createResponse.json();
      addResult('POST /api/ideas', createResponse.ok, createData);

      if (createResponse.ok && createData.data?.id) {
        const createdId = createData.data.id;

        // Test GET /api/ideas/[id]
        console.log('Testing GET /api/ideas/[id]...');
        const getByIdResponse = await fetch(`/api/ideas/${createdId}`);
        const getByIdData = await getByIdResponse.json();
        addResult('GET /api/ideas/[id]', getByIdResponse.ok, getByIdData);

        // Test PUT /api/ideas/[id]
        console.log('Testing PUT /api/ideas/[id]...');
        const updateData = {
          name: 'Updated Test API Idea',
          status: 'active'
        };
        const updateResponse = await fetch(`/api/ideas/${createdId}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(updateData)
        });
        const updateResponseData = await updateResponse.json();
        addResult('PUT /api/ideas/[id]', updateResponse.ok, updateResponseData);

        // Test POST /api/tasks (create task for the idea)
        console.log('Testing POST /api/tasks...');
        const newTask = {
          title: 'Test API Task',
          description: 'This is a test task created via API',
          parent_type: 'idea',
          parent_id: createdId,
          status: 'pending',
          priority: 'high'
        };
        const createTaskResponse = await fetch('/api/tasks', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(newTask)
        });
        const createTaskData = await createTaskResponse.json();
        addResult('POST /api/tasks', createTaskResponse.ok, createTaskData);

        // Test GET /api/tasks
        console.log('Testing GET /api/tasks...');
        const tasksResponse = await fetch('/api/tasks');
        const tasksData = await tasksResponse.json();
        addResult('GET /api/tasks', tasksResponse.ok, tasksData);

        // Test DELETE /api/ideas/[id] (cleanup)
        console.log('Testing DELETE /api/ideas/[id]...');
        const deleteResponse = await fetch(`/api/ideas/${createdId}`, {
          method: 'DELETE'
        });
        const deleteData = await deleteResponse.json();
        addResult('DELETE /api/ideas/[id]', deleteResponse.ok, deleteData);
      }

      // Test GET /api/visual-nodes
      console.log('Testing GET /api/visual-nodes...');
      const visualNodesResponse = await fetch('/api/visual-nodes');
      const visualNodesData = await visualNodesResponse.json();
      addResult('GET /api/visual-nodes', visualNodesResponse.ok, visualNodesData);

    } catch (error) {
      addResult('Test execution', false, { 
        error: error instanceof Error ? error.message : 'Unknown error' 
      });
    }

    setLoading(false);
  };

  return (
    <div className="min-h-screen bg-gray-900 text-white p-8">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-3xl font-bold mb-8">API Test Suite</h1>
        
        <button
          onClick={runTests}
          disabled={loading}
          className="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 px-6 py-3 rounded-lg font-semibold mb-8"
        >
          {loading ? 'Running Tests...' : 'Run API Tests'}
        </button>

        <div className="space-y-4">
          {results.map((result, index) => (
            <div
              key={index}
              className={`p-4 rounded-lg border ${
                result.success 
                  ? 'bg-green-900 border-green-600' 
                  : 'bg-red-900 border-red-600'
              }`}
            >
              <div className="flex items-center justify-between mb-2">
                <h3 className="font-semibold">{result.test}</h3>
                <span className={`px-2 py-1 rounded text-sm ${
                  result.success ? 'bg-green-600' : 'bg-red-600'
                }`}>
                  {result.success ? 'PASS' : 'FAIL'}
                </span>
              </div>
              <div className="text-sm text-gray-300 mb-2">
                {result.timestamp}
              </div>
              <pre className="bg-gray-800 p-2 rounded text-xs overflow-x-auto">
                {JSON.stringify(result.data, null, 2)}
              </pre>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
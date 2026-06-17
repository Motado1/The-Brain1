// Test script to verify real-time status updates
// This simulates what happens when the RAG worker processes files

async function testRealtimeUpdates() {
  console.log('🧪 Testing real-time status updates...');
  
  // Step 1: Create a test artifact via API
  console.log('📁 Creating test artifact...');
  
  const createResponse = await fetch('http://localhost:3001/api/artifacts', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      name: 'Real-time Test Document',
      description: 'Testing real-time status updates',
      type: 'note',
      content: 'This document will test real-time status updates in The Brain upload interface.'
    })
  });

  if (!createResponse.ok) {
    console.error('❌ Failed to create test artifact');
    return;
  }

  const createResult = await createResponse.json();
  console.log('✅ Test artifact created:', createResult.artifact.id);
  
  // Step 2: Wait a moment for UI to show the artifact
  console.log('⏳ Waiting 3 seconds...');
  await new Promise(resolve => setTimeout(resolve, 3000));
  
  // Step 3: Trigger the RAG worker to process it
  console.log('🤖 Triggering RAG worker...');
  
  const workerResponse = await fetch('http://localhost:54321/functions/v1/ingestion-worker', {
    method: 'POST',
    headers: {
      'Authorization': 'Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0'
    }
  });

  const workerResult = await workerResponse.json();
  
  if (workerResponse.ok) {
    console.log('✅ RAG worker processed successfully');
    console.log('📡 Real-time update should now appear in UI!');
    console.log('👀 Check the browser - the document status should change to "indexed"');
  } else {
    console.error('❌ RAG worker failed:', workerResult);
  }
}

// Run the test
testRealtimeUpdates().catch(console.error);
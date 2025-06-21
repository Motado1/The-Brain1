// Test script to verify immediate RAG processing
// This tests that files get processed immediately upon upload instead of waiting for cron

async function testImmediateProcessing() {
  console.log('🧪 Testing Immediate RAG Processing...');
  
  try {
    // Step 1: Create a test artifact (this should trigger immediate processing)
    console.log('📁 Creating test artifact with immediate processing...');
    
    const createResponse = await fetch('http://localhost:3001/api/artifacts', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        name: 'Immediate Processing Test Document',
        description: 'Testing immediate RAG processing trigger',
        type: 'note',
        content: 'This document should be processed immediately after creation, not waiting for the cron schedule. It contains important information about immediate processing capabilities in The Brain system.'
      })
    });

    if (!createResponse.ok) {
      console.error('❌ Failed to create test artifact');
      const error = await createResponse.json();
      console.error('Error:', error);
      return;
    }

    const createResult = await createResponse.json();
    console.log('✅ Test artifact created:', createResult.artifact.id);
    console.log('📋 Job created:', createResult.job?.id || 'No job info');
    
    // Step 2: Wait a moment and check if processing started immediately
    console.log('⏳ Waiting 3 seconds to check processing status...');
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    // Step 3: Check the artifact status to see if it was processed
    console.log('🔍 Checking artifact status...');
    
    const statusResponse = await fetch(`http://localhost:3001/api/artifacts`);
    if (statusResponse.ok) {
      const statusResult = await statusResponse.json();
      const ourArtifact = statusResult.data.find(artifact => 
        artifact.id === createResult.artifact.id
      );
      
      if (ourArtifact) {
        console.log('📊 Artifact status:', ourArtifact.status);
        
        if (ourArtifact.status === 'indexed') {
          console.log('🎉 SUCCESS: Artifact was processed immediately!');
          console.log('⚡ Processing time: Less than 3 seconds (immediate)');
        } else if (ourArtifact.status === 'processing') {
          console.log('⚙️ PROCESSING: Artifact is currently being processed');
          console.log('💡 This is expected for immediate processing');
          
          // Wait a bit more and check again
          console.log('⏳ Waiting 5 more seconds...');
          await new Promise(resolve => setTimeout(resolve, 5000));
          
          const finalResponse = await fetch(`http://localhost:3001/api/artifacts`);
          if (finalResponse.ok) {
            const finalResult = await finalResponse.json();
            const finalArtifact = finalResult.data.find(artifact => 
              artifact.id === createResult.artifact.id
            );
            
            if (finalArtifact?.status === 'indexed') {
              console.log('🎉 SUCCESS: Artifact was processed within 8 seconds!');
              console.log('⚡ Immediate processing is working correctly');
            } else {
              console.log('⏰ SLOW: Artifact still processing after 8 seconds');
              console.log('💭 This might indicate cron-based processing instead of immediate');
            }
          }
        } else {
          console.log('❌ UNEXPECTED: Artifact status is', ourArtifact.status);
        }
      } else {
        console.log('❌ Artifact not found in status check');
      }
    }
    
    // Step 4: Test with a file upload to verify end-to-end immediate processing
    console.log('\n📎 Testing file upload with immediate processing...');
    
    // Create a simple text file blob
    const testFileContent = 'This is a test file for immediate processing verification. It should be processed right after upload without waiting for cron schedule.';
    
    // We'll simulate what the upload modal does
    const fileArtifactResponse = await fetch('http://localhost:3001/api/artifacts', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        name: 'Immediate Test File',
        description: 'File upload test for immediate processing',
        type: 'file',
        storagePath: 'knowledge/raw/test-immediate-file.txt',
        contentType: 'text/plain',
        fileSize: testFileContent.length,
        originalFileName: 'test-immediate.txt'
      })
    });
    
    if (fileArtifactResponse.ok) {
      const fileResult = await fileArtifactResponse.json();
      console.log('✅ File artifact created:', fileResult.artifact.id);
      console.log('⚡ Should trigger immediate processing...');
    } else {
      console.log('❌ File artifact creation failed');
    }
    
  } catch (error) {
    console.error('❌ Test failed:', error.message);
  }
}

// Test error scenarios
async function testImmediateProcessingEdgeCases() {
  console.log('\n🧪 Testing Edge Cases...');
  
  // Test 1: Invalid job ID trigger
  console.log('🔍 Testing invalid job ID trigger...');
  try {
    const response = await fetch('http://localhost:54321/functions/v1/ingestion-worker', {
      method: 'POST',
      headers: {
        'Authorization': 'Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        trigger: 'immediate',
        jobId: 'invalid-job-id-12345'
      })
    });

    const result = await response.json();
    console.log('📄 Invalid job ID response:', result.message || result.error);
    
  } catch (error) {
    console.log('❌ Edge case test failed:', error.message);
  }
}

// Run tests
console.log('🚀 Starting Immediate Processing Tests...');
testImmediateProcessing()
  .then(() => testImmediateProcessingEdgeCases())
  .then(() => console.log('\n✅ All immediate processing tests completed!'))
  .catch(console.error);
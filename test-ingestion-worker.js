// Test script for the ingestion worker
// This will call the worker function to process any pending jobs

async function testIngestionWorker() {
  const functionUrl = 'http://127.0.0.1:54321/functions/v1/ingestion-worker'
  const anonKey = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0'

  console.log('🧪 Testing ingestion worker...')

  try {
    const response = await fetch(functionUrl, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${anonKey}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ trigger: 'manual_test' })
    })

    const result = await response.json()
    
    console.log('Response status:', response.status)
    console.log('Response body:', JSON.stringify(result, null, 2))

    if (response.ok) {
      console.log('✅ Worker test completed')
    } else {
      console.log('❌ Worker returned error')
    }

  } catch (error) {
    console.error('💥 Test failed:', error.message)
  }
}

// Run the test
testIngestionWorker()
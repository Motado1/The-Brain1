// Test script for the ingestion worker
// Usage: deno run --allow-net --allow-env test.ts

const FUNCTION_URL = 'http://localhost:54321/functions/v1/ingestion-worker'

async function testWorker() {
  console.log('🧪 Testing ingestion worker...')
  
  try {
    const response = await fetch(FUNCTION_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${Deno.env.get('SUPABASE_ANON_KEY')}`,
      },
      body: JSON.stringify({
        test: true
      })
    })

    const result = await response.text()
    console.log('Response status:', response.status)
    console.log('Response body:', result)

    if (response.ok) {
      console.log('✅ Worker is responding')
    } else {
      console.log('❌ Worker error')
    }
  } catch (error) {
    console.error('💥 Test failed:', error)
  }
}

if (import.meta.main) {
  await testWorker()
}
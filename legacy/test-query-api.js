// Test script for the query API endpoint
// This tests the RAG query functionality

async function testQueryAPI() {
  console.log('🧪 Testing Query API...');
  
  const testQuestions = [
    'What is The Brain?',
    'How does the knowledge management system work?',
    'Tell me about the upload process',
    'What can I search for?'
  ];

  for (const question of testQuestions) {
    console.log(`\n📋 Testing question: "${question}"`);
    
    try {
      const response = await fetch('http://localhost:3001/api/query', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ question })
      });

      const result = await response.json();
      
      if (response.ok) {
        console.log('✅ Query successful');
        console.log('📖 Answer:', result.answer);
        console.log('📚 Sources:', result.sources.length, 'sources found');
        result.sources.forEach((source, index) => {
          console.log(`   ${index + 1}. ${source.id}: ${source.snippet.substring(0, 100)}...`);
        });
      } else {
        console.log('❌ Query failed:', result.error);
      }
      
    } catch (error) {
      console.error('❌ Request failed:', error.message);
    }
  }
}

// Test invalid requests
async function testErrorHandling() {
  console.log('\n🧪 Testing Error Handling...');
  
  const invalidRequests = [
    { test: 'Empty question', body: { question: '' } },
    { test: 'Missing question', body: {} },
    { test: 'Invalid type', body: { question: 123 } }
  ];

  for (const testCase of invalidRequests) {
    console.log(`\n🔍 Testing: ${testCase.test}`);
    
    try {
      const response = await fetch('http://localhost:3001/api/query', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(testCase.body)
      });

      const result = await response.json();
      
      if (response.status === 400) {
        console.log('✅ Correctly returned 400 error:', result.error);
      } else {
        console.log('❌ Unexpected response:', response.status, result);
      }
      
    } catch (error) {
      console.error('❌ Request failed:', error.message);
    }
  }
}

// Run tests
console.log('🚀 Starting Query API Tests...');
testQueryAPI()
  .then(() => testErrorHandling())
  .then(() => console.log('\n✅ All tests completed!'))
  .catch(console.error);
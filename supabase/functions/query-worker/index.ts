// Supabase Edge Function for RAG query processing
// Handles heavy vector operations: embeddings generation, vector search, and AI chat completion

import { serve } from 'https://deno.land/std@0.177.0/http/server.ts'
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2.38.4'

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

interface QueryRequest {
  question: string;
}

interface QueryResponse {
  answer: string;
  sources: {
    id: string;
    snippet: string;
  }[];
}

interface QdrantHit {
  id: string;
  score: number;
  payload: {
    artifact_id: string;
    name: string;
    type: string;
    text: string;
    content_preview?: string;
    url?: string;
    created_at: string;
    metadata?: Record<string, any>;
  };
}

interface QdrantSearchResponse {
  result: QdrantHit[];
}

// Environment configuration
const OLLAMA_URL = Deno.env.get('OLLAMA_URL') || 'http://localhost:11434';
const OLLAMA_EMBEDDING_MODEL = Deno.env.get('OLLAMA_EMBEDDING_MODEL') || 'nomic-embed-text';
const OLLAMA_CHAT_MODEL = Deno.env.get('OLLAMA_CHAT_MODEL') || 'llama3.2';
const QDRANT_URL = Deno.env.get('QDRANT_URL') || 'http://localhost:6333';
const QDRANT_COLLECTION = Deno.env.get('QDRANT_COLLECTION') || 'brain-knowledge';
const DEVELOPMENT_MODE = Deno.env.get('DEVELOPMENT_MODE') === 'true' || true; // Default to true for local development

serve(async (req) => {
  // Handle CORS preflight
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    console.log('🔍 Query worker started')
    console.log('📡 Request method:', req.method)
    console.log('📡 Request URL:', req.url)

    // Only accept POST requests
    if (req.method !== 'POST') {
      console.error('❌ Invalid method:', req.method)
      return new Response(
        JSON.stringify({ error: 'Method not allowed' }),
        { 
          status: 405, 
          headers: { ...corsHeaders, 'Content-Type': 'application/json' } 
        }
      )
    }

    // Parse and validate request body
    let body: QueryRequest
    try {
      body = await req.json()
      console.log('📋 Request body received:', { question: body.question?.substring(0, 100) + '...' })
    } catch (error) {
      console.error('❌ Invalid JSON body:', error)
      return new Response(
        JSON.stringify({ error: 'Invalid JSON body' }),
        { 
          status: 400, 
          headers: { ...corsHeaders, 'Content-Type': 'application/json' } 
        }
      )
    }

    // Validate question
    if (!body.question || typeof body.question !== 'string' || body.question.trim().length === 0) {
      console.error('❌ Missing or invalid question')
      return new Response(
        JSON.stringify({ error: 'Question is required and must be a non-empty string' }),
        { 
          status: 400, 
          headers: { ...corsHeaders, 'Content-Type': 'application/json' } 
        }
      )
    }

    const question = body.question.trim()
    console.log('🤔 Processing question:', question)

    // Step 1: Generate embedding for the question
    console.log('🤖 Step 1: Generating question embedding...')
    const questionEmbedding = await generateQuestionEmbedding(question)
    console.log('✅ Embedding generated:', questionEmbedding.length, 'dimensions')

    // Step 2: Search Qdrant for similar content  
    console.log('🗃️ Step 2: Searching vector database...')
    const searchResults = await searchVectorDatabase(questionEmbedding)
    console.log('✅ Found', searchResults.length, 'relevant documents')

    // Step 3: Build RAG prompt with context
    console.log('📝 Step 3: Building RAG prompt...')
    const ragPrompt = buildRAGPrompt(question, searchResults)
    console.log('✅ Prompt built with', searchResults.length, 'context snippets')

    // Step 4: Generate answer using chat completion
    console.log('💬 Step 4: Generating AI response...')
    const answer = await generateChatCompletion(ragPrompt)
    console.log('✅ Response generated:', answer.substring(0, 100) + '...')

    // Step 5: Format response
    const sources = searchResults.map((hit, index) => ({
      id: hit.id,
      snippet: hit.payload.content_preview || hit.payload.text || `${hit.payload.name} (${hit.payload.type})`
    }))

    const response: QueryResponse = {
      answer,
      sources
    }

    console.log('🎉 Query completed successfully')
    console.log('📊 Response summary:', {
      answerLength: answer.length,
      sourceCount: sources.length,
      processingMode: DEVELOPMENT_MODE ? 'development' : 'production'
    })

    return new Response(
      JSON.stringify(response),
      { 
        status: 200,
        headers: { ...corsHeaders, 'Content-Type': 'application/json' } 
      }
    )

  } catch (error) {
    console.error('💥 Query worker error:', error)
    
    const errorMessage = error instanceof Error ? error.message : 'An unexpected error occurred'
    
    // Return appropriate error status
    if (errorMessage.includes('No knowledge found')) {
      return new Response(
        JSON.stringify({ 
          error: 'No relevant knowledge found for your question',
          answer: 'I don\'t have enough information in my knowledge base to answer that question.',
          sources: []
        }),
        { 
          status: 200,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' } 
        }
      )
    } else {
      return new Response(
        JSON.stringify({ 
          error: 'Query processing failed', 
          details: DEVELOPMENT_MODE ? errorMessage : 'Internal server error'
        }),
        { 
          status: 500,
          headers: { ...corsHeaders, 'Content-Type': 'application/json' } 
        }
      )
    }
  }
})

// Generate embedding for the question using Ollama
async function generateQuestionEmbedding(question: string): Promise<number[]> {
  console.log('🤖 Generating question embedding with Ollama')
  
  if (DEVELOPMENT_MODE) {
    console.log('🔧 Using mock embedding (development mode)')
    return await mockGenerateEmbedding(question)
  }
  
  try {
    const response = await fetch(`${OLLAMA_URL}/api/embeddings`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: OLLAMA_EMBEDDING_MODEL,
        prompt: question,
      }),
    })

    if (!response.ok) {
      throw new Error(`Ollama API error: ${response.status} ${response.statusText}`)
    }

    const data = await response.json()
    
    if (!data.embedding || !Array.isArray(data.embedding)) {
      throw new Error('Invalid embedding response from Ollama')
    }

    console.log(`✅ Generated ${data.embedding.length}-dimensional embedding`)
    return data.embedding
    
  } catch (error) {
    console.error('❌ Ollama embedding error:', error)
    
    // Fallback to mock in development or on error
    if (DEVELOPMENT_MODE) {
      console.log('🔧 Falling back to mock embedding')
      return await mockGenerateEmbedding(question)
    }
    
    throw new Error(`Failed to generate question embedding: ${error instanceof Error ? error.message : 'Unknown error'}`)
  }
}

// Search Qdrant vector database for similar content
async function searchVectorDatabase(embedding: number[]): Promise<QdrantHit[]> {
  console.log('🗃️ Searching Qdrant vector database')
  
  if (DEVELOPMENT_MODE) {
    console.log('🔧 Using mock vector search (development mode)')
    return await mockVectorSearch(embedding)
  }
  
  try {
    const searchPayload = {
      vector: embedding,
      limit: 5,
      with_payload: true,
      score_threshold: 0.7 // Minimum similarity threshold
    }

    console.log('📡 Querying Qdrant:', `${QDRANT_URL}/collections/${QDRANT_COLLECTION}/points/search`)

    const response = await fetch(`${QDRANT_URL}/collections/${QDRANT_COLLECTION}/points/search`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(searchPayload),
    })

    if (!response.ok) {
      const errorText = await response.text()
      throw new Error(`Qdrant API error: ${response.status} ${errorText}`)
    }

    const data: QdrantSearchResponse = await response.json()
    
    if (!data.result || !Array.isArray(data.result)) {
      throw new Error('Invalid search response from Qdrant')
    }

    console.log(`✅ Found ${data.result.length} relevant documents`)
    data.result.forEach((hit, index) => {
      console.log(`  ${index + 1}. ${hit.payload.name} (score: ${hit.score.toFixed(3)})`)
    })
    
    return data.result
    
  } catch (error) {
    console.error('❌ Qdrant search error:', error)
    
    // Fallback to mock in development or on error
    if (DEVELOPMENT_MODE) {
      console.log('🔧 Falling back to mock vector search')
      return await mockVectorSearch(embedding)
    }
    
    throw new Error(`Failed to search vector database: ${error instanceof Error ? error.message : 'Unknown error'}`)
  }
}

// Build RAG prompt with context from search results
function buildRAGPrompt(question: string, searchResults: QdrantHit[]): string {
  console.log('📝 Building RAG prompt with context')
  
  if (searchResults.length === 0) {
    throw new Error('No knowledge found to answer the question')
  }
  
  let prompt = 'You are an intelligent assistant with access to The Brain knowledge base. Use the following snippets to answer the user\'s question. Be accurate, helpful, and cite the relevant snippets when appropriate.\n\n'
  
  // Add context snippets
  prompt += 'KNOWLEDGE SNIPPETS:\n'
  searchResults.forEach((hit, index) => {
    const snippetNumber = index + 1
    const content = hit.payload.content_preview || hit.payload.text || hit.payload.name
    const truncatedContent = content.length > 500 ? content.substring(0, 500) + '...' : content
    
    prompt += `\nSnippet ${snippetNumber}: ${truncatedContent}\n`
    prompt += `Source: ${hit.payload.name} (${hit.payload.type})\n`
  })
  
  // Add the question
  prompt += `\nQuestion: ${question}\n\n`
  prompt += 'Answer: '
  
  console.log('✅ Prompt built with', searchResults.length, 'snippets')
  return prompt
}

// Generate chat completion using Ollama
async function generateChatCompletion(prompt: string): Promise<string> {
  console.log('💬 Generating chat completion with Ollama')
  
  if (DEVELOPMENT_MODE) {
    console.log('🔧 Using mock chat completion (development mode)')
    return await mockChatCompletion(prompt)
  }
  
  try {
    const response = await fetch(`${OLLAMA_URL}/api/generate`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: OLLAMA_CHAT_MODEL,
        prompt: prompt,
        stream: false,
        options: {
          temperature: 0.7,
          top_p: 0.9,
          max_tokens: 1000
        }
      }),
    })

    if (!response.ok) {
      throw new Error(`Ollama API error: ${response.status} ${response.statusText}`)
    }

    const data = await response.json()
    
    if (!data.response || typeof data.response !== 'string') {
      throw new Error('Invalid chat completion response from Ollama')
    }

    console.log('✅ Generated chat completion')
    return data.response.trim()
    
  } catch (error) {
    console.error('❌ Ollama chat completion error:', error)
    
    // Fallback to mock in development or on error
    if (DEVELOPMENT_MODE) {
      console.log('🔧 Falling back to mock chat completion')
      return await mockChatCompletion(prompt)
    }
    
    throw new Error(`Failed to generate chat completion: ${error instanceof Error ? error.message : 'Unknown error'}`)
  }
}

// Mock functions for development mode
async function mockGenerateEmbedding(text: string): Promise<number[]> {
  console.log('🔧 Generating mock embedding for:', text.substring(0, 50) + '...')
  
  // Generate a simple mock embedding based on text hash
  const encoder = new TextEncoder()
  const data = encoder.encode(text)
  const hashBuffer = await crypto.subtle.digest('SHA-256', data)
  const hashArray = Array.from(new Uint8Array(hashBuffer))
  
  // Convert to 384-dimensional embedding (common size)
  const embedding = []
  for (let i = 0; i < 384; i++) {
    const value = (hashArray[i % hashArray.length] - 128) / 128
    embedding.push(value)
  }
  
  console.log('✅ Mock embedding generated:', embedding.length, 'dimensions')
  return embedding
}

async function mockVectorSearch(embedding: number[]): Promise<QdrantHit[]> {
  console.log('🔧 Performing mock vector search')
  
  // Return mock search results that simulate finding relevant documents
  const mockResults: QdrantHit[] = [
    {
      id: 'mock-document-1',
      score: 0.95,
      payload: {
        artifact_id: 'mock-artifact-1',
        name: 'The Brain System Overview',
        type: 'document',
        text: 'The Brain is a 3D knowledge visualization system that helps you organize and explore your ideas, projects, and information in an intuitive spatial interface.',
        content_preview: 'The Brain is a 3D knowledge visualization system that helps you organize and explore your ideas, projects, and information in an intuitive spatial interface.',
        created_at: new Date().toISOString(),
        metadata: { source: 'mock', category: 'system' }
      }
    },
    {
      id: 'mock-document-2', 
      score: 0.87,
      payload: {
        artifact_id: 'mock-artifact-2',
        name: 'Knowledge Management Features',
        type: 'note',
        text: 'Key features include real-time processing, AI-powered search, immediate file indexing, and semantic query capabilities.',
        content_preview: 'Key features include real-time processing, AI-powered search, immediate file indexing, and semantic query capabilities.',
        created_at: new Date().toISOString(),
        metadata: { source: 'mock', category: 'features' }
      }
    },
    {
      id: 'mock-document-3',
      score: 0.82,
      payload: {
        artifact_id: 'mock-artifact-3', 
        name: 'Upload and Processing Guide',
        type: 'file',
        text: 'Documents are processed immediately upon upload using RAG (Retrieval-Augmented Generation) technology for instant searchability.',
        content_preview: 'Documents are processed immediately upon upload using RAG technology for instant searchability.',
        created_at: new Date().toISOString(),
        metadata: { source: 'mock', category: 'guide' }
      }
    }
  ]
  
  console.log('✅ Mock search completed:', mockResults.length, 'results')
  return mockResults
}

async function mockChatCompletion(prompt: string): Promise<string> {
  console.log('🔧 Generating mock chat completion')
  
  // Extract the question from the prompt
  const questionMatch = prompt.match(/Question: (.+)/);
  const question = questionMatch ? questionMatch[1] : 'your question';
  
  const mockResponse = `Based on the knowledge snippets provided, I can help answer your question about "${question}".

The Brain is a comprehensive 3D knowledge visualization and management system that combines several powerful features:

**Core Capabilities:**
- **3D Visualization**: Organizes information in an intuitive spatial interface with pillars, nodes, and connections
- **Immediate Processing**: Files are processed instantly upon upload using RAG (Retrieval-Augmented Generation) technology
- **AI-Powered Search**: Semantic search capabilities allow you to query your knowledge base naturally
- **Real-time Updates**: All changes are reflected immediately across the system

**Key Features from the knowledge base:**
- Upload documents and they become searchable within seconds
- Ask questions and get AI-powered answers with source citations
- Visual organization of ideas, projects, tasks, and knowledge
- Integration with Supabase for data storage and real-time synchronization

The system is designed to make knowledge management intuitive and powerful, allowing you to upload, organize, and query your information seamlessly.

*Note: This is a development mode response. Connect Ollama and Qdrant for full AI capabilities.*`

  console.log('✅ Mock chat completion generated')
  return mockResponse
}
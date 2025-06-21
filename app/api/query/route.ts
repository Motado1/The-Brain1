import { NextRequest, NextResponse } from 'next/server';

// Types
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
const OLLAMA_URL = process.env.OLLAMA_URL || 'http://localhost:11434';
const OLLAMA_EMBEDDING_MODEL = process.env.OLLAMA_EMBEDDING_MODEL || 'nomic-embed-text';
const OLLAMA_CHAT_MODEL = process.env.OLLAMA_CHAT_MODEL || 'llama3.2';
const QDRANT_URL = process.env.QDRANT_URL || 'http://localhost:6333';
const QDRANT_COLLECTION = process.env.QDRANT_COLLECTION || 'brain-knowledge';
const DEVELOPMENT_MODE = process.env.DEVELOPMENT_MODE === 'true';

export async function POST(request: NextRequest) {
  try {
    // Parse and validate request
    const body = await request.json() as QueryRequest;
    
    if (!body.question || typeof body.question !== 'string' || body.question.trim().length === 0) {
      return NextResponse.json(
        { error: 'Question is required and must be a non-empty string' },
        { status: 400 }
      );
    }

    const question = body.question.trim();
    console.log('🔍 Processing query:', question);

    // Step 1: Generate embedding for the question
    const questionEmbedding = await generateQuestionEmbedding(question);
    
    // Step 2: Search Qdrant for similar content
    const searchResults = await searchVectorDatabase(questionEmbedding);
    
    // Step 3: Build RAG prompt with context
    const ragPrompt = buildRAGPrompt(question, searchResults);
    
    // Step 4: Generate answer using chat completion
    const answer = await generateChatCompletion(ragPrompt);
    
    // Step 5: Format response
    const sources = searchResults.map((hit, index) => ({
      id: hit.id,
      snippet: hit.payload.content_preview || hit.payload.text || `${hit.payload.name} (${hit.payload.type})`
    }));

    const response: QueryResponse = {
      answer,
      sources
    };

    console.log('✅ Query completed successfully');
    return NextResponse.json(response);

  } catch (error) {
    console.error('❌ Query failed:', error);
    
    const errorMessage = error instanceof Error ? error.message : 'An unexpected error occurred';
    
    // Return appropriate error status
    if (errorMessage.includes('Question is required')) {
      return NextResponse.json({ error: errorMessage }, { status: 400 });
    } else if (errorMessage.includes('No knowledge found')) {
      return NextResponse.json({ 
        error: 'No relevant knowledge found for your question',
        answer: 'I don\'t have enough information in my knowledge base to answer that question.',
        sources: []
      }, { status: 200 });
    } else {
      return NextResponse.json({ error: 'Internal server error' }, { status: 500 });
    }
  }
}

// Generate embedding for the question using Ollama
async function generateQuestionEmbedding(question: string): Promise<number[]> {
  console.log('🤖 Generating question embedding with Ollama');
  
  if (DEVELOPMENT_MODE) {
    return await mockGenerateEmbedding(question);
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
    });

    if (!response.ok) {
      throw new Error(`Ollama API error: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    
    if (!data.embedding || !Array.isArray(data.embedding)) {
      throw new Error('Invalid embedding response from Ollama');
    }

    console.log(`Generated ${data.embedding.length}-dimensional embedding`);
    return data.embedding;
    
  } catch (error) {
    console.error('Ollama embedding error:', error);
    
    // Fallback to mock in development or on error
    if (DEVELOPMENT_MODE) {
      console.log('Falling back to mock embedding');
      return await mockGenerateEmbedding(question);
    }
    
    throw new Error(`Failed to generate question embedding: ${(error as Error).message}`);
  }
}

// Search Qdrant vector database for similar content
async function searchVectorDatabase(embedding: number[]): Promise<QdrantHit[]> {
  console.log('🗃️ Searching Qdrant vector database');
  
  if (DEVELOPMENT_MODE) {
    return await mockVectorSearch(embedding);
  }
  
  try {
    const searchPayload = {
      vector: embedding,
      limit: 5,
      with_payload: true,
      score_threshold: 0.7 // Minimum similarity threshold
    };

    const response = await fetch(`${QDRANT_URL}/collections/${QDRANT_COLLECTION}/points/search`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(searchPayload),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Qdrant API error: ${response.status} ${errorText}`);
    }

    const data: QdrantSearchResponse = await response.json();
    
    if (!data.result || !Array.isArray(data.result)) {
      throw new Error('Invalid search response from Qdrant');
    }

    console.log(`Found ${data.result.length} relevant documents`);
    return data.result;
    
  } catch (error) {
    console.error('Qdrant search error:', error);
    
    // Fallback to mock in development or on error
    if (DEVELOPMENT_MODE) {
      console.log('Falling back to mock vector search');
      return await mockVectorSearch(embedding);
    }
    
    throw new Error(`Failed to search vector database: ${(error as Error).message}`);
  }
}

// Build RAG prompt with context from search results
function buildRAGPrompt(question: string, searchResults: QdrantHit[]): string {
  console.log('📝 Building RAG prompt with context');
  
  if (searchResults.length === 0) {
    throw new Error('No knowledge found to answer the question');
  }
  
  let prompt = 'You are an intelligent assistant with access to The Brain knowledge base. Use the following snippets to answer the user\'s question. Be accurate, helpful, and cite the relevant snippets when appropriate.\n\n';
  
  // Add context snippets
  prompt += 'KNOWLEDGE SNIPPETS:\n';
  searchResults.forEach((hit, index) => {
    const snippetNumber = index + 1;
    const content = hit.payload.content_preview || hit.payload.text || hit.payload.name;
    const truncatedContent = content.length > 500 ? content.substring(0, 500) + '...' : content;
    
    prompt += `\nSnippet ${snippetNumber}: ${truncatedContent}\n`;
    prompt += `Source: ${hit.payload.name} (${hit.payload.type})\n`;
  });
  
  // Add the question
  prompt += `\nQuestion: ${question}\n\n`;
  prompt += 'Answer: ';
  
  return prompt;
}

// Generate chat completion using Ollama
async function generateChatCompletion(prompt: string): Promise<string> {
  console.log('💬 Generating chat completion with Ollama');
  
  if (DEVELOPMENT_MODE) {
    return await mockChatCompletion(prompt);
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
    });

    if (!response.ok) {
      throw new Error(`Ollama API error: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    
    if (!data.response || typeof data.response !== 'string') {
      throw new Error('Invalid chat completion response from Ollama');
    }

    console.log('Generated chat completion');
    return data.response.trim();
    
  } catch (error) {
    console.error('Ollama chat completion error:', error);
    
    // Fallback to mock in development or on error
    if (DEVELOPMENT_MODE) {
      console.log('Falling back to mock chat completion');
      return await mockChatCompletion(prompt);
    }
    
    throw new Error(`Failed to generate chat completion: ${(error as Error).message}`);
  }
}

// Mock functions for development mode
async function mockGenerateEmbedding(text: string): Promise<number[]> {
  console.log('🔧 Using mock embedding (development mode)');
  
  // Generate a simple mock embedding based on text hash
  const encoder = new TextEncoder();
  const data = encoder.encode(text);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  
  // Convert to 384-dimensional embedding (common size)
  const embedding = [];
  for (let i = 0; i < 384; i++) {
    const value = (hashArray[i % hashArray.length] - 128) / 128;
    embedding.push(value);
  }
  
  return embedding;
}

async function mockVectorSearch(embedding: number[]): Promise<QdrantHit[]> {
  console.log('🔧 Using mock vector search (development mode)');
  
  // Return mock search results
  return [
    {
      id: 'mock-1',
      score: 0.95,
      payload: {
        artifact_id: 'mock-artifact-1',
        name: 'Sample Document 1',
        type: 'document',
        text: 'This is a sample document that contains relevant information about the topic you asked about.',
        content_preview: 'This is a sample document that contains relevant information about the topic you asked about.',
        created_at: new Date().toISOString(),
        metadata: { source: 'mock' }
      }
    },
    {
      id: 'mock-2',
      score: 0.87,
      payload: {
        artifact_id: 'mock-artifact-2',
        name: 'Sample Note 2',
        type: 'note',
        text: 'Here is additional context that might help answer your question with more details.',
        content_preview: 'Here is additional context that might help answer your question with more details.',
        created_at: new Date().toISOString(),
        metadata: { source: 'mock' }
      }
    }
  ];
}

async function mockChatCompletion(prompt: string): Promise<string> {
  console.log('🔧 Using mock chat completion (development mode)');
  
  // Extract the question from the prompt
  const questionMatch = prompt.match(/Question: (.+)/);
  const question = questionMatch ? questionMatch[1] : 'your question';
  
  return `Based on the knowledge snippets provided, I can help answer your question about "${question}". 

The information from Snippet 1 and Snippet 2 provides relevant context that addresses your inquiry. While this is a mock response for development purposes, in production this would be a comprehensive answer generated by the AI model using the actual knowledge base content.

Please note: This is a development mode response. Connect Ollama for full AI capabilities.`;
}
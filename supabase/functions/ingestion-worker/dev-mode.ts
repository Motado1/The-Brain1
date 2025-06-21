// Development mode utilities for the ingestion worker
// Provides mock implementations when external services aren't available

export function isDevelopmentMode(): boolean {
  // Always enable development mode for now since we don't have Ollama/Qdrant running
  return true
  // return Deno.env.get('DEVELOPMENT_MODE') === 'true'
}

// Mock Ollama embeddings for development
export async function mockGenerateEmbeddings(text: string): Promise<number[]> {
  console.log('🔧 Using mock embeddings (development mode)')
  
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
  
  return embedding
}

// Mock Qdrant storage for development
export async function mockStoreInVectorDB(artifact: any, text: string, embeddings: number[]) {
  console.log('🔧 Using mock vector storage (development mode)')
  console.log(`Mock storing artifact ${artifact.id} with ${embeddings.length}-dim embedding`)
  
  // Simulate success
  return { status: 'ok', result: { operation_id: 'mock_' + Date.now() } }
}

// Mock file download for development
export async function mockDownloadFile(storagePath: string): Promise<string> {
  console.log('🔧 Using mock file download (development mode)')
  
  // Return mock content based on file extension
  const extension = storagePath.split('.').pop()?.toLowerCase()
  
  switch (extension) {
    case 'txt':
      return `Mock text content from ${storagePath}. This would normally be the actual file content.`
    case 'pdf':
      return `Mock PDF content extracted from ${storagePath}. This represents extracted text from a PDF document.`
    case 'md':
      return `# Mock Markdown\n\nThis is mock markdown content from ${storagePath}.\n\n- List item 1\n- List item 2`
    default:
      return `Mock content from ${storagePath} (${extension} file)`
  }
}
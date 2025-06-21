# RAG Ingestion Worker

Edge Function for processing artifact ingestion jobs in The Brain's RAG system.

## Overview

This Deno-based Edge Function processes artifacts by:
1. Dequeuing jobs from the job_queue table
2. Extracting text content from files or using provided content
3. Generating embeddings using Ollama (or mock embeddings in development)
4. Storing vectors in Qdrant database (or mock storage in development)
5. Updating artifact status and metadata

## Features

### ✅ Implemented
- **Job Queue Processing**: Dequeues and locks jobs with optimistic locking
- **File Download**: Downloads files from Supabase Storage via signed URLs
- **Text Extraction**: Placeholder implementation for various file types
- **Embedding Generation**: Integrates with Ollama API for embeddings
- **Vector Storage**: Stores embeddings in Qdrant vector database
- **Error Handling**: Implements retry logic with exponential backoff
- **Development Mode**: Mock implementations when external services unavailable

### 🚧 Future Enhancements
- Advanced text extraction for PDF, Word documents
- Chunking for large documents
- Different embedding models
- Vector search optimization

## Configuration

Environment variables:
```bash
# Required
SUPABASE_URL=http://localhost:54321
SUPABASE_SERVICE_ROLE_KEY=your-service-role-key

# Optional - External Services
OLLAMA_URL=http://localhost:11434
OLLAMA_EMBEDDING_MODEL=nomic-embed-text
QDRANT_URL=http://localhost:6333
QDRANT_COLLECTION=brain-knowledge

# Development
DEVELOPMENT_MODE=true  # Uses mock services
```

## Usage

### Deploy Function
```bash
npx supabase functions deploy ingestion-worker
```

### Local Development
```bash
npx supabase functions serve
```

### Trigger Processing
```bash
curl -X POST "http://localhost:54321/functions/v1/ingestion-worker" \
  -H "Authorization: Bearer your-anon-key"
```

## Job Processing Flow

1. **Dequeue**: Finds pending/retrying jobs ordered by priority and age
2. **Lock**: Sets job status to 'running' with optimistic locking
3. **Process**: Extracts text, generates embeddings, stores vectors
4. **Complete**: Updates artifact status to 'indexed' and job to 'completed'
5. **Error**: Implements retry with exponential backoff, max 3 attempts

## Development Mode

When external services (Ollama, Qdrant) aren't available, the function uses:
- **Mock embeddings**: 384-dimensional vectors based on content hash
- **Mock vector storage**: Simulated storage operations
- **Mock file download**: Sample content for missing files

## Testing

Test the worker:
```bash
node test-ingestion-worker.js
```

Create test artifacts:
```bash
curl -X POST "http://localhost:3001/api/artifacts" \
  -H "Content-Type: application/json" \
  -d '{"name": "Test", "type": "note", "content": "Test content"}'
```

## Error Handling

- **File not found**: Uses mock content in development mode
- **Ollama unavailable**: Falls back to mock embeddings in development
- **Qdrant unavailable**: Falls back to mock storage in development
- **Job failures**: Retries with exponential backoff (5, 10, 20 minutes)
- **Max retries**: Marks job and artifact as failed after 3 attempts
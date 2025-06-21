# 🔍 Query API - RAG Search Complete

The Brain now has a fully functional RAG (Retrieval-Augmented Generation) query system! Ask questions and get AI-powered answers from your knowledge base.

## ✅ Query API Features

### 🚀 **Core Functionality**
- **POST `/api/query`**: Ask questions about your knowledge base
- **RAG Pipeline**: Question embedding → Vector search → Context building → AI generation
- **Smart Responses**: AI-powered answers with cited sources
- **Error Handling**: Comprehensive validation and fallback responses
- **Development Mode**: Mock responses when AI services unavailable

### 🔧 **Technical Architecture**

#### **Request Format**
```typescript
POST /api/query
Content-Type: application/json

{
  "question": "What is The Brain?"
}
```

#### **Response Format**
```typescript
{
  "answer": "AI-generated response based on knowledge base",
  "sources": [
    {
      "id": "artifact-uuid",
      "snippet": "Relevant content preview from source document"
    }
  ]
}
```

#### **Error Responses**
```typescript
// Validation Error (400)
{
  "error": "Question is required and must be a non-empty string"
}

// No Knowledge Found (200)
{
  "error": "No relevant knowledge found for your question",
  "answer": "I don't have enough information...",
  "sources": []
}

// Server Error (500)
{
  "error": "Internal server error"
}
```

## 🤖 **AI Integration**

### **Embedding Generation**
- **Service**: Ollama API (`/api/embeddings`)
- **Model**: `nomic-embed-text` (384-dimensional embeddings)
- **Fallback**: Mock embeddings using SHA-256 hash in development mode

### **Vector Search**
- **Service**: Qdrant vector database
- **Collection**: `brain-knowledge`
- **Parameters**: Top 5 results, 0.7 similarity threshold
- **Fallback**: Mock search results in development mode

### **Chat Completion**
- **Service**: Ollama API (`/api/generate`)
- **Model**: `llama3.2` (configurable)
- **Settings**: Temperature 0.7, Top-p 0.9, Max tokens 1000
- **Fallback**: Mock responses in development mode

## 🎯 **RAG Prompt Engineering**

### **Prompt Structure**
```
You are an intelligent assistant with access to The Brain knowledge base. 
Use the following snippets to answer the user's question. Be accurate, 
helpful, and cite the relevant snippets when appropriate.

KNOWLEDGE SNIPPETS:

Snippet 1: [First relevant document content...]
Source: Document Name (document type)

Snippet 2: [Second relevant document content...]
Source: Note Title (note type)

...

Question: What is The Brain?

Answer: 
```

### **Context Processing**
- **Snippet Limit**: Top 5 most relevant results
- **Content Truncation**: 500 characters per snippet
- **Source Attribution**: Document name and type included
- **Question Preservation**: Original question included in prompt

## 🌐 **Frontend Integration**

### **Knowledge Core Interface**
The query system is integrated into the Knowledge Core pillar panel:

1. **Query Input**: Text field with "Ask The Brain" interface
2. **Real-time Results**: Instant responses with loading states
3. **Source Display**: Expandable source snippets with attribution
4. **Error Handling**: User-friendly error messages

### **Usage Flow**
1. Click on "Knowledge Core" pillar in The Brain
2. Type question in "🔍 Ask The Brain" input field
3. Press Enter or click search button
4. View AI-generated answer with source citations
5. Explore source snippets for more context

## 🔧 **Configuration**

### **Environment Variables**
```bash
# Ollama Configuration
OLLAMA_URL=http://localhost:11434
OLLAMA_EMBEDDING_MODEL=nomic-embed-text
OLLAMA_CHAT_MODEL=llama3.2

# Qdrant Configuration  
QDRANT_URL=http://localhost:6333
QDRANT_COLLECTION=brain-knowledge

# Development Mode
DEVELOPMENT_MODE=true
```

### **Production Setup**
For production deployment:
1. Set `DEVELOPMENT_MODE=false`
2. Ensure Ollama is running and accessible
3. Ensure Qdrant is running with `brain-knowledge` collection
4. Configure proper API endpoints
5. Test embedding and chat models

## 🧪 **Testing**

### **API Testing**
```bash
# Run comprehensive tests
node test-query-api.js

# Manual API test
curl -X POST http://localhost:3001/api/query \
  -H "Content-Type: application/json" \
  -d '{"question":"How does knowledge management work?"}'
```

### **Browser Testing**
1. Navigate to http://localhost:3001
2. Click "Knowledge Core" pillar
3. Use "🔍 Ask The Brain" interface
4. Test various questions
5. Verify responses and sources

### **Error Testing**
- Empty questions → 400 error
- Invalid JSON → 400 error
- Server issues → 500 error
- No knowledge → Graceful fallback

## ⚡ **Performance Features**

### **Efficient Processing**
- **Async Operations**: Non-blocking API calls
- **Error Recovery**: Graceful fallbacks to mock services
- **Resource Management**: Optimized embedding and search
- **Response Streaming**: Future enhancement ready

### **Development Experience**
- **Mock Services**: Full functionality without external dependencies
- **Debug Logging**: Comprehensive console output
- **Error Messages**: Clear developer-friendly errors
- **Type Safety**: Full TypeScript support

## 🎉 **Ready for AI Queries!**

Your Brain now provides:
- ✅ **RAG Query API** - Ask questions, get AI answers
- ✅ **Vector Search** - Semantic similarity matching
- ✅ **Source Attribution** - Transparent knowledge sourcing
- ✅ **Frontend Integration** - Beautiful query interface
- ✅ **Development Mode** - Works without external AI services
- ✅ **Error Handling** - Robust error management
- ✅ **Type Safety** - Full TypeScript support

### **Example Queries**
- "What is The Brain?"
- "How does the upload process work?"
- "Tell me about knowledge management"
- "What files can I upload?"
- "How do I search my documents?"

Ask The Brain anything about your uploaded knowledge! 🧠✨
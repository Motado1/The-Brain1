// Edge Function for RAG ingestion worker
// Processes artifacts by extracting text, generating embeddings, and storing in vector DB

import { serve } from 'https://deno.land/std@0.177.0/http/server.ts'
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2.38.4'
import { isDevelopmentMode, mockGenerateEmbeddings, mockStoreInVectorDB, mockDownloadFile } from './dev-mode.ts'

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

interface JobQueueItem {
  id: string
  job_type: string
  status: string
  priority: number
  payload: Record<string, any>
  retry_count: number
  max_retries: number
  next_run_at: string
  created_at: string
  updated_at: string
}

interface ArtifactData {
  id: string
  name: string
  type: string
  url?: string
  content?: string
  status: string
  metadata: Record<string, any>
}

serve(async (req) => {
  // Handle CORS
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    // Initialize Supabase client with service role
    const supabaseUrl = Deno.env.get('SUPABASE_URL')!
    const serviceRoleKey = Deno.env.get('SUPABASE_SERVICE_ROLE_KEY')!
    
    const supabase = createClient(supabaseUrl, serviceRoleKey)

    console.log('🔄 Ingestion worker started')

    // Check if this is an immediate trigger request
    let job: JobQueueItem | null = null
    
    if (req.method === 'POST') {
      try {
        const body = await req.json()
        if (body.trigger === 'immediate' && body.jobId) {
          console.log('⚡ Immediate trigger for job:', body.jobId)
          job = await getSpecificJob(supabase, body.jobId)
          if (job) {
            // Lock the specific job for processing
            await updateJobStatus(supabase, job.id, 'processing')
          }
        }
      } catch (e) {
        console.log('📄 No valid JSON body, proceeding with queue processing')
      }
    }
    
    // Step 1: If no immediate job, dequeue from queue
    if (!job) {
      job = await dequeueJob(supabase)
      if (!job) {
        return new Response(
          JSON.stringify({ message: 'No jobs available for processing' }),
          { headers: { ...corsHeaders, 'Content-Type': 'application/json' }, status: 200 }
        )
      }
    }

    console.log(`📋 Processing job: ${job.id} (${job.job_type})`)

    try {
      // Step 2: Process the artifact based on job type
      if (job.job_type === 'ingest_artifact') {
        await processArtifactIngestion(supabase, job)
      } else {
        throw new Error(`Unknown job type: ${job.job_type}`)
      }

      // Step 3: Mark job as completed
      await updateJobStatus(supabase, job.id, 'completed')
      console.log(`✅ Job ${job.id} completed successfully`)

      return new Response(
        JSON.stringify({ 
          message: 'Job processed successfully', 
          jobId: job.id,
          jobType: job.job_type 
        }),
        { headers: { ...corsHeaders, 'Content-Type': 'application/json' }, status: 200 }
      )

    } catch (error) {
      console.error(`❌ Job ${job.id} failed:`, error)
      
      // Update job with failure status
      await updateJobWithError(supabase, job, error as Error)
      
      return new Response(
        JSON.stringify({ 
          error: 'Job processing failed', 
          jobId: job.id,
          message: (error as Error).message 
        }),
        { headers: { ...corsHeaders, 'Content-Type': 'application/json' }, status: 500 }
      )
    }

  } catch (error) {
    console.error('💥 Worker error:', error)
    return new Response(
      JSON.stringify({ error: 'Internal server error' }),
      { headers: { ...corsHeaders, 'Content-Type': 'application/json' }, status: 500 }
    )
  }
})

// Dequeue and lock a job for processing
async function dequeueJob(supabase: any): Promise<JobQueueItem | null> {
  const now = new Date().toISOString()
  
  // Find and lock the next available job
  const { data: jobs, error } = await supabase
    .from('job_queue')
    .select('*')
    .in('status', ['pending', 'retrying'])
    .lte('next_run_at', now)
    .order('priority', { ascending: false })
    .order('created_at', { ascending: true })
    .limit(1)

  if (error) {
    console.error('Error fetching jobs:', error)
    throw error
  }

  if (!jobs || jobs.length === 0) {
    return null
  }

  const job = jobs[0]

  // Lock the job by updating its status to 'running'
  const { error: lockError } = await supabase
    .from('job_queue')
    .update({
      status: 'running',
      started_at: now,
      updated_at: now
    })
    .eq('id', job.id)
    .eq('status', job.status) // Optimistic locking

  if (lockError) {
    console.error('Error locking job:', lockError)
    throw lockError
  }

  return { ...job, status: 'running' }
}

// Get a specific job by ID for immediate processing
async function getSpecificJob(supabase: any, jobId: string): Promise<JobQueueItem | null> {
  console.log('🔍 Fetching specific job:', jobId)
  
  const { data: job, error } = await supabase
    .from('job_queue')
    .select('*')
    .eq('id', jobId)
    .in('status', ['pending', 'retrying']) // Only get jobs that can be processed
    .single()

  if (error) {
    console.error('Error fetching specific job:', error)
    return null
  }

  if (!job) {
    console.log('Job not found or not available for processing:', jobId)
    return null
  }

  console.log('✅ Found specific job:', job.id, job.job_type)
  return job
}

// Process artifact ingestion job
async function processArtifactIngestion(supabase: any, job: JobQueueItem) {
  const { artifactId, storagePath, content, url, type } = job.payload

  // Get artifact details
  const { data: artifact, error: artifactError } = await supabase
    .from('artifacts')
    .select('*')
    .eq('id', artifactId)
    .single()

  if (artifactError || !artifact) {
    throw new Error(`Artifact not found: ${artifactId}`)
  }

  console.log(`📄 Processing artifact: ${artifact.name} (${artifact.type})`)

  let textContent: string = ''

  // Step 1: Extract text content
  if (artifact.type === 'file' && storagePath) {
    // Download file from storage and extract text
    textContent = await downloadAndExtractText(supabase, storagePath)
  } else if (artifact.type === 'note' && content) {
    // Use content directly for notes
    textContent = content
  } else if (artifact.type === 'link' && url) {
    // For now, use URL as text (could be enhanced to fetch content)
    textContent = `Link: ${url}`
  } else {
    throw new Error(`Cannot process artifact type: ${artifact.type}`)
  }

  if (!textContent.trim()) {
    throw new Error('No text content extracted from artifact')
  }

  console.log(`📝 Extracted ${textContent.length} characters of text`)

  // Step 2: Generate embeddings using Ollama
  const embeddings = await generateEmbeddings(textContent)
  console.log(`🧠 Generated ${embeddings.length}-dimensional embedding`)

  // Step 3: Store in Qdrant vector database
  await storeInVectorDB(artifact, textContent, embeddings)
  console.log('💾 Stored in vector database')

  // Step 4: Update artifact status and metadata
  await updateArtifactCompletion(supabase, artifactId, {
    embedding: embeddings,
    content_length: textContent.length,
    processed_at: new Date().toISOString(),
    content_hash: await generateContentHash(textContent)
  })

  console.log(`✨ Artifact ${artifactId} processing completed`)
}

// Download file from Supabase Storage and extract text
async function downloadAndExtractText(supabase: any, storagePath: string): Promise<string> {
  console.log(`⬇️ Downloading file: ${storagePath}`)

  // Use mock download in development mode if file doesn't exist
  if (isDevelopmentMode()) {
    try {
      const { data: signedUrlData, error: urlError } = await supabase.storage
        .from('knowledge')
        .createSignedUrl(storagePath.replace('knowledge/', ''), 60)

      if (urlError) {
        console.log('File not found, using mock content')
        const mockContent = await mockDownloadFile(storagePath)
        return extractText(mockContent, storagePath)
      }

      // Try to download real file
      const response = await fetch(signedUrlData.signedUrl)
      if (!response.ok) {
        console.log('File download failed, using mock content')
        const mockContent = await mockDownloadFile(storagePath)
        return extractText(mockContent, storagePath)
      }

      const fileContent = await response.text()
      return extractText(fileContent, storagePath)
    } catch (error) {
      console.log('Error accessing file, using mock content:', error.message)
      const mockContent = await mockDownloadFile(storagePath)
      return extractText(mockContent, storagePath)
    }
  }

  // Production mode - require real file
  const { data: signedUrlData, error: urlError } = await supabase.storage
    .from('knowledge')
    .createSignedUrl(storagePath.replace('knowledge/', ''), 60)

  if (urlError || !signedUrlData?.signedUrl) {
    throw new Error(`Failed to create signed URL: ${urlError?.message}`)
  }

  const response = await fetch(signedUrlData.signedUrl)
  if (!response.ok) {
    throw new Error(`Failed to download file: ${response.statusText}`)
  }

  const fileContent = await response.text()
  return extractText(fileContent, storagePath)
}

// Extract text from file content (placeholder)
function extractText(content: string, filePath: string): string {
  console.log(`🔍 Extracting text from: ${filePath}`)
  
  // TODO: Implement proper text extraction based on file type
  // For now, return content as-is for text files
  const extension = filePath.split('.').pop()?.toLowerCase()
  
  switch (extension) {
    case 'txt':
    case 'md':
    case 'csv':
    case 'json':
      return content
    case 'pdf':
      // TODO: Implement PDF text extraction
      return 'TODO: PDF text extraction not implemented'
    case 'doc':
    case 'docx':
      // TODO: Implement Word document text extraction
      return 'TODO: Word document text extraction not implemented'
    default:
      // Fallback: try to return content as text
      return content || 'TODO: extracted text'
  }
}

// Generate embeddings using Ollama
async function generateEmbeddings(text: string): Promise<number[]> {
  console.log('🤖 Generating embeddings with Ollama')
  
  // Use mock embeddings in development mode
  if (isDevelopmentMode()) {
    return await mockGenerateEmbeddings(text)
  }
  
  const ollamaUrl = Deno.env.get('OLLAMA_URL') || 'http://localhost:11434'
  const model = Deno.env.get('OLLAMA_EMBEDDING_MODEL') || 'nomic-embed-text'

  try {
    const response = await fetch(`${ollamaUrl}/api/embeddings`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: model,
        prompt: text,
      }),
    })

    if (!response.ok) {
      throw new Error(`Ollama API error: ${response.status} ${response.statusText}`)
    }

    const data = await response.json()
    
    if (!data.embedding || !Array.isArray(data.embedding)) {
      throw new Error('Invalid embedding response from Ollama')
    }

    return data.embedding
  } catch (error) {
    console.error('Ollama embedding error:', error)
    
    // Fallback to mock in development mode
    if (isDevelopmentMode()) {
      console.log('Falling back to mock embeddings')
      return await mockGenerateEmbeddings(text)
    }
    
    throw new Error(`Failed to generate embeddings: ${(error as Error).message}`)
  }
}

// Store in Qdrant vector database
async function storeInVectorDB(artifact: ArtifactData, text: string, embeddings: number[]) {
  console.log('🗃️ Storing in Qdrant vector database')
  
  // Use mock storage in development mode
  if (isDevelopmentMode()) {
    return await mockStoreInVectorDB(artifact, text, embeddings)
  }
  
  const qdrantUrl = Deno.env.get('QDRANT_URL') || 'http://localhost:6333'
  const collectionName = Deno.env.get('QDRANT_COLLECTION') || 'brain-knowledge'

  const payload = {
    points: [{
      id: artifact.id,
      vector: embeddings,
      payload: {
        artifact_id: artifact.id,
        name: artifact.name,
        type: artifact.type,
        text: text.substring(0, 1000), // Store first 1000 chars
        url: artifact.url,
        created_at: new Date().toISOString(),
        metadata: artifact.metadata
      }
    }]
  }

  try {
    const response = await fetch(`${qdrantUrl}/collections/${collectionName}/points`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    })

    if (!response.ok) {
      const errorText = await response.text()
      throw new Error(`Qdrant API error: ${response.status} ${errorText}`)
    }

    const result = await response.json()
    console.log('Qdrant storage result:', result)
  } catch (error) {
    console.error('Qdrant storage error:', error)
    
    // Fallback to mock in development mode
    if (isDevelopmentMode()) {
      console.log('Falling back to mock vector storage')
      return await mockStoreInVectorDB(artifact, text, embeddings)
    }
    
    throw new Error(`Failed to store in vector database: ${(error as Error).message}`)
  }
}

// Update artifact with completion metadata
async function updateArtifactCompletion(
  supabase: any, 
  artifactId: string, 
  metadata: Record<string, any>
) {
  const { error } = await supabase
    .from('artifacts')
    .update({
      status: 'indexed',
      metadata: metadata,
      indexed_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    })
    .eq('id', artifactId)

  if (error) {
    throw new Error(`Failed to update artifact: ${error.message}`)
  }
}

// Update job status
async function updateJobStatus(supabase: any, jobId: string, status: string) {
  const { error } = await supabase
    .from('job_queue')
    .update({
      status: status,
      completed_at: status === 'completed' ? new Date().toISOString() : null,
      updated_at: new Date().toISOString()
    })
    .eq('id', jobId)

  if (error) {
    throw new Error(`Failed to update job status: ${error.message}`)
  }
}

// Update job with error information
async function updateJobWithError(supabase: any, job: JobQueueItem, error: Error) {
  const newRetryCount = job.retry_count + 1
  const shouldRetry = newRetryCount < job.max_retries
  
  const updates: any = {
    retry_count: newRetryCount,
    error_message: error.message,
    updated_at: new Date().toISOString()
  }

  if (shouldRetry) {
    // Schedule retry with exponential backoff
    const backoffMinutes = Math.pow(2, newRetryCount) * 5 // 5, 10, 20 minutes
    const nextRunAt = new Date(Date.now() + backoffMinutes * 60 * 1000)
    
    updates.status = 'retrying'
    updates.next_run_at = nextRunAt.toISOString()
    
    console.log(`🔄 Scheduling retry ${newRetryCount}/${job.max_retries} in ${backoffMinutes} minutes`)
  } else {
    updates.status = 'failed'
    console.log(`💀 Job failed permanently after ${newRetryCount} attempts`)
    
    // Also mark the artifact as failed
    if (job.payload.artifactId) {
      await supabase
        .from('artifacts')
        .update({
          status: 'failed',
          metadata: { error: error.message, failed_at: new Date().toISOString() },
          updated_at: new Date().toISOString()
        })
        .eq('id', job.payload.artifactId)
    }
  }

  const { error: updateError } = await supabase
    .from('job_queue')
    .update(updates)
    .eq('id', job.id)

  if (updateError) {
    console.error('Failed to update job with error:', updateError)
  }
}

// Generate content hash for deduplication
async function generateContentHash(content: string): Promise<string> {
  const encoder = new TextEncoder()
  const data = encoder.encode(content)
  const hashBuffer = await crypto.subtle.digest('SHA-256', data)
  const hashArray = Array.from(new Uint8Array(hashBuffer))
  const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('')
  return hashHex
}

console.log('🚀 Ingestion worker Edge Function initialized')
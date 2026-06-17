-- Enable pgvector extension for vector embeddings
CREATE EXTENSION IF NOT EXISTS vector;

-- Create entity tables for ideas, projects, tasks, and artifacts
CREATE TABLE IF NOT EXISTS ideas (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'spark', -- spark, validation, approved, active, completed, archived
  priority TEXT DEFAULT 'medium', -- low, medium, high
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS projects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'planning', -- planning, active, completed, on_hold
  priority TEXT DEFAULT 'medium', -- low, medium, high
  idea_id UUID REFERENCES ideas(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tasks (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'pending', -- pending, in_progress, completed, blocked
  priority TEXT DEFAULT 'medium', -- low, medium, high
  parent_type TEXT NOT NULL, -- idea, project
  parent_id UUID NOT NULL, -- references ideas or projects
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS artifacts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  description TEXT,
  type TEXT NOT NULL, -- document, link, file, note
  url TEXT,
  content TEXT,
  status TEXT DEFAULT 'processing', -- processing, indexed, failed
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Enable RLS on entity tables
ALTER TABLE ideas ENABLE ROW LEVEL SECURITY;
ALTER TABLE projects ENABLE ROW LEVEL SECURITY;
ALTER TABLE tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE artifacts ENABLE ROW LEVEL SECURITY;

-- RLS policies for entity tables (public read, authenticated write)
CREATE POLICY "public_read_ideas" ON ideas FOR SELECT USING (true);
CREATE POLICY "authenticated_write_ideas" ON ideas FOR ALL TO authenticated USING (true);

CREATE POLICY "public_read_projects" ON projects FOR SELECT USING (true);
CREATE POLICY "authenticated_write_projects" ON projects FOR ALL TO authenticated USING (true);

CREATE POLICY "public_read_tasks" ON tasks FOR SELECT USING (true);
CREATE POLICY "authenticated_write_tasks" ON tasks FOR ALL TO authenticated USING (true);

CREATE POLICY "public_read_artifacts" ON artifacts FOR SELECT USING (true);
CREATE POLICY "authenticated_write_artifacts" ON artifacts FOR ALL TO authenticated USING (true);

-- Create job_queue table for RAG system background processing
CREATE TABLE job_queue (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  job_type TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending', -- pending, running, completed, failed, retrying
  priority INTEGER DEFAULT 0, -- higher numbers = higher priority
  payload JSONB NOT NULL DEFAULT '{}',
  result JSONB,
  error_message TEXT,
  retry_count INTEGER DEFAULT 0,
  max_retries INTEGER DEFAULT 3,
  next_run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_job_queue_status ON job_queue(status);
CREATE INDEX idx_job_queue_next_run_at ON job_queue(next_run_at);
CREATE INDEX idx_job_queue_job_type ON job_queue(job_type);
CREATE INDEX idx_job_queue_priority ON job_queue(priority DESC);
CREATE INDEX idx_job_queue_status_next_run ON job_queue(status, next_run_at) WHERE status IN ('pending', 'retrying');

-- Create updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_job_queue_updated_at 
    BEFORE UPDATE ON job_queue 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Enable Row Level Security
ALTER TABLE job_queue ENABLE ROW LEVEL SECURITY;

-- Create RLS policies - service_role only for background processing
CREATE POLICY "service_role_full_access" ON job_queue
  FOR ALL 
  TO service_role
  USING (true)
  WITH CHECK (true);

-- Deny all access to anon and authenticated users (jobs are internal only)
CREATE POLICY "deny_anon_access" ON job_queue
  FOR ALL 
  TO anon
  USING (false);

CREATE POLICY "deny_authenticated_access" ON job_queue
  FOR ALL 
  TO authenticated
  USING (false);

-- Add embedding and metadata columns to artifacts table for RAG
ALTER TABLE artifacts 
ADD COLUMN IF NOT EXISTS embedding VECTOR(1536), -- OpenAI ada-002 embedding size
ADD COLUMN IF NOT EXISTS metadata JSONB DEFAULT '{}',
ADD COLUMN IF NOT EXISTS content_hash TEXT, -- for deduplication
ADD COLUMN IF NOT EXISTS chunk_index INTEGER, -- for document chunking
ADD COLUMN IF NOT EXISTS parent_artifact_id UUID REFERENCES artifacts(id), -- for chunks
ADD COLUMN IF NOT EXISTS indexed_at TIMESTAMPTZ; -- when embedding was created

-- Create index for vector similarity search
CREATE INDEX IF NOT EXISTS idx_artifacts_embedding ON artifacts 
USING ivfflat (embedding vector_cosine_ops) 
WITH (lists = 100);

-- Create index for metadata queries
CREATE INDEX IF NOT EXISTS idx_artifacts_metadata ON artifacts USING GIN (metadata);

-- Create index for content hash (deduplication)
CREATE INDEX IF NOT EXISTS idx_artifacts_content_hash ON artifacts(content_hash);

-- Create index for parent-child relationships
CREATE INDEX IF NOT EXISTS idx_artifacts_parent ON artifacts(parent_artifact_id);

-- Comment on columns
COMMENT ON COLUMN job_queue.job_type IS 'Type of job: embed_artifact, index_document, similarity_search, etc.';
COMMENT ON COLUMN job_queue.payload IS 'Job-specific data like artifact_id, query, parameters';
COMMENT ON COLUMN job_queue.result IS 'Job output like embedding vector, search results, etc.';
COMMENT ON COLUMN job_queue.priority IS 'Higher numbers processed first';
COMMENT ON COLUMN artifacts.embedding IS 'Vector embedding for similarity search';
COMMENT ON COLUMN artifacts.metadata IS 'Additional metadata like chunk_size, model_used, etc.';
COMMENT ON COLUMN artifacts.content_hash IS 'SHA-256 hash of content for deduplication';
COMMENT ON COLUMN artifacts.chunk_index IS 'Index when artifact is split into chunks';
COMMENT ON COLUMN artifacts.parent_artifact_id IS 'Parent artifact if this is a chunk';
COMMENT ON COLUMN artifacts.indexed_at IS 'When the embedding was last generated';

-- Add some useful job types as examples
INSERT INTO job_queue (job_type, payload, status) VALUES 
('example_embed_artifact', '{"artifact_id": "00000000-0000-0000-0000-000000000000", "model": "text-embedding-ada-002"}', 'pending'),
('example_index_document', '{"url": "https://example.com/doc.pdf", "chunk_size": 1000}', 'pending')
ON CONFLICT DO NOTHING;
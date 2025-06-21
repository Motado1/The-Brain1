-- Remove example jobs from migration
DELETE FROM job_queue WHERE job_type IN ('example_embed_artifact', 'example_index_document');

-- This file can be run to clean up example jobs if needed
-- The actual cron setup needs to be done via Supabase Dashboard or CLI
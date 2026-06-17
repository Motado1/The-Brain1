-- Create storage bucket for knowledge uploads
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES (
  'knowledge',
  'knowledge', 
  true,  -- public for now
  10485760,  -- 10MB limit
  ARRAY['text/plain', 'text/markdown', 'application/pdf', 'text/csv', 'application/json', 'text/html', 'application/msword', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document']
) ON CONFLICT (id) DO NOTHING;

-- Create RLS policies for storage bucket
-- Since bucket is public, allow public access for read
CREATE POLICY "Public read access for knowledge bucket" 
ON storage.objects FOR SELECT 
USING (bucket_id = 'knowledge');

-- Allow public upload for now (since bucket is public)
CREATE POLICY "Public upload access to knowledge bucket" 
ON storage.objects FOR INSERT 
WITH CHECK (bucket_id = 'knowledge');

-- Allow public updates for now
CREATE POLICY "Public update access for knowledge bucket" 
ON storage.objects FOR UPDATE 
USING (bucket_id = 'knowledge');

-- Allow public delete for now
CREATE POLICY "Public delete access for knowledge bucket" 
ON storage.objects FOR DELETE 
USING (bucket_id = 'knowledge');

-- Note: For future private access, replace policies with:
-- CREATE POLICY "Authenticated users can access knowledge bucket" 
-- ON storage.objects FOR ALL 
-- USING (bucket_id = 'knowledge' AND auth.uid() IS NOT NULL);
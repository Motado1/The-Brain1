-- Migration: Add pillar and size columns to visual_nodes
-- Created: 2025-06-20

-- Add is_pillar column with default false
ALTER TABLE visual_nodes 
ADD COLUMN is_pillar boolean NOT NULL DEFAULT false;

-- Add size column with default 1.0
ALTER TABLE visual_nodes 
ADD COLUMN size float NOT NULL DEFAULT 1.0;

-- Create index on is_pillar for efficient pillar queries
CREATE INDEX idx_visual_nodes_is_pillar ON visual_nodes(is_pillar);

-- Update existing pillar entities to have is_pillar = true
UPDATE visual_nodes 
SET is_pillar = true 
WHERE entity_type = 'pillar';

-- Update pillar sizes to be larger (scale factor 3.0)
UPDATE visual_nodes 
SET size = 3.0 
WHERE is_pillar = true;

-- Update project sizes to be medium (scale factor 1.5)
UPDATE visual_nodes 
SET size = 1.5 
WHERE entity_type = 'project';

-- Update idea sizes to be slightly larger (scale factor 1.2)
UPDATE visual_nodes 
SET size = 1.2 
WHERE entity_type = 'idea';

-- Keep tasks and other entities at default size 1.0
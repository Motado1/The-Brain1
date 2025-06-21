-- Database Schema for The Brain Visualization System
-- Following the README specifications exactly

-- Table for visual graph nodes
CREATE TABLE visual_nodes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  entity_type TEXT NOT NULL,    -- e.g. 'pillar', 'idea', 'task', etc.
  entity_id UUID NOT NULL,      -- foreign key reference to the actual entity (idea/task/etc table)
  name TEXT NOT NULL,           -- display label
  x FLOAT, y FLOAT, z FLOAT,    -- coordinates in 3D space
  fx FLOAT, fy FLOAT, fz FLOAT, -- "fixed" coordinates for pinned nodes (optional)
  scale FLOAT DEFAULT 1.0,      -- visual size multiplier
  color TEXT,                   -- hex color or CSS color string for the node
  parent_id UUID REFERENCES visual_nodes(id),  -- parent node (for hierarchical grouping)
  layer INT NOT NULL,           -- hierarchy layer (e.g. 0 for top-level pillars, 1 for child nodes)
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Table for visual graph edges (connections between visual_nodes)
CREATE TABLE visual_edges (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source UUID REFERENCES visual_nodes(id) ON DELETE CASCADE,
  target UUID REFERENCES visual_nodes(id) ON DELETE CASCADE,
  edge_type TEXT,      -- e.g. 'hierarchy', 'reference', 'dependency'
  strength FLOAT DEFAULT 1.0,  -- weight of connection (for physics or visual thickness)
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Enable Row Level Security on tables
ALTER TABLE visual_nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE visual_edges ENABLE ROW LEVEL SECURITY;

-- Allow anyone to read visualization data (public read-only access)
CREATE POLICY "public_read_visual_nodes" ON visual_nodes
  FOR SELECT USING (true);

CREATE POLICY "public_read_visual_edges" ON visual_edges
  FOR SELECT USING (true);

-- Allow authenticated users to modify (insert/update) if needed
CREATE POLICY "authenticated_insert_visual_nodes" ON visual_nodes
  FOR INSERT WITH CHECK (auth.uid() IS NOT NULL);

CREATE POLICY "authenticated_update_visual_nodes" ON visual_nodes
  FOR UPDATE USING (auth.uid() IS NOT NULL);

CREATE POLICY "authenticated_insert_visual_edges" ON visual_edges
  FOR INSERT WITH CHECK (auth.uid() IS NOT NULL);

CREATE POLICY "authenticated_update_visual_edges" ON visual_edges
  FOR UPDATE USING (auth.uid() IS NOT NULL);
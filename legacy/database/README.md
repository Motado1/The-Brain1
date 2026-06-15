# Database Setup

## Supabase Configuration

1. Create a new Supabase project at https://supabase.com
2. In your project's SQL editor, run the schema.sql file to create tables
3. Run the seed.sql file to populate with sample data
4. Update .env.local with your project's URL and anon key from Settings -> API

## Schema Overview

- `visual_nodes`: Stores 3D visualization nodes with position, color, and metadata
- `visual_edges`: Stores connections between nodes

## Row Level Security

The tables have RLS enabled with policies that allow:
- Public read access (for visualization)
- Authenticated user write access (for editing)
-- Sample Data Seeding
-- Insert six pillar nodes (layer 0) as specified in README

INSERT INTO visual_nodes (entity_type, entity_id, name, layer, x, y, z, scale, color) VALUES
  ('pillar', gen_random_uuid(), 'Knowledge Core', 0,   0,   0,   0, 2.0, '#6B46C1'),
  ('pillar', gen_random_uuid(), 'Idea & Project Hub', 0, 100,  0,   0, 2.0, '#EC4899'),
  ('pillar', gen_random_uuid(), 'Action & Task Dashboard', 0, 50,  86,  0, 2.0, '#F59E0B'),
  ('pillar', gen_random_uuid(), 'Learning Ledger', 0,   -50, 86,  0, 2.0, '#10B981'),
  ('pillar', gen_random_uuid(), 'AI Assistant Layer', 0, -100,  0,  0, 2.0, '#3B82F6'),
  ('pillar', gen_random_uuid(), 'Workbench', 0,       -50, -86,  0, 2.0, '#8B5CF6');
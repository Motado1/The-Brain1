const { createClient } = require('@supabase/supabase-js');

const supabase = createClient(
  'http://127.0.0.1:54321',
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU'
);

async function fixResonanceNode() {
  console.log('🔧 Fixing Resonance node...');
  
  // First, find the Idea & Project Hub pillar
  const { data: ideaHub } = await supabase
    .from('visual_nodes')
    .select('*')
    .eq('name', 'Idea & Project Hub')
    .single();
    
  if (!ideaHub) {
    console.error('❌ Could not find Idea & Project Hub pillar');
    return;
  }
  
  console.log('📍 Idea Hub at:', ideaHub.x, ideaHub.y);
  
  // Calculate a visible position near the Idea Hub
  const newX = ideaHub.x + 25; // 25 units to the right
  const newY = ideaHub.y + 0;  // Same Y level
  const newZ = 0;              // At Z level 0
  
  // Update the Resonance node
  const { data, error } = await supabase
    .from('visual_nodes')
    .update({
      x: newX,
      y: newY,
      z: newZ,
      layer: 1,
      size: 1.2,
      color: '#ffaa00' // Make it bright yellow like other ideas
    })
    .eq('name', 'Resonance')
    .select();
    
  if (error) {
    console.error('❌ Error updating Resonance:', error);
  } else {
    console.log('✅ Resonance node updated:', data);
    console.log(`📍 New position: (${newX}, ${newY}, ${newZ})`);
  }
}

fixResonanceNode().catch(console.error);
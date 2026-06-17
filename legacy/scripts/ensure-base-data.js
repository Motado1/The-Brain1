const { createClient } = require('@supabase/supabase-js');

const supabase = createClient(
  'http://127.0.0.1:54321',
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU'
);

async function ensureBaseData() {
  console.log('🧠 Ensuring base data exists (without clearing existing data)...');
  
  // Check if pillars exist
  const { data: existingPillars } = await supabase
    .from('visual_nodes')
    .select('*')
    .eq('entity_type', 'pillar');
    
  console.log(`Found ${existingPillars?.length || 0} existing pillars`);
  
  // Only create pillars if they don't exist
  if (!existingPillars || existingPillars.length === 0) {
    console.log('Creating base pillar structure...');
    
    const basePillars = [
      { name: 'Knowledge Core', x: 0, y: 0, color: '#6B46C1' },
      { name: 'Idea & Project Hub', x: 100, y: 0, color: '#EC4899' },
      { name: 'Action & Task Dashboard', x: 50, y: 86, color: '#F59E0B' },
      { name: 'Learning Ledger', x: -50, y: 86, color: '#10B981' },
      { name: 'AI Assistant Layer', x: -100, y: 0, color: '#3B82F6' },
      { name: 'Workbench', x: -50, y: -86, color: '#8B5CF6' }
    ];
    
    for (const pillar of basePillars) {
      // Create entity first
      const { data: entityData } = await supabase.from('entities').insert({
        name: pillar.name,
        type: 'pillar',
        description: `Core pillar: ${pillar.name}`
      }).select().single();
      
      if (entityData) {
        // Create visual node
        await supabase.from('visual_nodes').insert({
          entity_type: 'pillar',
          entity_id: entityData.id,
          name: pillar.name,
          x: pillar.x,
          y: pillar.y,
          z: 0,
          layer: 0,
          color: pillar.color,
          scale: 2,
          is_pillar: true,
          size: 3.0
        });
      }
    }
    console.log('✅ Base pillars created');
  } else {
    console.log('✅ Pillars already exist, skipping creation');
  }
  
  // Check if we have enough non-pillar nodes for a good demo
  const { data: existingNodes } = await supabase
    .from('visual_nodes')
    .select('*')
    .neq('entity_type', 'pillar');
    
  console.log(`Found ${existingNodes?.length || 0} existing non-pillar nodes`);
  
  // Only add demo nodes if we have very few
  if (!existingNodes || existingNodes.length < 5) {
    console.log('Adding some demo nodes for neural network effect...');
    
    const ideaHub = existingPillars?.find(p => p.name === 'Idea & Project Hub') || 
                   (await supabase.from('visual_nodes').select('*').eq('name', 'Idea & Project Hub').single()).data;
    const taskDashboard = existingPillars?.find(p => p.name === 'Action & Task Dashboard') ||
                         (await supabase.from('visual_nodes').select('*').eq('name', 'Action & Task Dashboard').single()).data;
    
    if (ideaHub && taskDashboard) {
      // Add a few demo nodes
      const demoNodes = [
        { name: 'Neural Visualization System', type: 'idea', parent: ideaHub.id, color: '#fbbf24', size: 1.2 },
        { name: 'Knowledge Graph Engine', type: 'project', parent: ideaHub.id, color: '#10b981', size: 1.5 },
        { name: 'Implement Smooth Synapses', type: 'task', parent: taskDashboard.id, color: '#3b82f6', size: 1.0 },
        { name: 'Add Node Clustering', type: 'task', parent: taskDashboard.id, color: '#3b82f6', size: 1.0 }
      ];
      
      for (const node of demoNodes) {
        // Create entity
        const { data: entityData } = await supabase.from('ideas').insert({
          name: node.name,
          description: `Demo ${node.type}: ${node.name}`,
          status: 'active',
          priority: 'medium'
        }).select().single();
        
        if (entityData) {
          await supabase.from('visual_nodes').insert({
            entity_type: node.type,
            entity_id: entityData.id,
            name: node.name,
            parent_id: node.parent,
            layer: 1,
            color: node.color,
            scale: 1.0,
            is_pillar: false,
            size: node.size
          });
        }
      }
      console.log('✅ Demo nodes added');
    }
  } else {
    console.log('✅ Sufficient nodes exist, skipping demo creation');
  }
  
  console.log('🎉 Base data setup complete! Your Brain is ready to grow.');
}

// Only run if called directly
if (require.main === module) {
  ensureBaseData().catch(console.error);
}

module.exports = { ensureBaseData };
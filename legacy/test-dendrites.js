const { createClient } = require('@supabase/supabase-js');

const supabase = createClient(
  'http://127.0.0.1:54321',
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU'
);

async function checkPillarNodes() {
  console.log('\n🧪 Testing Dendrite Rendering Setup\n');
  
  // 1. Check pillar nodes
  const { data: pillars, error } = await supabase
    .from('visual_nodes')
    .select('*')
    .eq('is_pillar', true);
  
  if (error) {
    console.error('❌ Error fetching pillars:', error);
    return;
  }
  
  console.log(`✅ Found ${pillars.length} pillar nodes in database`);
  
  // 2. Display pillar details
  console.log('\n📊 Pillar Node Details:');
  pillars.forEach(pillar => {
    console.log(`\n- ${pillar.name}:`);
    console.log(`  • ID: ${pillar.id}`);
    console.log(`  • Entity Type: ${pillar.entity_type}`);
    console.log(`  • is_pillar: ${pillar.is_pillar}`);
    console.log(`  • Position: (${pillar.x}, ${pillar.y}, ${pillar.z})`);
    console.log(`  • Scale: ${pillar.scale}`);
    console.log(`  • Size: ${pillar.size}`);
  });
  
  // 3. Check for any nodes with entity_type = 'pillar' but is_pillar != true
  const { data: inconsistent } = await supabase
    .from('visual_nodes')
    .select('*')
    .eq('entity_type', 'pillar')
    .neq('is_pillar', true);
    
  if (inconsistent && inconsistent.length > 0) {
    console.log('\n⚠️  Found inconsistent pillar nodes:');
    inconsistent.forEach(node => {
      console.log(`- ${node.name}: entity_type='pillar' but is_pillar=${node.is_pillar}`);
    });
  }
  
  // 4. Summary
  console.log('\n📝 Summary:');
  console.log('- Dendrites should render on all pillar nodes');
  console.log('- Each pillar should show 6 orange dendrite arms');
  console.log('- Dendrites use the MultiPulseMaterial with orange colors');
  console.log('- Check browser console for "🌟 Rendering pillar node:" logs');
  
  console.log('\n💡 If dendrites are not visible:');
  console.log('1. Check browser console for errors');
  console.log('2. Verify Three.js scene is rendering properly');
  console.log('3. Check if Dendrites component is being mounted');
  console.log('4. Verify materials are loading correctly');
}

checkPillarNodes().catch(console.error);
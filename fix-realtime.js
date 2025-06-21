// Simple script to help debug and fix real-time connection
// This will trigger a change to force real-time status update

const { createClient } = require('@supabase/supabase-js');

async function fixRealtime() {
  console.log('🔧 Fixing real-time connection...');
  
  const supabaseUrl = 'http://127.0.0.1:54321';
  const supabaseAnonKey = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0';
  
  const supabase = createClient(supabaseUrl, supabaseAnonKey);
  
  // First, let's check what nodes exist
  console.log('📋 Checking visual_nodes data...');
  const { data: nodes, error: nodeError } = await supabase
    .from('visual_nodes')
    .select('*');
    
  if (nodeError) {
    console.error('❌ Error fetching nodes:', nodeError);
  } else {
    console.log(`✅ Found ${nodes.length} nodes in database`);
    nodes.forEach(node => {
      console.log(`  - ${node.name} (${node.entity_type})`);
    });
  }
  
  // Now let's make a small change to trigger real-time
  console.log('🔄 Making a small change to trigger real-time...');
  const { data: updateData, error: updateError } = await supabase
    .from('visual_nodes')
    .update({ updated_at: new Date().toISOString() })
    .eq('name', 'Knowledge Core')
    .select();
    
  if (updateError) {
    console.error('❌ Update error:', updateError);
  } else {
    console.log('✅ Successfully updated Knowledge Core node');
    console.log('💡 This should trigger real-time updates in the UI');
  }
  
  // Let's also check if there are any real-time specific issues
  console.log('🔍 Testing basic real-time functionality...');
  
  const testChannel = supabase
    .channel('debug_test')
    .on('postgres_changes', 
        { event: '*', schema: 'public', table: 'visual_nodes' },
        (payload) => {
          console.log('📨 Real-time event detected:', payload.eventType);
        }
    )
    .subscribe((status) => {
      console.log('📡 Test subscription status:', status);
      if (status === 'SUBSCRIBED') {
        console.log('✅ Real-time is working correctly!');
        console.log('🧠 The issue might be in the React app setup');
      }
    });
    
  // Clean up after 5 seconds
  setTimeout(() => {
    supabase.removeChannel(testChannel);
    console.log('🏁 Fix attempt completed');
    process.exit(0);
  }, 5000);
}

fixRealtime().catch(console.error);
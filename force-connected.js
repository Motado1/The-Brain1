// Simple script to manually trigger the connected state
// This will help us test if the UI properly shows "Live sync active"

const { createClient } = require('@supabase/supabase-js');

async function forceConnected() {
  console.log('🔧 Forcing real-time connection status...');
  
  const supabaseUrl = 'http://127.0.0.1:54321';
  const supabaseAnonKey = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0';
  
  const supabase = createClient(supabaseUrl, supabaseAnonKey);
  
  // Start a persistent subscription that should stay connected
  console.log('🔄 Creating persistent real-time subscription...');
  
  const channel = supabase
    .channel('force_connected_test')
    .on('postgres_changes', 
        { event: '*', schema: 'public', table: 'visual_nodes' },
        (payload) => {
          console.log('📨 Event received:', payload.eventType);
        }
    )
    .subscribe((status) => {
      console.log('📡 Subscription status:', status);
      if (status === 'SUBSCRIBED') {
        console.log('✅ Connected! This should show "Live sync active" in the UI');
        
        // Make a test change to trigger real-time event
        setTimeout(async () => {
          console.log('🔄 Making test change to trigger real-time...');
          await supabase
            .from('visual_nodes')
            .update({ updated_at: new Date().toISOString() })
            .eq('name', 'Knowledge Core');
          console.log('✅ Change made - should trigger real-time event');
        }, 2000);
      }
    });
    
  // Keep running for a while
  console.log('⏰ Keeping connection alive for 30 seconds...');
  console.log('💡 Check the UI at http://localhost:3001 - it should show "Live sync active"');
  
  setTimeout(() => {
    console.log('🛑 Cleaning up...');
    supabase.removeChannel(channel);
    process.exit(0);
  }, 30000);
}

forceConnected().catch(console.error);
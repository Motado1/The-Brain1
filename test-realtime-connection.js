// Test script to debug real-time WebSocket connection
// This will help us understand why "Sync disconnected" is showing

const { createClient } = require('@supabase/supabase-js');

async function testRealtimeConnection() {
  console.log('🧪 Testing Supabase Real-time Connection...');
  
  const supabaseUrl = 'http://127.0.0.1:54321';
  const supabaseAnonKey = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0';
  
  const supabase = createClient(supabaseUrl, supabaseAnonKey, {
    realtime: {
      params: {
        eventsPerSecond: 10,
      },
    },
  });
  
  console.log('📡 Creating real-time subscription...');
  
  // Test subscription to visual_nodes (same as the app does)
  const channel = supabase
    .channel('test_visual_nodes_changes')
    .on(
      'postgres_changes',
      { 
        event: '*', 
        schema: 'public', 
        table: 'visual_nodes' 
      },
      (payload) => {
        console.log('📨 Real-time event received:', payload);
      }
    )
    .subscribe((status) => {
      console.log('📡 Subscription status:', status);
      
      if (status === 'SUBSCRIBED') {
        console.log('✅ Real-time connection successful!');
        console.log('🔄 Now testing by making a change to visual_nodes...');
        
        // Make a test change to trigger real-time event
        setTimeout(async () => {
          try {
            const { data, error } = await supabase
              .from('visual_nodes')
              .update({ updated_at: new Date().toISOString() })
              .eq('name', 'Knowledge Core')
              .select();
            
            if (error) {
              console.error('❌ Update failed:', error);
            } else {
              console.log('✅ Update successful, should trigger real-time event');
            }
          } catch (err) {
            console.error('❌ Update error:', err);
          }
        }, 2000);
        
      } else if (status === 'CHANNEL_ERROR') {
        console.error('❌ Real-time subscription failed');
      } else if (status === 'TIMED_OUT') {
        console.error('⏰ Real-time subscription timed out');
      } else if (status === 'CLOSED') {
        console.log('🔒 Real-time subscription closed');
      }
    });
  
  // Keep the test running for 10 seconds
  setTimeout(() => {
    console.log('🛑 Cleaning up test...');
    supabase.removeChannel(channel);
    console.log('✅ Test completed');
    process.exit(0);
  }, 10000);
}

testRealtimeConnection().catch(console.error);
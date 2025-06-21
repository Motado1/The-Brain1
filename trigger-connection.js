// Script to trigger real-time events and help establish connection

const { createClient } = require('@supabase/supabase-js');

async function triggerConnection() {
  console.log('🔧 Triggering real-time connection...');
  
  const supabaseUrl = 'http://172.29.160.1:54321';
  const supabaseAnonKey = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0';
  
  const supabase = createClient(supabaseUrl, supabaseAnonKey);
  
  console.log('🔄 Creating persistent subscription...');
  
  const channel = supabase
    .channel('connection_helper')
    .on('postgres_changes', 
        { event: '*', schema: 'public', table: 'visual_nodes' },
        (payload) => {
          console.log('📨 Real-time event triggered!', payload.eventType);
        }
    )
    .subscribe((status) => {
      console.log('📡 Connection status:', status);
      if (status === 'SUBSCRIBED') {
        console.log('✅ Connection established!');
        console.log('🔄 Making a change to trigger real-time...');
        
        // Make repeated changes to help establish connection
        let count = 0;
        const interval = setInterval(async () => {
          count++;
          console.log(`🔄 Trigger ${count}: Updating node...`);
          
          await supabase
            .from('visual_nodes')
            .update({ updated_at: new Date().toISOString() })
            .eq('name', 'Knowledge Core');
            
          if (count >= 5) {
            clearInterval(interval);
            console.log('✅ Completed connection triggers');
          }
        }, 1000);
      }
    });
    
  console.log('⏰ Running for 15 seconds to help establish connection...');
  console.log('💡 Check http://localhost:3001 - refresh the page if needed');
  
  setTimeout(() => {
    console.log('🛑 Cleaning up...');
    supabase.removeChannel(channel);
    console.log('✅ Connection helper completed');
    process.exit(0);
  }, 15000);
}

triggerConnection().catch(console.error);
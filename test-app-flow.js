// Test script that mimics the exact flow the React app should follow
// This will help identify where the disconnect happens

const { createClient } = require('@supabase/supabase-js');

async function testAppFlow() {
  console.log('🧪 Testing App Flow (mimicking CanvasScene + InfoPanel)...');
  
  const supabaseUrl = 'http://127.0.0.1:54321';
  const supabaseAnonKey = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0';
  
  const supabase = createClient(supabaseUrl, supabaseAnonKey);
  
  let isRealTimeConnected = false;
  
  // Step 1: Start real-time sync (like CanvasScene does)
  console.log('🔄 Starting real-time sync...');
  
  const nodeSubscription = supabase
    .channel('visual_nodes_changes')
    .on(
      'postgres_changes',
      { 
        event: '*', 
        schema: 'public', 
        table: 'visual_nodes' 
      },
      (payload) => {
        console.log('📡 Node change received:', payload);
      }
    )
    .subscribe((status) => {
      console.log('📡 Node subscription status:', status);
      if (status === 'SUBSCRIBED') {
        console.log('✅ Node subscription successful - setting connected to true');
        isRealTimeConnected = true;
      } else if (status === 'CHANNEL_ERROR') {
        console.error('❌ Node subscription error');
        isRealTimeConnected = false;
      } else if (status === 'TIMED_OUT') {
        console.error('⏰ Node subscription timed out');
        isRealTimeConnected = false;
      } else if (status === 'CLOSED') {
        console.log('🔒 Node subscription closed');
        isRealTimeConnected = false;
      } else {
        console.log('📡 Node subscription status changed:', status);
      }
      
      console.log('🌐 Real-time connected status:', isRealTimeConnected);
    });
  
  const edgeSubscription = supabase
    .channel('visual_edges_changes')
    .on(
      'postgres_changes',
      { 
        event: '*', 
        schema: 'public', 
        table: 'visual_edges' 
      },
      (payload) => {
        console.log('📡 Edge change received:', payload);
      }
    )
    .subscribe((status) => {
      console.log('📡 Edge subscription status:', status);
    });
  
  // Step 2: Fetch visual nodes (like CanvasScene does)
  console.log('📋 Fetching visual nodes...');
  
  try {
    const res = await supabase.from('visual_nodes').select('*');
    
    if (res.error) {
      console.error('Error loading visual_nodes:', res.error);
      console.log('This would fall back to mock data');
    } else {
      console.log('✅ Loaded', res.data?.length, 'nodes from Supabase');
      console.log('📊 Node data preview:', res.data?.slice(0, 2));
    }
  } catch (error) {
    console.error('❌ Fetch error:', error);
  }
  
  // Step 3: Fetch visual edges
  console.log('📋 Fetching visual edges...');
  
  try {
    const res = await supabase.from('visual_edges').select('*');
    
    if (res.error) {
      console.error('Error loading visual_edges:', res.error);
    } else {
      console.log('✅ Loaded', res.data?.length, 'edges from Supabase');
    }
  } catch (error) {
    console.error('❌ Edge fetch error:', error);
  }
  
  // Wait a moment for subscriptions to establish
  setTimeout(() => {
    console.log('\n📊 Final Status Summary:');
    console.log('- Real-time Connected:', isRealTimeConnected);
    console.log('- Node Subscription:', nodeSubscription.state);
    console.log('- Edge Subscription:', edgeSubscription.state);
    
    if (isRealTimeConnected) {
      console.log('✅ Real-time should be working in the UI');
    } else {
      console.log('❌ Real-time connection issue detected');
    }
    
    // Cleanup
    supabase.removeChannel(nodeSubscription);
    supabase.removeChannel(edgeSubscription);
    console.log('🏁 Test completed');
    process.exit(0);
  }, 3000);
}

testAppFlow().catch(console.error);
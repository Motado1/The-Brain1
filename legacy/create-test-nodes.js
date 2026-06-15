const { createClient } = require('@supabase/supabase-js');

const supabase = createClient(
  'http://127.0.0.1:54321',
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImV4cCI6MTk4MzgxMjk5Nn0.EGIM96RAZx35lJzdJsyH-qQwv8Hdp7fsn3W0YpN81IU'
);

async function createTestNodes() {
  console.log('Creating test nodes for neural network visualization...');
  
  // Get pillar IDs
  const { data: pillars } = await supabase.from('visual_nodes').select('*').eq('entity_type', 'pillar');
  const ideaHub = pillars.find(p => p.name === 'Idea & Project Hub');
  const taskDashboard = pillars.find(p => p.name === 'Action & Task Dashboard');
  const knowledgeCore = pillars.find(p => p.name === 'Knowledge Core');
  
  // Create some ideas
  const ideasToCreate = [
    { name: 'Neural Knowledge Graph', description: 'Build a 3D knowledge visualization system', parent_pillar: ideaHub.id },
    { name: 'AI-Powered Assistant', description: 'Create an intelligent assistant for knowledge navigation', parent_pillar: ideaHub.id },
    { name: 'Real-time Collaboration', description: 'Enable multiple users to collaborate on knowledge graphs', parent_pillar: ideaHub.id },
  ];
  
  // Create some projects  
  const projectsToCreate = [
    { name: 'Brain Visualization Engine', description: 'Core 3D rendering system', parent_pillar: ideaHub.id },
    { name: 'Knowledge Processing Pipeline', description: 'RAG system for document processing', parent_pillar: taskDashboard.id },
  ];
  
  // Create some tasks
  const tasksToCreate = [
    { name: 'Implement D3 Force Simulation', description: 'Add physics-based node positioning', parent_pillar: taskDashboard.id },
    { name: 'Create Shader Materials', description: 'Build glowing neural materials', parent_pillar: taskDashboard.id },
    { name: 'Add Edge Pulse Animation', description: 'Animate synaptic connections', parent_pillar: taskDashboard.id },
  ];
  
  // Create some artifacts
  const artifactsToCreate = [
    { name: 'Neural Aesthetics Guide', description: 'Design principles for brain visualization', parent_pillar: knowledgeCore.id },
    { name: 'Force Physics Documentation', description: 'D3 simulation implementation notes', parent_pillar: knowledgeCore.id },
  ];
  
  // Insert ideas
  for (const idea of ideasToCreate) {
    const { data: ideaData } = await supabase.from('ideas').insert({
      name: idea.name,
      description: idea.description,
      status: 'active',
      priority: 'high'
    }).select().single();
    
    if (ideaData) {
      await supabase.from('visual_nodes').insert({
        entity_type: 'idea',
        entity_id: ideaData.id,
        name: idea.name,
        parent_id: idea.parent_pillar,
        layer: 1,
        color: '#fbbf24',
        scale: 1.0,
        is_pillar: false,
        size: 1.2
      });
    }
  }
  
  // Insert projects
  for (const project of projectsToCreate) {
    const { data: projectData } = await supabase.from('projects').insert({
      name: project.name,
      description: project.description,
      status: 'active',
      priority: 'high'
    }).select().single();
    
    if (projectData) {
      await supabase.from('visual_nodes').insert({
        entity_type: 'project',
        entity_id: projectData.id,
        name: project.name,
        parent_id: project.parent_pillar,
        layer: 1,
        color: '#10b981',
        scale: 1.0,
        is_pillar: false,
        size: 1.5
      });
    }
  }
  
  // Insert tasks
  for (const task of tasksToCreate) {
    const { data: taskData } = await supabase.from('tasks').insert({
      title: task.name,
      description: task.description,
      status: 'pending',
      priority: 'high',
      parent_type: 'project',
      parent_id: '00000000-0000-0000-0000-000000000000' // dummy parent
    }).select().single();
    
    if (taskData) {
      await supabase.from('visual_nodes').insert({
        entity_type: 'task',
        entity_id: taskData.id,
        name: task.name,
        parent_id: task.parent_pillar,
        layer: 2,
        color: '#3b82f6',
        scale: 1.0,
        is_pillar: false,
        size: 1.0
      });
    }
  }
  
  // Insert artifacts
  for (const artifact of artifactsToCreate) {
    const { data: artifactData } = await supabase.from('artifacts').insert({
      name: artifact.name,
      description: artifact.description,
      type: 'document',
      status: 'indexed'
    }).select().single();
    
    if (artifactData) {
      await supabase.from('visual_nodes').insert({
        entity_type: 'artifact',
        entity_id: artifactData.id,
        name: artifact.name,
        parent_id: artifact.parent_pillar,
        layer: 1,
        color: '#8b5cf6',
        scale: 1.0,
        is_pillar: false,
        size: 1.0
      });
    }
  }
  
  console.log('Test nodes created successfully!');
  
  // Create some test edges
  console.log('Creating test edges...');
  const { data: allNodes } = await supabase.from('visual_nodes').select('*');
  
  const testEdges = [
    { source: allNodes.find(n => n.name === 'Neural Knowledge Graph')?.id, target: allNodes.find(n => n.name === 'Brain Visualization Engine')?.id, edge_type: 'hierarchy' },
    { source: allNodes.find(n => n.name === 'AI-Powered Assistant')?.id, target: allNodes.find(n => n.name === 'Knowledge Processing Pipeline')?.id, edge_type: 'dependency' },
    { source: allNodes.find(n => n.name === 'Brain Visualization Engine')?.id, target: allNodes.find(n => n.name === 'Implement D3 Force Simulation')?.id, edge_type: 'contains' },
    { source: allNodes.find(n => n.name === 'Brain Visualization Engine')?.id, target: allNodes.find(n => n.name === 'Create Shader Materials')?.id, edge_type: 'contains' },
    { source: allNodes.find(n => n.name === 'Neural Aesthetics Guide')?.id, target: allNodes.find(n => n.name === 'Create Shader Materials')?.id, edge_type: 'reference' },
  ];
  
  for (const edge of testEdges) {
    if (edge.source && edge.target) {
      await supabase.from('visual_edges').insert({
        source: edge.source,
        target: edge.target,
        edge_type: edge.edge_type,
        strength: 1.0
      });
    }
  }
  
  console.log('Test edges created successfully!');
  console.log('Neural network data is ready! Refresh your browser to see the full effect.');
}

createTestNodes().catch(console.error);
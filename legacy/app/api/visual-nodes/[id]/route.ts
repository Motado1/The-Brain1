import { NextRequest, NextResponse } from 'next/server';
import { supabase } from '@/lib/supabaseClient';
import { NodeData } from '@/lib/types';

// GET /api/visual-nodes/[id] - Get specific visual node
export async function GET(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const { data, error } = await supabase
      .from('visual_nodes')
      .select('*')
      .eq('id', params.id)
      .single();
    
    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Visual node not found' },
          { status: 404 }
        );
      }
      console.error('Error fetching visual node:', error);
      return NextResponse.json(
        { error: 'Failed to fetch visual node', details: error.message },
        { status: 500 }
      );
    }
    
    return NextResponse.json({ data });
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}

// PUT /api/visual-nodes/[id] - Update specific visual node
export async function PUT(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const body: Partial<NodeData> = await request.json();
    
    // Validate required fields
    if (body.name !== undefined && (!body.name || !body.name.trim())) {
      return NextResponse.json(
        { error: 'Name cannot be empty' },
        { status: 400 }
      );
    }
    
    const updateData: any = {
      updated_at: new Date().toISOString(),
    };
    
    if (body.name) updateData.name = body.name.trim();
    if (body.entity_type) updateData.entity_type = body.entity_type;
    if (body.entity_id) updateData.entity_id = body.entity_id;
    if (body.layer !== undefined) updateData.layer = body.layer;
    if (body.x !== undefined) updateData.x = body.x;
    if (body.y !== undefined) updateData.y = body.y;
    if (body.z !== undefined) updateData.z = body.z;
    if (body.fx !== undefined) updateData.fx = body.fx;
    if (body.fy !== undefined) updateData.fy = body.fy;
    if (body.fz !== undefined) updateData.fz = body.fz;
    if (body.scale !== undefined) updateData.scale = body.scale;
    if (body.color !== undefined) updateData.color = body.color;
    if (body.parent_id !== undefined) updateData.parent_id = body.parent_id;
    
    const { data, error } = await supabase
      .from('visual_nodes')
      .update(updateData)
      .eq('id', params.id)
      .select()
      .single();
    
    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Visual node not found' },
          { status: 404 }
        );
      }
      console.error('Error updating visual node:', error);
      return NextResponse.json(
        { error: 'Failed to update visual node', details: error.message },
        { status: 500 }
      );
    }
    
    return NextResponse.json({ data });
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}

// DELETE /api/visual-nodes/[id] - Delete specific visual node
export async function DELETE(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const { error } = await supabase
      .from('visual_nodes')
      .delete()
      .eq('id', params.id);
    
    if (error) {
      console.error('Error deleting visual node:', error);
      return NextResponse.json(
        { error: 'Failed to delete visual node', details: error.message },
        { status: 500 }
      );
    }
    
    return NextResponse.json({ message: 'Visual node deleted successfully' });
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}
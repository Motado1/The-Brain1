import { NextRequest, NextResponse } from 'next/server';
import { supabase } from '@/lib/supabaseClient';
import { NodeData } from '@/lib/types';

// GET /api/visual-nodes - List all visual nodes
export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const entity_type = searchParams.get('entity_type');
    const layer = searchParams.get('layer');
    
    let query = supabase
      .from('visual_nodes')
      .select('*')
      .order('layer', { ascending: true })
      .order('created_at', { ascending: false });
    
    if (entity_type) {
      query = query.eq('entity_type', entity_type);
    }
    if (layer) {
      query = query.eq('layer', parseInt(layer));
    }
    
    const { data, error } = await query;
    
    if (error) {
      console.error('Error fetching visual nodes:', error);
      return NextResponse.json(
        { error: 'Failed to fetch visual nodes', details: error.message },
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

// POST /api/visual-nodes - Create new visual node
export async function POST(request: NextRequest) {
  try {
    const body: Partial<NodeData> = await request.json();
    
    // Validate required fields
    if (!body.name || !body.name.trim()) {
      return NextResponse.json(
        { error: 'Name is required' },
        { status: 400 }
      );
    }
    
    if (!body.entity_type) {
      return NextResponse.json(
        { error: 'Entity type is required' },
        { status: 400 }
      );
    }
    
    if (!body.entity_id) {
      return NextResponse.json(
        { error: 'Entity ID is required' },
        { status: 400 }
      );
    }
    
    if (body.layer === undefined || body.layer === null) {
      return NextResponse.json(
        { error: 'Layer is required' },
        { status: 400 }
      );
    }
    
    const nodeData = {
      name: body.name.trim(),
      entity_type: body.entity_type,
      entity_id: body.entity_id,
      layer: body.layer,
      x: body.x || null,
      y: body.y || null,
      z: body.z || null,
      fx: body.fx || null,
      fy: body.fy || null,
      fz: body.fz || null,
      scale: body.scale || 1.0,
      color: body.color || null,
      parent_id: body.parent_id || null,
    };
    
    const { data, error } = await supabase
      .from('visual_nodes')
      .insert([nodeData])
      .select()
      .single();
    
    if (error) {
      console.error('Error creating visual node:', error);
      return NextResponse.json(
        { error: 'Failed to create visual node', details: error.message },
        { status: 500 }
      );
    }
    
    return NextResponse.json({ data }, { status: 201 });
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}
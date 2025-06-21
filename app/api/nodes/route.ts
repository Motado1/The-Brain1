import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@supabase/supabase-js';
import { NodeData } from '../../../lib/types';
import { randomUUID } from 'crypto';

// Use service role for API operations to bypass RLS
const supabase = createClient(
  process.env.NEXT_PUBLIC_SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_ROLE_KEY!
);

export async function GET() {
  try {
    const { data, error } = await supabase
      .from('visual_nodes')
      .select('*')
      .order('created_at', { ascending: false });

    if (error) {
      console.error('Error fetching nodes:', error);
      return NextResponse.json(
        { error: 'Failed to fetch nodes' },
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

export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    
    // Validate required fields
    if (!body.name || !body.entity_type) {
      return NextResponse.json(
        { error: 'Missing required fields: name, entity_type' },
        { status: 400 }
      );
    }

    // Generate entity_id if not provided, or validate if provided
    let entityId = body.entity_id;
    if (!entityId) {
      entityId = randomUUID();
    } else {
      // Validate UUID format
      const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
      if (!uuidRegex.test(entityId)) {
        entityId = randomUUID();
      }
    }

    // Prepare node data with defaults (only database fields)
    const nodeData = {
      name: body.name,
      entity_type: body.entity_type,
      entity_id: entityId,
      x: body.x || 0,
      y: body.y || 0,
      z: body.z || 0,
      fx: body.fx || null,
      fy: body.fy || null,
      fz: body.fz || null,
      scale: body.scale || 1.0,
      color: body.color || '#3b82f6',
      parent_id: body.parent_id || null,
      layer: body.layer || 0,
    };

    const { data, error } = await supabase
      .from('visual_nodes')
      .insert([nodeData])
      .select()
      .single();

    if (error) {
      console.error('Error creating node:', error);
      return NextResponse.json(
        { error: 'Failed to create node' },
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
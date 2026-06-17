import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@supabase/supabase-js';
import { NodeData } from '../../../../lib/types';

// Use service role for API operations to bypass RLS
const supabase = createClient(
  process.env.NEXT_PUBLIC_SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_ROLE_KEY!
);

export async function GET(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const { id } = params;

    if (!id) {
      return NextResponse.json(
        { error: 'Node ID is required' },
        { status: 400 }
      );
    }

    const { data, error } = await supabase
      .from('visual_nodes')
      .select('*')
      .eq('id', id)
      .single();

    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Node not found' },
          { status: 404 }
        );
      }
      console.error('Error fetching node:', error);
      return NextResponse.json(
        { error: 'Failed to fetch node' },
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

export async function PUT(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const { id } = params;
    const body = await request.json();

    if (!id) {
      return NextResponse.json(
        { error: 'Node ID is required' },
        { status: 400 }
      );
    }

    // Extract updatable fields (only database fields)
    const updateData: any = {};
    
    const allowedFields = [
      'name', 'entity_type', 'entity_id', 'x', 'y', 'z', 'fx', 'fy', 'fz',
      'scale', 'color', 'parent_id', 'layer'
    ];

    allowedFields.forEach(field => {
      if (body[field] !== undefined) {
        updateData[field] = body[field];
      }
    });

    if (Object.keys(updateData).length === 0) {
      return NextResponse.json(
        { error: 'No valid fields to update' },
        { status: 400 }
      );
    }

    const { data, error } = await supabase
      .from('visual_nodes')
      .update(updateData)
      .eq('id', id)
      .select()
      .single();

    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Node not found' },
          { status: 404 }
        );
      }
      console.error('Error updating node:', error);
      return NextResponse.json(
        { error: 'Failed to update node' },
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

export async function DELETE(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const { id } = params;

    if (!id) {
      return NextResponse.json(
        { error: 'Node ID is required' },
        { status: 400 }
      );
    }

    // First check if node exists
    const { data: existingNode, error: fetchError } = await supabase
      .from('visual_nodes')
      .select('id')
      .eq('id', id)
      .single();

    if (fetchError) {
      if (fetchError.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Node not found' },
          { status: 404 }
        );
      }
      console.error('Error checking node existence:', fetchError);
      return NextResponse.json(
        { error: 'Failed to check node existence' },
        { status: 500 }
      );
    }

    // Delete the node
    const { error } = await supabase
      .from('visual_nodes')
      .delete()
      .eq('id', id);

    if (error) {
      console.error('Error deleting node:', error);
      return NextResponse.json(
        { error: 'Failed to delete node' },
        { status: 500 }
      );
    }

    return NextResponse.json(
      { message: 'Node deleted successfully' },
      { status: 200 }
    );
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}
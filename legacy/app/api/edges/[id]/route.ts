import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@supabase/supabase-js';
import { EdgeData } from '../../../../lib/types';

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
        { error: 'Edge ID is required' },
        { status: 400 }
      );
    }

    const { data, error } = await supabase
      .from('visual_edges')
      .select('*')
      .eq('id', id)
      .single();

    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Edge not found' },
          { status: 404 }
        );
      }
      console.error('Error fetching edge:', error);
      return NextResponse.json(
        { error: 'Failed to fetch edge' },
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
        { error: 'Edge ID is required' },
        { status: 400 }
      );
    }

    // Extract updatable fields
    const updateData: Partial<EdgeData> = {};
    
    const allowedFields = ['source', 'target', 'edge_type', 'strength'];

    allowedFields.forEach(field => {
      if (body[field] !== undefined) {
        (updateData as any)[field] = body[field];
      }
    });

    if (Object.keys(updateData).length === 0) {
      return NextResponse.json(
        { error: 'No valid fields to update' },
        { status: 400 }
      );
    }

    // If updating source or target, verify nodes exist
    if (updateData.source) {
      const { data: sourceNode, error: sourceError } = await supabase
        .from('visual_nodes')
        .select('id')
        .eq('id', updateData.source)
        .single();

      if (sourceError || !sourceNode) {
        return NextResponse.json(
          { error: 'Source node not found' },
          { status: 400 }
        );
      }
    }

    if (updateData.target) {
      const { data: targetNode, error: targetError } = await supabase
        .from('visual_nodes')
        .select('id')
        .eq('id', updateData.target)
        .single();

      if (targetError || !targetNode) {
        return NextResponse.json(
          { error: 'Target node not found' },
          { status: 400 }
        );
      }
    }

    // If updating both source and target, check for duplicate edge
    if (updateData.source && updateData.target) {
      const { data: existingEdge } = await supabase
        .from('visual_edges')
        .select('id')
        .eq('source', updateData.source)
        .eq('target', updateData.target)
        .neq('id', id)
        .single();

      if (existingEdge) {
        return NextResponse.json(
          { error: 'Edge already exists between these nodes' },
          { status: 409 }
        );
      }
    }

    const { data, error } = await supabase
      .from('visual_edges')
      .update(updateData)
      .eq('id', id)
      .select()
      .single();

    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Edge not found' },
          { status: 404 }
        );
      }
      console.error('Error updating edge:', error);
      return NextResponse.json(
        { error: 'Failed to update edge' },
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
        { error: 'Edge ID is required' },
        { status: 400 }
      );
    }

    // First check if edge exists
    const { data: existingEdge, error: fetchError } = await supabase
      .from('visual_edges')
      .select('id')
      .eq('id', id)
      .single();

    if (fetchError) {
      if (fetchError.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Edge not found' },
          { status: 404 }
        );
      }
      console.error('Error checking edge existence:', fetchError);
      return NextResponse.json(
        { error: 'Failed to check edge existence' },
        { status: 500 }
      );
    }

    // Delete the edge
    const { error } = await supabase
      .from('visual_edges')
      .delete()
      .eq('id', id);

    if (error) {
      console.error('Error deleting edge:', error);
      return NextResponse.json(
        { error: 'Failed to delete edge' },
        { status: 500 }
      );
    }

    return NextResponse.json(
      { message: 'Edge deleted successfully' },
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
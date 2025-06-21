import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@supabase/supabase-js';
import { EdgeData } from '../../../lib/types';

// Use service role for API operations to bypass RLS
const supabase = createClient(
  process.env.NEXT_PUBLIC_SUPABASE_URL!,
  process.env.SUPABASE_SERVICE_ROLE_KEY!
);

export async function GET() {
  try {
    const { data, error } = await supabase
      .from('visual_edges')
      .select('*')
      .order('created_at', { ascending: false });

    if (error) {
      console.error('Error fetching edges:', error);
      return NextResponse.json(
        { error: 'Failed to fetch edges' },
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

export async function DELETE() {
  try {
    const { error } = await supabase
      .from('visual_edges')
      .delete()
      .neq('id', '00000000-0000-0000-0000-000000000000'); // Delete all rows

    if (error) {
      console.error('Error deleting edges:', error);
      return NextResponse.json(
        { error: 'Failed to delete edges' },
        { status: 500 }
      );
    }

    return NextResponse.json({ message: 'All edges deleted successfully' });
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
    if (!body.source || !body.target) {
      return NextResponse.json(
        { error: 'Missing required fields: source, target' },
        { status: 400 }
      );
    }

    // Verify that source and target nodes exist
    const { data: sourceNode, error: sourceError } = await supabase
      .from('visual_nodes')
      .select('id')
      .eq('id', body.source)
      .single();

    if (sourceError || !sourceNode) {
      return NextResponse.json(
        { error: 'Source node not found' },
        { status: 400 }
      );
    }

    const { data: targetNode, error: targetError } = await supabase
      .from('visual_nodes')
      .select('id')
      .eq('id', body.target)
      .single();

    if (targetError || !targetNode) {
      return NextResponse.json(
        { error: 'Target node not found' },
        { status: 400 }
      );
    }

    // Check if edge already exists
    const { data: existingEdge } = await supabase
      .from('visual_edges')
      .select('id')
      .eq('source', body.source)
      .eq('target', body.target)
      .single();

    if (existingEdge) {
      return NextResponse.json(
        { error: 'Edge already exists between these nodes' },
        { status: 409 }
      );
    }

    // Prepare edge data with defaults
    const edgeData: Partial<EdgeData> = {
      source: body.source,
      target: body.target,
      edge_type: body.edge_type || 'connection',
      strength: body.strength || 1.0,
    };

    const { data, error } = await supabase
      .from('visual_edges')
      .insert([edgeData])
      .select()
      .single();

    if (error) {
      console.error('Error creating edge:', error);
      return NextResponse.json(
        { error: 'Failed to create edge' },
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
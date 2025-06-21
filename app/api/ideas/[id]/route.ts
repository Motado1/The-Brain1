import { NextRequest, NextResponse } from 'next/server';
import { supabase } from '@/lib/supabaseClient';
import { Idea } from '@/lib/types';

// GET /api/ideas/[id] - Get specific idea
export async function GET(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const { data, error } = await supabase
      .from('ideas')
      .select('*')
      .eq('id', params.id)
      .single();
    
    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Idea not found' },
          { status: 404 }
        );
      }
      console.error('Error fetching idea:', error);
      return NextResponse.json(
        { error: 'Failed to fetch idea', details: error.message },
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

// PUT /api/ideas/[id] - Update specific idea
export async function PUT(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const body: Partial<Idea> = await request.json();
    
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
    if (body.description !== undefined) updateData.description = body.description;
    if (body.status) updateData.status = body.status;
    if (body.priority) updateData.priority = getPriorityValue(body.priority);
    
    const { data, error } = await supabase
      .from('ideas')
      .update(updateData)
      .eq('id', params.id)
      .select()
      .single();
    
    if (error) {
      if (error.code === 'PGRST116') {
        return NextResponse.json(
          { error: 'Idea not found' },
          { status: 404 }
        );
      }
      console.error('Error updating idea:', error);
      return NextResponse.json(
        { error: 'Failed to update idea', details: error.message },
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

// DELETE /api/ideas/[id] - Delete specific idea
export async function DELETE(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  try {
    const { error } = await supabase
      .from('ideas')
      .delete()
      .eq('id', params.id);
    
    if (error) {
      console.error('Error deleting idea:', error);
      return NextResponse.json(
        { error: 'Failed to delete idea', details: error.message },
        { status: 500 }
      );
    }
    
    return NextResponse.json({ message: 'Idea deleted successfully' });
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}

// Helper function to convert priority string to number
function getPriorityValue(priority: string): number {
  switch (priority) {
    case 'low': return 1;
    case 'medium': return 2;
    case 'high': return 3;
    default: return 0;
  }
}
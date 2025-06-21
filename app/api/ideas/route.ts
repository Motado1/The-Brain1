import { NextRequest, NextResponse } from 'next/server';
import { supabase } from '@/lib/supabaseClient';
import { Idea } from '@/lib/types';

// GET /api/ideas - List all ideas
export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const page = parseInt(searchParams.get('page') || '1');
    const limit = parseInt(searchParams.get('limit') || '50');
    const status = searchParams.get('status');
    
    let query = supabase
      .from('ideas')
      .select('*', { count: 'exact' })
      .order('created_at', { ascending: false })
      .range((page - 1) * limit, page * limit - 1);
    
    if (status) {
      query = query.eq('status', status);
    }
    
    const { data, error, count } = await query;
    
    if (error) {
      console.error('Error fetching ideas:', error);
      return NextResponse.json(
        { error: 'Failed to fetch ideas', details: error.message },
        { status: 500 }
      );
    }
    
    return NextResponse.json({
      data,
      count: count || 0,
      page,
      limit
    });
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}

// POST /api/ideas - Create new idea
export async function POST(request: NextRequest) {
  try {
    const body: Partial<Idea> = await request.json();
    
    // Validate required fields
    if (!body.name || !body.name.trim()) {
      return NextResponse.json(
        { error: 'Name is required' },
        { status: 400 }
      );
    }
    
    const ideaData = {
      name: body.name.trim(),
      description: body.description || null,
      status: body.status || 'spark',
      priority: body.priority ? getPriorityValue(body.priority) : 0,
    };
    
    const { data, error } = await supabase
      .from('ideas')
      .insert([ideaData])
      .select()
      .single();
    
    if (error) {
      console.error('Error creating idea:', error);
      return NextResponse.json(
        { error: 'Failed to create idea', details: error.message },
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

// Helper function to convert priority string to number
function getPriorityValue(priority: string): number {
  switch (priority) {
    case 'low': return 1;
    case 'medium': return 2;
    case 'high': return 3;
    default: return 0;
  }
}
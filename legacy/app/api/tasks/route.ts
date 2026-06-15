import { NextRequest, NextResponse } from 'next/server';
import { supabase } from '@/lib/supabaseClient';
import { Task } from '@/lib/types';

// GET /api/tasks - List all tasks
export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const page = parseInt(searchParams.get('page') || '1');
    const limit = parseInt(searchParams.get('limit') || '50');
    const status = searchParams.get('status');
    const parent_id = searchParams.get('parent_id');
    const parent_type = searchParams.get('parent_type');
    
    let query = supabase
      .from('tasks')
      .select('*', { count: 'exact' })
      .order('created_at', { ascending: false })
      .range((page - 1) * limit, page * limit - 1);
    
    if (status) {
      query = query.eq('status', status);
    }
    if (parent_id) {
      query = query.eq('parent_id', parent_id);
    }
    if (parent_type) {
      query = query.eq('parent_type', parent_type);
    }
    
    const { data, error, count } = await query;
    
    if (error) {
      console.error('Error fetching tasks:', error);
      return NextResponse.json(
        { error: 'Failed to fetch tasks', details: error.message },
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

// POST /api/tasks - Create new task
export async function POST(request: NextRequest) {
  try {
    const body: Partial<Task> = await request.json();
    
    // Validate required fields
    if (!body.title || !body.title.trim()) {
      return NextResponse.json(
        { error: 'Title is required' },
        { status: 400 }
      );
    }
    
    if (!body.parent_id || !body.parent_type) {
      return NextResponse.json(
        { error: 'Parent ID and parent type are required' },
        { status: 400 }
      );
    }
    
    if (!['idea', 'project'].includes(body.parent_type)) {
      return NextResponse.json(
        { error: 'Parent type must be either "idea" or "project"' },
        { status: 400 }
      );
    }
    
    const taskData = {
      title: body.title.trim(),
      description: body.description || null,
      parent_type: body.parent_type,
      parent_id: body.parent_id,
      status: body.status || 'pending',
      priority: body.priority ? getPriorityValue(body.priority) : 0,
    };
    
    const { data, error } = await supabase
      .from('tasks')
      .insert([taskData])
      .select()
      .single();
    
    if (error) {
      console.error('Error creating task:', error);
      return NextResponse.json(
        { error: 'Failed to create task', details: error.message },
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
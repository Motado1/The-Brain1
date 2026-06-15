import { NextRequest, NextResponse } from 'next/server';
import { supabase } from '@/lib/supabaseClient';
import { Artifact, JobQueue } from '@/lib/types';

// Helper function to trigger immediate RAG processing
async function triggerImmediateProcessing(jobId: string) {
  const SUPABASE_FUNCTION_URL = process.env.NEXT_PUBLIC_SUPABASE_URL?.replace('/rest/v1', '') || 'http://127.0.0.1:54321';
  const SUPABASE_ANON_KEY = process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY;
  
  if (!SUPABASE_ANON_KEY) {
    throw new Error('Supabase anon key not found');
  }
  
  const functionUrl = `${SUPABASE_FUNCTION_URL}/functions/v1/ingestion-worker`;
  
  console.log('📞 Calling Edge Function at:', functionUrl);
  
  const response = await fetch(functionUrl, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${SUPABASE_ANON_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      trigger: 'immediate',
      jobId: jobId
    })
  });
  
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Edge Function call failed: ${response.status} ${errorText}`);
  }
  
  const result = await response.json();
  console.log('✅ Edge Function triggered successfully:', result);
  return result;
}

// GET /api/artifacts - List all artifacts
export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const page = parseInt(searchParams.get('page') || '1');
    const limit = parseInt(searchParams.get('limit') || '50');
    const status = searchParams.get('status');
    const type = searchParams.get('type');
    
    let query = supabase
      .from('artifacts')
      .select('*', { count: 'exact' })
      .order('created_at', { ascending: false })
      .range((page - 1) * limit, page * limit - 1);
    
    if (status) {
      query = query.eq('status', status);
    }
    
    if (type) {
      query = query.eq('type', type);
    }
    
    const { data, error, count } = await query;
    
    if (error) {
      console.error('Error fetching artifacts:', error);
      return NextResponse.json(
        { error: 'Failed to fetch artifacts', details: error.message },
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

// POST /api/artifacts - Create new artifact and trigger RAG processing
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    
    // Validate required fields
    if (!body.name || !body.name.trim()) {
      return NextResponse.json(
        { error: 'Name is required' },
        { status: 400 }
      );
    }
    
    if (!body.type) {
      return NextResponse.json(
        { error: 'Type is required' },
        { status: 400 }
      );
    }
    
    // For file uploads, require storage path
    if (body.type === 'file' && !body.storagePath) {
      return NextResponse.json(
        { error: 'Storage path is required for file uploads' },
        { status: 400 }
      );
    }
    
    // Prepare artifact data
    const artifactData: Partial<Artifact> = {
      name: body.name.trim(),
      description: body.description || null,
      type: body.type,
      url: body.url || null,
      content: body.content || null,
      status: 'processing', // Set to processing for RAG pipeline
    };
    
    // Use service role for database operations (to bypass RLS)
    const { createClient } = require('@supabase/supabase-js');
    const supabaseService = createClient(
      process.env.NEXT_PUBLIC_SUPABASE_URL!,
      process.env.SUPABASE_SERVICE_ROLE_KEY!
    );
    
    // Insert artifact metadata using service role
    const { data: artifact, error: artifactError } = await supabaseService
      .from('artifacts')
      .insert([artifactData])
      .select()
      .single();
    
    if (artifactError) {
      console.error('Error creating artifact:', artifactError);
      return NextResponse.json(
        { error: 'Failed to create artifact', details: artifactError.message },
        { status: 500 }
      );
    }
    
    // Insert job into job_queue for background processing
    const jobPayload = {
      artifactId: artifact.id,
      ...(body.storagePath && { storagePath: body.storagePath }),
      ...(body.url && { url: body.url }),
      ...(body.content && { content: body.content }),
      type: body.type,
      name: body.name
    };
    
    const { data: job, error: jobError } = await supabaseService
      .from('job_queue')
      .insert([{
        job_type: 'ingest_artifact',
        payload: jobPayload,
        status: 'pending',
        priority: 1, // Normal priority
        next_run_at: new Date().toISOString()
      }])
      .select()
      .single();
    
    if (jobError) {
      console.error('Error creating job:', jobError);
      // Don't fail the request, but log the error
      console.warn('Artifact created but job queue insertion failed:', jobError.message);
    }
    
    // Immediately trigger the Edge Function worker to process the job
    if (job) {
      try {
        console.log('🚀 Triggering immediate RAG processing for job:', job.id);
        await triggerImmediateProcessing(job.id);
      } catch (triggerError) {
        console.error('Failed to trigger immediate processing:', triggerError);
        // Don't fail the request, processing will happen via cron as fallback
      }
    }
    
    // Return 202 Accepted with artifact and job info
    return NextResponse.json({
      message: 'Artifact queued for processing',
      artifact,
      job: job || null,
      status: 'processing'
    }, { status: 202 });
    
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}
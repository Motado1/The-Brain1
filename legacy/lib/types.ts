import * as THREE from 'three';

// Core Entity Types
export interface Idea {
  id: string;
  name: string;
  description?: string;
  status: 'spark' | 'validation' | 'approved' | 'active' | 'completed' | 'archived';
  priority?: 'low' | 'medium' | 'high';
  created_at?: string;
  updated_at?: string;
}

export interface Project {
  id: string;
  name: string;
  description?: string;
  status: 'planning' | 'active' | 'completed' | 'on_hold';
  priority?: 'low' | 'medium' | 'high';
  idea_id?: string; // Reference to parent idea
  created_at?: string;
  updated_at?: string;
}

export interface Task {
  id: string;
  title: string;
  description?: string;
  status: 'pending' | 'in_progress' | 'completed' | 'blocked';
  priority?: 'low' | 'medium' | 'high';
  parent_type: 'idea' | 'project';
  parent_id: string; // References idea or project
  created_at?: string;
  updated_at?: string;
}

export interface Artifact {
  id: string;
  name: string;
  description?: string;
  type: 'document' | 'link' | 'file' | 'note';
  url?: string;
  content?: string;
  status?: 'processing' | 'indexed' | 'failed';
  embedding?: number[]; // Vector embedding
  metadata?: Record<string, any>; // JSONB metadata
  content_hash?: string; // SHA-256 hash for deduplication
  chunk_index?: number; // For document chunking
  parent_artifact_id?: string; // For chunks
  indexed_at?: string; // When embedding was created
  created_at?: string;
  updated_at?: string;
}

export interface JobQueue {
  id: string;
  job_type: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'retrying';
  priority: number;
  payload: Record<string, any>;
  result?: Record<string, any>;
  error_message?: string;
  retry_count: number;
  max_retries: number;
  next_run_at: string;
  started_at?: string;
  completed_at?: string;
  created_at: string;
  updated_at: string;
}

export interface KnowledgeItem {
  id: string;
  name: string;
  type: 'file' | 'note' | 'link';
  status: 'processing' | 'indexed' | 'failed';
  fileName?: string;
  fileSize?: number;
  uploadedAt: string;
  processingStatus?: 'queued' | 'processing' | 'completed' | 'failed';
  error?: string;
}

// Visual Representation Types
export interface NodeData {
  id: string;
  name: string;
  entity_type: string;
  entity_id: string;
  x?: number;
  y?: number;
  z?: number;
  fx?: number;
  fy?: number;
  fz?: number;
  pos?: THREE.Vector3; // Single mutable Vector3 for position
  scale?: number;
  color?: string;
  parent_id?: string;
  layer: number;
  type?: string;
  pillar?: string;
  url?: string;
  activity_level?: string;
  visual_state?: any;
  is_pillar?: boolean;
  size?: number;
  created_at?: string;
  updated_at?: string;
}

export interface EdgeData {
  id: string;
  source: string;
  target: string;
  edge_type?: string;
  label?: string;
  strength?: number;
  created_at?: string;
}

// API Response Types
export interface ApiResponse<T> {
  data?: T;
  error?: string;
  message?: string;
}

export interface PaginatedResponse<T> {
  data: T[];
  count: number;
  page: number;
  limit: number;
}
import { Idea, Task, NodeData, EdgeData, ApiResponse, PaginatedResponse } from './types';

const API_BASE = '/api';

// Generic API request function
async function apiRequest<T>(
  endpoint: string, 
  options: RequestInit = {}
): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${API_BASE}${endpoint}`, {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    });

    const data = await response.json();

    if (!response.ok) {
      return {
        error: data.error || `HTTP ${response.status}`,
        message: data.message,
      };
    }

    return { data: data.data || data };
  } catch (error) {
    return {
      error: error instanceof Error ? error.message : 'Network error',
    };
  }
}

// Ideas API
export const ideasApi = {
  // Get all ideas with optional filtering
  getAll: (params?: { 
    page?: number; 
    limit?: number; 
    status?: string; 
  }): Promise<ApiResponse<PaginatedResponse<Idea>>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.limit) searchParams.set('limit', params.limit.toString());
    if (params?.status) searchParams.set('status', params.status);
    
    const query = searchParams.toString();
    return apiRequest(`/ideas${query ? `?${query}` : ''}`);
  },

  // Get specific idea by ID
  getById: (id: string): Promise<ApiResponse<Idea>> => {
    return apiRequest(`/ideas/${id}`);
  },

  // Create new idea
  create: (idea: Omit<Idea, 'id' | 'created_at' | 'updated_at'>): Promise<ApiResponse<Idea>> => {
    return apiRequest('/ideas', {
      method: 'POST',
      body: JSON.stringify(idea),
    });
  },

  // Update existing idea
  update: (id: string, updates: Partial<Idea>): Promise<ApiResponse<Idea>> => {
    return apiRequest(`/ideas/${id}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  },

  // Delete idea
  delete: (id: string): Promise<ApiResponse<{ message: string }>> => {
    return apiRequest(`/ideas/${id}`, {
      method: 'DELETE',
    });
  },
};

// Tasks API
export const tasksApi = {
  // Get all tasks with optional filtering
  getAll: (params?: { 
    page?: number; 
    limit?: number; 
    status?: string;
    parent_id?: string;
    parent_type?: string;
  }): Promise<ApiResponse<PaginatedResponse<Task>>> => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', params.page.toString());
    if (params?.limit) searchParams.set('limit', params.limit.toString());
    if (params?.status) searchParams.set('status', params.status);
    if (params?.parent_id) searchParams.set('parent_id', params.parent_id);
    if (params?.parent_type) searchParams.set('parent_type', params.parent_type);
    
    const query = searchParams.toString();
    return apiRequest(`/tasks${query ? `?${query}` : ''}`);
  },

  // Get specific task by ID
  getById: (id: string): Promise<ApiResponse<Task>> => {
    return apiRequest(`/tasks/${id}`);
  },

  // Create new task
  create: (task: Omit<Task, 'id' | 'created_at' | 'updated_at'>): Promise<ApiResponse<Task>> => {
    return apiRequest('/tasks', {
      method: 'POST',
      body: JSON.stringify(task),
    });
  },

  // Update existing task
  update: (id: string, updates: Partial<Task>): Promise<ApiResponse<Task>> => {
    return apiRequest(`/tasks/${id}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  },

  // Delete task
  delete: (id: string): Promise<ApiResponse<{ message: string }>> => {
    return apiRequest(`/tasks/${id}`, {
      method: 'DELETE',
    });
  },
};

// Visual Nodes API
export const visualNodesApi = {
  // Get all visual nodes with optional filtering
  getAll: (params?: { 
    entity_type?: string;
    layer?: number;
  }): Promise<ApiResponse<NodeData[]>> => {
    const searchParams = new URLSearchParams();
    if (params?.entity_type) searchParams.set('entity_type', params.entity_type);
    if (params?.layer !== undefined) searchParams.set('layer', params.layer.toString());
    
    const query = searchParams.toString();
    return apiRequest(`/visual-nodes${query ? `?${query}` : ''}`);
  },

  // Get specific visual node by ID
  getById: (id: string): Promise<ApiResponse<NodeData>> => {
    return apiRequest(`/visual-nodes/${id}`);
  },

  // Create new visual node
  create: (node: Omit<NodeData, 'id' | 'created_at' | 'updated_at'>): Promise<ApiResponse<NodeData>> => {
    return apiRequest('/visual-nodes', {
      method: 'POST',
      body: JSON.stringify(node),
    });
  },

  // Update existing visual node
  update: (id: string, updates: Partial<NodeData>): Promise<ApiResponse<NodeData>> => {
    return apiRequest(`/visual-nodes/${id}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  },

  // Delete visual node
  delete: (id: string): Promise<ApiResponse<{ message: string }>> => {
    return apiRequest(`/visual-nodes/${id}`, {
      method: 'DELETE',
    });
  },
};

// Helper function to create visual node when creating core entities
export const createEntityWithVisualization = {
  // Create idea with corresponding visual node
  idea: async (ideaData: Omit<Idea, 'id' | 'created_at' | 'updated_at'>, visualData?: Partial<NodeData>) => {
    const ideaResult = await ideasApi.create(ideaData);
    
    if (ideaResult.error || !ideaResult.data) {
      return ideaResult;
    }

    // Create corresponding visual node
    const visualNodeData: Omit<NodeData, 'id' | 'created_at' | 'updated_at'> = {
      name: ideaData.name,
      entity_type: 'idea',
      entity_id: ideaResult.data.id,
      layer: 1,
      scale: 5, // 0.5x pillar size as per our visualization
      color: '#EC4899', // Pink for ideas
      ...visualData,
    };

    const visualResult = await visualNodesApi.create(visualNodeData);
    
    return {
      data: {
        idea: ideaResult.data,
        visualNode: visualResult.data,
      },
      error: visualResult.error,
    };
  },

  // Create task with corresponding visual node
  task: async (taskData: Omit<Task, 'id' | 'created_at' | 'updated_at'>, visualData?: Partial<NodeData>) => {
    const taskResult = await tasksApi.create(taskData);
    
    if (taskResult.error || !taskResult.data) {
      return taskResult;
    }

    // Create corresponding visual node
    const visualNodeData: Omit<NodeData, 'id' | 'created_at' | 'updated_at'> = {
      name: taskData.title,
      entity_type: 'task',
      entity_id: taskResult.data.id,
      layer: 2,
      scale: 2, // 0.2x pillar size as per our visualization
      color: '#F59E0B', // Orange for tasks
      parent_id: taskData.parent_id, // Connect to parent idea/project
      ...visualData,
    };

    const visualResult = await visualNodesApi.create(visualNodeData);
    
    return {
      data: {
        task: taskResult.data,
        visualNode: visualResult.data,
      },
      error: visualResult.error,
    };
  },
};
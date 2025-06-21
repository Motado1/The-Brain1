import { create } from 'zustand';

export interface KnowledgeItem {
  id: string;
  title: string;
  content: string;
  type: 'document' | 'note' | 'reference';
  tags: string[];
  created_at: string;
  updated_at: string;
  source_url?: string;
  related_nodes?: string[];
}

interface KnowledgeState {
  items: KnowledgeItem[];
  selectedItem: KnowledgeItem | null;
  searchQuery: string;
  isLoading: boolean;
  
  // Actions
  setItems: (items: KnowledgeItem[]) => void;
  addItem: (item: KnowledgeItem) => void;
  updateItem: (id: string, updates: Partial<KnowledgeItem>) => void;
  deleteItem: (id: string) => void;
  setSelectedItem: (item: KnowledgeItem | null) => void;
  setSearchQuery: (query: string) => void;
  setLoading: (loading: boolean) => void;
  
  // Search and filter
  searchItems: (query: string) => KnowledgeItem[];
  getItemsByType: (type: KnowledgeItem['type']) => KnowledgeItem[];
  getItemsByTag: (tag: string) => KnowledgeItem[];
}

export const useKnowledgeStore = create<KnowledgeState>((set, get) => ({
  items: [],
  selectedItem: null,
  searchQuery: '',
  isLoading: false,
  
  setItems: (items) => set({ items }),
  
  addItem: (item) => set(state => ({ 
    items: [...state.items, item] 
  })),
  
  updateItem: (id, updates) => set(state => ({
    items: state.items.map(item => 
      item.id === id ? { ...item, ...updates } : item
    )
  })),
  
  deleteItem: (id) => set(state => ({
    items: state.items.filter(item => item.id !== id),
    selectedItem: state.selectedItem?.id === id ? null : state.selectedItem
  })),
  
  setSelectedItem: (item) => set({ selectedItem: item }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setLoading: (loading) => set({ isLoading: loading }),
  
  searchItems: (query) => {
    const { items } = get();
    if (!query.trim()) return items;
    
    const lowerQuery = query.toLowerCase();
    return items.filter(item =>
      item.title.toLowerCase().includes(lowerQuery) ||
      item.content.toLowerCase().includes(lowerQuery) ||
      item.tags.some(tag => tag.toLowerCase().includes(lowerQuery))
    );
  },
  
  getItemsByType: (type) => {
    const { items } = get();
    return items.filter(item => item.type === type);
  },
  
  getItemsByTag: (tag) => {
    const { items } = get();
    return items.filter(item => item.tags.includes(tag));
  }
}));
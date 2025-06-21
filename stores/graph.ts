import { create } from 'zustand';

// Throttle helper
function throttle<T extends (...args: any[]) => any>(func: T, delay: number): T {
  let timeoutId: NodeJS.Timeout | null = null;
  let lastExecTime = 0;
  
  return ((...args: any[]) => {
    const currentTime = Date.now();
    
    if (currentTime - lastExecTime > delay) {
      func(...args);
      lastExecTime = currentTime;
    } else {
      if (timeoutId) clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        func(...args);
        lastExecTime = Date.now();
      }, delay - (currentTime - lastExecTime));
    }
  }) as T;
}

interface GraphState {
  selectedNodeId: string | null;
  mode: 'idle' | 'focus';
}

interface GraphActions {
  selectNode: (id: string | null) => void;
  setMode: (m: 'idle' | 'focus') => void;
}

interface GraphStore extends GraphState, GraphActions {}

export const useGraphStore = create<GraphStore>((set) => {
  // Create throttled selectNode function
  const throttledSelectNode = throttle((id: string | null) => {
    set({ selectedNodeId: id });
  }, 150); // 150ms throttle to prevent rapid double-clicks

  return {
    // State
    selectedNodeId: null,
    mode: 'idle',

    // Actions
    selectNode: throttledSelectNode,

    setMode: (m: 'idle' | 'focus') => {
      set({ mode: m });
    },
  };
});

// Convenience hooks for component consumption
export const useSelectedNodeId = () => useGraphStore((state) => state.selectedNodeId);
export const useMode = () => useGraphStore((state) => state.mode);
export const useGraphActions = () => useGraphStore((state) => ({
  selectNode: state.selectNode,
  setMode: state.setMode,
}));
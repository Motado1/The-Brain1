import { EdgeData } from '../lib/types';

export class AdjacencyMap {
  private adjacencyMap: Map<string, Set<string>> = new Map();

  constructor(edges: EdgeData[]) {
    this.buildMap(edges);
  }

  private buildMap(edges: EdgeData[]) {
    this.adjacencyMap.clear();
    
    for (const edge of edges) {
      // Add bidirectional connections
      if (!this.adjacencyMap.has(edge.source)) {
        this.adjacencyMap.set(edge.source, new Set());
      }
      if (!this.adjacencyMap.has(edge.target)) {
        this.adjacencyMap.set(edge.target, new Set());
      }
      
      this.adjacencyMap.get(edge.source)!.add(edge.target);
      this.adjacencyMap.get(edge.target)!.add(edge.source);
    }
  }

  isConnectedTo(nodeId: string, targetId: string): boolean {
    const connections = this.adjacencyMap.get(nodeId);
    return connections ? connections.has(targetId) : false;
  }

  update(edges: EdgeData[]) {
    this.buildMap(edges);
  }
}

// Global adjacency map instance
let globalAdjacencyMap: AdjacencyMap | null = null;

export function setGlobalAdjacencyMap(edges: EdgeData[]) {
  globalAdjacencyMap = new AdjacencyMap(edges);
}

export function isConnectedTo(nodeId: string, targetId: string): boolean {
  return globalAdjacencyMap ? globalAdjacencyMap.isConnectedTo(nodeId, targetId) : false;
}
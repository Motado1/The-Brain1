import { NodeData, EdgeData } from './types';

export const mockNodes: NodeData[] = [
  {
    id: '1',
    entity_type: 'pillar',
    entity_id: '1',
    name: 'Knowledge Core',
    layer: 0,
    x: 0, y: 0, z: 0,
    scale: 2.0,
    color: '#6B46C1'
  },
  {
    id: '2',
    entity_type: 'pillar',
    entity_id: '2',
    name: 'Idea & Project Hub',
    layer: 0,
    x: 100, y: 0, z: 0,
    scale: 2.0,
    color: '#EC4899'
  },
  {
    id: '3',
    entity_type: 'pillar',
    entity_id: '3',
    name: 'Action & Task Dashboard',
    layer: 0,
    x: 50, y: 86, z: 0,
    scale: 2.0,
    color: '#F59E0B'
  },
  {
    id: '4',
    entity_type: 'pillar',
    entity_id: '4',
    name: 'Learning Ledger',
    layer: 0,
    x: -50, y: 86, z: 0,
    scale: 2.0,
    color: '#10B981'
  },
  {
    id: '5',
    entity_type: 'pillar',
    entity_id: '5',
    name: 'AI Assistant Layer',
    layer: 0,
    x: -100, y: 0, z: 0,
    scale: 2.0,
    color: '#3B82F6'
  },
  {
    id: '6',
    entity_type: 'pillar',
    entity_id: '6',
    name: 'Workbench',
    layer: 0,
    x: -50, y: -86, z: 0,
    scale: 2.0,
    color: '#8B5CF6'
  }
];

export const mockEdges: EdgeData[] = [
  {
    id: 'e1',
    source: '1',
    target: '2',
    edge_type: 'reference',
    strength: 1.0
  },
  {
    id: 'e2',
    source: '2',
    target: '3',
    edge_type: 'reference',
    strength: 1.0
  }
];
"use client";
import { Line } from '@react-three/drei';
import { NodeData } from '@/lib/types';

interface PillarConnectionProps {
  pillar: NodeData;
  childNode: NodeData;
}

export default function PillarConnection({ pillar, childNode }: PillarConnectionProps) {
  // Use the actual rendered pillar coordinates (already spread out in CanvasScene)
  const start = [pillar.x || 0, pillar.y || 0, pillar.z || 0] as [number, number, number];
  const end = [childNode.x || 0, childNode.y || 0, childNode.z || 0] as [number, number, number];

  return (
    <Line 
      points={[start, end]}
      color="#444"
      lineWidth={0.5}
      transparent 
      opacity={0.3}
      dashed={true}
      dashSize={2}
      gapSize={1}
    />
  );
}
import { NeuroLink } from './NeuroLink'
import * as THREE from 'three'

interface DendritesProps {
  pillar: any // The pillar node
  childNodes: any[] // Nodes that belong to this pillar
  color?: string
}

export function Dendrites({ 
  pillar, 
  childNodes,
  color = '#00ccff' 
}: DendritesProps) {
  const pillarPos = new THREE.Vector3(pillar.x, pillar.y, pillar.z)
  
  // Create connections to all child nodes
  return (
    <group>
      {childNodes.map((childNode) => (
        <NeuroLink
          key={`${pillar.id}-${childNode.id}`}
          start={pillarPos}
          end={new THREE.Vector3(childNode.x, childNode.y, childNode.z)}
          color={color}
          amplitude={0.3}      // Organic curve
          segments={5}         // Smooth curve
          radius={0.008}       // Visible thickness
          pulseSpeed={0.6}     // Normal pulse speed
          pulseCount={2}       // A couple pulses
          showParticles={true}
          particleCount={10}
        />
      ))}
    </group>
  )
}
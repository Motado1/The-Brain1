'use client';

import { Canvas } from '@react-three/fiber';
import { OrbitControls } from '@react-three/drei';
import { Dendrites } from '@/components/Dendrites';
import VisualNode from '@/components/VisualNode';
import { VisualNodeData } from '@/components/VisualNode';

export default function TestDendrites() {
  // Test pillar node
  const testPillarNode: VisualNodeData = {
    id: 'test-pillar',
    name: 'Test Pillar',
    entity_type: 'pillar',
    is_pillar: true,
    size: 3.0,
    x: 0,
    y: 0,
    z: 0
  };

  return (
    <div style={{ width: '100vw', height: '100vh' }}>
      <Canvas camera={{ position: [50, 50, 50], fov: 75 }}>
        <color attach="background" args={['#000000']} />
        <ambientLight intensity={0.5} />
        <pointLight position={[10, 10, 10]} />
        
        <OrbitControls />
        
        {/* Test just the Dendrites component */}
        <group position={[-20, 0, 0]}>
          <mesh>
            <sphereGeometry args={[1, 32, 32]} />
            <meshStandardMaterial color="#00ff00" />
          </mesh>
          <Dendrites color="#ff6600" />
        </group>
        
        {/* Test VisualNode with pillar */}
        <VisualNode 
          node={testPillarNode}
          onClick={() => console.log('Clicked test pillar')}
        />
        
      </Canvas>
    </div>
  );
}
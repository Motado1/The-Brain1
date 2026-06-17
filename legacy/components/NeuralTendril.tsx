import { useMemo, useRef } from 'react'
import { useFrame, extend } from '@react-three/fiber'
import * as THREE from 'three'
import { shaderMaterial } from '@react-three/drei'

// Neural pulse shader following the exact specification
const NeuralPulseMaterial = shaderMaterial(
  {
    time: 0,
    headSize: 0.04,    // 4% of bar length
    tailSize: 0.25,    // 25% of bar length
    pulseSpeed: 0.7,   // Speed multiplier
  },
  // Vertex shader
  `varying vec2 vUv;
   void main() {
     vUv = uv;
     gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
   }`,
  // Fragment shader - exact specification
  `uniform float time;
   uniform float headSize;
   uniform float tailSize;
   uniform float pulseSpeed;
   varying vec2 vUv;
   
   void main() {
     // Head position travels 0-1 along tube length (vUv.x)
     float headPosition = fract(time * pulseSpeed);
     
     // Color ramp along radius (vUv.y from center to edge)
     float radiusFromCenter = abs(vUv.y - 0.5) * 2.0; // 0 at center, 1 at edge
     vec3 centerColor = vec3(1.0, 0.8, 0.4);    // #FFCC66 (very bright)
     vec3 midColor = vec3(1.0, 0.6, 0.0);       // #FF9900
     vec3 outerColor = vec3(0.2, 0.1, 0.0);     // #331900 (almost black)
     
     vec3 color;
     if (radiusFromCenter < 0.5) {
       // Center to mid
       color = mix(centerColor, midColor, radiusFromCenter * 2.0);
     } else {
       // Mid to outer
       color = mix(midColor, outerColor, (radiusFromCenter - 0.5) * 2.0);
     }
     
     // Pulse logic - only show behind and at head position
     float alpha = 0.0;
     float v = vUv.x; // Position along tube length
     
     if (v <= headPosition) {
       float distFromHead = headPosition - v;
       
       if (distFromHead <= headSize) {
         // Sharp gaussian falloff for head core (first 4%) - clamped HDR value
         float headFactor = distFromHead / headSize;
         alpha = exp(-headFactor * headFactor * 25.0) * 1.4; // Head core intensity ≈ 1.4 (was 2.5)
       } else if (distFromHead <= tailSize) {
         // Exponential fade for tail (next 25%)
         float tailFactor = (distFromHead - headSize) / (tailSize - headSize);
         alpha = exp(-tailFactor * 8.0) * 0.6; // Tail intensity ≈ 0.6
       }
     }
     
     // NEW seam-less rim (replaces old radial fade to hide UV seam)
     float rim = sin(vUv.x * 3.14159265);        // 0 at seam, 1 at center
     rim *= rim;                                 // sharper core
     
     // Smooth edge fade for tube depth
     float edgeFade = 1.0 - pow(radiusFromCenter, 2.0);
     alpha *= edgeFade * rim;
     
     gl_FragColor = vec4(color, alpha);
   }`
)

extend({ NeuralPulseMaterial })

// Create organic curve with multiple control points
function createTendrilCurve(start: THREE.Vector3, end: THREE.Vector3) {
  const distance = start.distanceTo(end)
  const controlPoints = []
  const numControls = Math.floor(distance / 3) + 3
  
  controlPoints.push(start)
  
  for (let i = 1; i < numControls - 1; i++) {
    const t = i / (numControls - 1)
    const point = start.clone().lerp(end, t)
    
    // Add random offset perpendicular to the line
    const offset = new THREE.Vector3(
      (Math.random() - 0.5) * distance * 0.2,
      (Math.random() - 0.5) * distance * 0.2,
      (Math.random() - 0.5) * distance * 0.2
    )
    
    // Reduce offset at ends for smooth connection
    const falloff = Math.sin(t * Math.PI)
    point.add(offset.multiplyScalar(falloff))
    
    controlPoints.push(point)
  }
  
  controlPoints.push(end)
  
  return new THREE.CatmullRomCurve3(controlPoints, false, 'catmullrom', 0.5)
}

interface NeuralTendrilProps {
  start: THREE.Vector3
  end: THREE.Vector3
  color?: string
  opacity?: number
  thickness?: number
  pulseSpeed?: number
  pulseDelay?: number
}

export function NeuralTendril({
  start,
  end,
  color = '#ffcc66',
  opacity = 1.0,
  thickness = 0.008,  // ~0.5% of screen height equivalent
  pulseSpeed = 0.7,
  pulseDelay = 0
}: NeuralTendrilProps) {
  const materialRef = useRef<any>(null)
  
  // Create single organic curve
  const curve = useMemo(() => createTendrilCurve(start, end), [start, end])
  
  // Create tube geometry with more radial segments for smoother appearance
  const geometry = useMemo(
    () => new THREE.TubeGeometry(curve, 64, thickness, 16, false),
    [curve, thickness]
  )
  
  // Animate pulse
  useFrame(({ clock }) => {
    if (materialRef.current) {
      materialRef.current.time = clock.elapsedTime + pulseDelay
    }
  })
  
  return (
    <mesh geometry={geometry}>
      {/* @ts-ignore */}
      <neuralPulseMaterial
        ref={materialRef}
        transparent
        depthWrite={false}
        blending={THREE.AdditiveBlending}
        side={THREE.DoubleSide}
        headSize={0.04}
        tailSize={0.25}
        pulseSpeed={pulseSpeed}
      />
    </mesh>
  )
}

// Keep the cluster for backward compatibility, but use single tendrils
export function TendrilCluster({
  start,
  end,
  count = 1,
  spread = 0,
  color = '#ffcc66'
}: any) {
  return (
    <NeuralTendril
      start={start}
      end={end}
      color={color}
    />
  )
}
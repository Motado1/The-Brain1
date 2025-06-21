import { useMemo, useRef } from 'react'
import { useFrame, extend } from '@react-three/fiber'
import * as THREE from 'three'
import { shaderMaterial } from '@react-three/drei'
import '@/materials/NeuroPathMaterial'

// Build organic curve function
function buildOrganicCurve(start: THREE.Vector3, end: THREE.Vector3, segments = 5, amplitude = 0.3) {
  const points = [start]
  
  for (let i = 1; i < segments; i++) {
    const t = i / segments
    const point = start.clone().lerp(end, t)
    
    // Add organic jitter
    if (i > 0 && i < segments - 1) {
      const jitter = amplitude * (1 - Math.abs(t - 0.5) * 2) // Less jitter at ends
      point.x += (Math.random() - 0.5) * jitter
      point.y += (Math.random() - 0.5) * jitter
      point.z += (Math.random() - 0.5) * jitter
    }
    
    points.push(point)
  }
  
  points.push(end)
  return new THREE.CatmullRomCurve3(points)
}

// Enhanced pulse shader with multiple waves
const NeuroPulseMaterial = shaderMaterial(
  {
    time: 0,
    pulseSpeed: 0.6,
    pulseCount: 3,
    coreColor: new THREE.Color('#ff6600'),
    edgeColor: new THREE.Color('#ff3300'),
    glowIntensity: 3.0,
  },
  // Vertex
  `varying vec2 vUv;
   varying vec3 vPosition;
   void main() {
     vUv = uv;
     vPosition = position;
     gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
   }`,
  // Fragment with multiple pulses
  `uniform float time;
   uniform float pulseSpeed;
   uniform float pulseCount;
   uniform vec3 coreColor;
   uniform vec3 edgeColor;
   uniform float glowIntensity;
   varying vec2 vUv;
   
   void main() {
     float intensity = 0.0;
     
     // Create multiple pulses
     for(float i = 0.0; i < 3.0; i++) {
       if(i >= pulseCount) break;
       
       float offset = i / pulseCount;
       float pulsePos = mod(vUv.x + time * pulseSpeed + offset, 1.0);
       
       // Sharp leading edge with long tail
       float dist = vUv.x - pulsePos;
       if(dist < 0.0) dist += 1.0;
       
       // Exponential decay for natural look
       intensity += exp(-dist * 15.0) * 2.0; // Sharp head
       intensity += exp(-dist * 3.0) * 0.5;   // Long tail
     }
     
     // Radial fade for tube depth
     float radialFade = 1.0 - pow(abs(vUv.y - 0.5) * 2.0, 2.0);
     intensity *= radialFade;
     
     // Mix edge and core colors
     vec3 color = mix(edgeColor, coreColor, radialFade);
     
     // White-hot cores
     if(intensity > 2.0) {
       color = mix(color, vec3(1.0), (intensity - 2.0) * 0.3);
     }
     
     // Base glow
     intensity = (intensity + 0.15) * glowIntensity;
     
     gl_FragColor = vec4(color * intensity, min(intensity * 0.8, 1.0));
   }`
)

extend({ NeuroPulseMaterial })

// Particle system for extra life
const LinkParticles = ({ curve, count = 20, color }: any) => {
  const particles = useRef<THREE.Points>(null!)
  const positions = useMemo(() => new Float32Array(count * 3), [count])
  const delays = useMemo(() => new Float32Array(count).map(() => Math.random()), [count])
  
  useFrame((state) => {
    if (!particles.current) return
    
    const posAttr = particles.current.geometry.attributes.position
    
    for (let i = 0; i < count; i++) {
      const t = (state.clock.elapsedTime * 0.2 + delays[i]) % 1
      const point = curve.getPointAt(t)
      
      positions[i * 3] = point.x + (Math.random() - 0.5) * 0.05
      positions[i * 3 + 1] = point.y + (Math.random() - 0.5) * 0.05
      positions[i * 3 + 2] = point.z + (Math.random() - 0.5) * 0.05
    }
    
    posAttr.needsUpdate = true
  })
  
  return (
    <points ref={particles}>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          count={count}
          array={positions}
          itemSize={3}
        />
      </bufferGeometry>
      <pointsMaterial
        size={0.02}
        color={color}
        transparent
        opacity={0.6}
        blending={THREE.AdditiveBlending}
        depthWrite={false}
      />
    </points>
  )
}

interface NeuroLinkProps {
  start: THREE.Vector3
  end: THREE.Vector3
  color?: string
  amplitude?: number
  segments?: number
  radius?: number
  pulseSpeed?: number
  pulseCount?: number
  showParticles?: boolean
  particleCount?: number
}

export function NeuroLink({
  start,
  end,
  color = '#ff6600',
  amplitude = 0.25,
  segments = 5,
  radius = 0.008,
  pulseSpeed = 0.6,
  pulseCount = 3,
  showParticles = true,
  particleCount = 15
}: NeuroLinkProps) {
  const matRef = useRef<any>(null)
  
  const curve = useMemo(
    () => buildOrganicCurve(start, end, segments, amplitude),
    [start, end, segments, amplitude]
  )
  
  const geometry = useMemo(
    () => new THREE.TubeGeometry(curve, 64, radius, 8, false),
    [curve, radius]
  )
  
  useFrame(({ clock }) => {
    if (matRef.current) {
      matRef.current.time = clock.elapsedTime
    }
  })
  
  return (
    <group>
      {/* Main tube with pulse effect */}
      <mesh geometry={geometry}>
        {/* @ts-ignore */}
        <neuroPathMaterial
          ref={matRef}
          baseGlow={0.15}        // faint but visible
          baseColor={'#ffffff'}  // white filament
          hotColor={'#ffb040'}   // orange pulses
          pulseSpeed={0.6}
        />
      </mesh>
      
      {/* Outer glow layer */}
      <mesh geometry={geometry} scale={[1.5, 1.5, 1]}>
        <meshBasicMaterial
          color={color}
          transparent
          opacity={0.1}
          blending={THREE.AdditiveBlending}
          depthWrite={false}
        />
      </mesh>
      
      {/* Particle effects */}
      {showParticles && (
        <LinkParticles 
          curve={curve} 
          count={particleCount} 
          color={color} 
        />
      )}
    </group>
  )
}
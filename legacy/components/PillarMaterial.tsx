import { shaderMaterial } from '@react-three/drei';
import * as THREE from 'three';

export const PillarMaterial = shaderMaterial(
  {
    time: 0,
    color: new THREE.Color('#00d8ff'),
    emissiveIntensity: 8.0
  },
  // vertex shader
  `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  // fragment shader
  `
    uniform float time;
    uniform vec3 color;
    uniform float emissiveIntensity;
    varying vec2 vUv;
    
    void main() {
      gl_FragColor = vec4(color * emissiveIntensity, 1.0);
    }
  `
);

export default PillarMaterial;
import * as THREE from 'three';
import { shaderMaterial } from '@react-three/drei';
import { extend } from '@react-three/fiber';

export const MultiPulseMaterial = shaderMaterial(
  // Uniforms
  {
    time: 0,
    pulseSpeed: 0.6,
    pulseOffsets: [0.0, 0.25, 0.5, 0.75],
    headSize: 0.03,
    tailSize: 0.25,
    baseGlow: 0.15,                                    // slightly brighter filament
    baseColor: new THREE.Color(0xffffff),             // NEW - white filament
    hotColor: new THREE.Color('#ffb040')               // orange pulse
  },
  // Vertex shader
  `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  // Fragment shader
  `
    uniform float time, pulseSpeed, headSize, tailSize, baseGlow;
    uniform float[4] pulseOffsets;
    uniform vec3  hotColor;            // orange pulse
    uniform vec3  baseColor;           // NEW  ← white filament (pass in vec3(1.0))
    varying vec2  vUv;

    float radialFactor(){
      float s = sin(vUv.x * 3.14159265);
      return s*s;
    }

    void main(){
      float rim  = radialFactor();
      float glow = baseGlow * rim;                    // CHANGED  base filament first
      vec3  col  = baseColor * glow;                  // CHANGED  start with faint white

      // —— add travelling pulses ——
      for(int i=0;i<4;i++){
        float head = mod(time*pulseSpeed + pulseOffsets[i], 1.0+tailSize);
        float d    = vUv.y - head;

        if(d < 0.0 && d > -headSize){
          float boost = exp(d/headSize*6.0) * 1.4;
          col += hotColor * boost * rim;              // CHANGED  add warm head
          glow += boost * rim;
        }
        if(d < -headSize && d > -tailSize){
          float tail = exp((d+headSize)/tailSize*3.0) * 0.5;
          col += hotColor * tail * rim;
          glow += tail * rim;
        }
      }

      // tiny random sparks
      float h = fract(sin(dot(vUv*4373.47, vec2(12.9898,78.233)))*43758.5453);
      col += baseColor * step(0.996, h) * 1.2 * rim;

      gl_FragColor = vec4(col, min(glow,1.0));
    }
  `,
  // onInit
  (mat) => {
    mat.transparent = true;
    mat.blending = THREE.AdditiveBlending;
    mat.depthWrite = false;
    mat.side = THREE.DoubleSide;
  }
);

extend({ MultiPulseMaterial });
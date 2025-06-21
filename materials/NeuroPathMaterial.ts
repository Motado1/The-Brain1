import * as THREE from 'three';
import { shaderMaterial } from '@react-three/drei';
import { extend } from '@react-three/fiber';

export const NeuroPathMaterial = shaderMaterial(
  // Uniforms
  {
    time: 0,
    pulseSpeed: 0.6,
    pulseOffsets: [0.0, 0.25, 0.5, 0.75],
    headSize: 0.03,
    tailSize: 0.25,
    baseGlow: 0.15,                                    // slightly brighter filament
    baseColor: new THREE.Color(0xffffff),             // NEW
    hotColor: new THREE.Color('#ffb040'),
    nodeColor: new THREE.Vector3(1.0, 1.0, 1.0),     // source node color
    fadeDist: 0.4,                                     // fade distance along tube
    startPos: new THREE.Vector3(),                     // world-space start of the tube
    dirNorm: new THREE.Vector3(),                      // normalized (end-start) direction
    segLen: 1.0                                        // full length of the tube
  },
  // Vertex shader
  `
    uniform vec3 startPos;
    uniform vec3 dirNorm;
    uniform float segLen;
    varying vec2 vUv;
    varying float vT;
    void main() {
      vUv = uv;
      vec3 worldPos = (modelMatrix * vec4(position, 1.0)).xyz;
      vT = dot(worldPos - startPos, dirNorm) / segLen; // 0-1 along curve
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  // Fragment shader
  `
    uniform float time, pulseSpeed, headSize, tailSize, baseGlow, fadeDist;
    uniform float[4] pulseOffsets;
    uniform vec3  hotColor;            // orange pulse
    uniform vec3  nodeColor;           // source node color
    varying vec2  vUv;
    varying float vT;

    float radialFactor(){
      float s = sin(vUv.x * 3.14159265);
      return s*s;
    }

    void main(){
      float rim  = radialFactor();
      float glow = baseGlow * rim;
      
      // White→nodeColor mix based on distance along tube
      float mixFactor = smoothstep(0.0, fadeDist, fract(vT));
      vec3 baseCol = mix(vec3(1.0, 1.0, 1.0), nodeColor, mixFactor);
      vec3 col = baseCol * glow;

      // —— add travelling pulses ——
      for(int i=0;i<4;i++){
        float head = mod(time*pulseSpeed + pulseOffsets[i], 1.0+tailSize);
        float d    = fract(vT) - head;

        if(d < 0.0 && d > -headSize){
          float boost = exp(d/headSize*6.0) * 1.4;
          col += hotColor * boost * rim;
          glow += boost * rim;
        }
        if(d < -headSize && d > -tailSize){
          float tail = exp((d+headSize)/tailSize*3.0) * 0.5;
          col += hotColor * tail * rim;
          glow += tail * rim;
        }
      }

      // tiny random sparks using base color
      float h = fract(sin(dot(vUv*4373.47, vec2(12.9898,78.233)))*43758.5453);
      col += baseCol * step(0.996, h) * 1.2 * rim;

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

extend({ NeuroPathMaterial });
import * as THREE from 'three';
import { shaderMaterial } from '@react-three/drei';
import { extend } from '@react-three/fiber';

export const EdgePulseMaterial = shaderMaterial(
  /* uniforms */   { uTime: 0, uColor: new THREE.Color('#ffffff'), uWidth: 0.02 },
  /* vertex   */   `
      varying vec2 vUv;
      void main() {
        vUv = uv;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
  `,
  /* fragment */   `
      uniform float uTime;
      uniform vec3  uColor;
      uniform float uWidth;
      varying vec2  vUv;
      void main() {
        float pct   = fract(vUv.y - uTime);
        float alpha = smoothstep(0.0, uWidth, pct)
                    * (1.0 - smoothstep(uWidth, uWidth*2.0, pct));
        gl_FragColor = vec4(uColor, alpha);
      }
  `,
  /* onInit */     (mat) => {
    // here `mat` is the material instance: set all flags once
    mat.transparent     = true;
    mat.depthWrite      = false;
    mat.blending        = THREE.AdditiveBlending;
    mat.side            = THREE.DoubleSide;
  }
);

extend({ EdgePulseMaterial });
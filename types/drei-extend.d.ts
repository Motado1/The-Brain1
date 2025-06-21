import { ReactThreeFiber } from '@react-three/fiber'
import { EdgePulseMaterial } from '@/materials/EdgePulseMaterial'
import { MultiPulseMaterial } from '@/materials/MultiPulseMaterial'

declare global {
  namespace JSX {
    interface IntrinsicElements {
      edgePulseMaterial: ReactThreeFiber.Object3DNode<EdgePulseMaterial, typeof EdgePulseMaterial>
      multiPulseMaterial: ReactThreeFiber.Object3DNode<MultiPulseMaterial, typeof MultiPulseMaterial>
    }
  }
}
'use client';

import { ReactNode } from 'react';
import { EffectComposer, Bloom } from '@react-three/postprocessing';

interface SceneComposerProps {
  children: ReactNode;
}

export function SceneComposer({ children }: SceneComposerProps) {
  return (
    <>
      {children}
      <EffectComposer>
        <Bloom intensity={0.4} radius={0.65} mipmapBlur luminanceThreshold={0.06} luminanceSmoothing={0.08}/>
      </EffectComposer>
    </>
  );
}

// Also export as default for compatibility
export default SceneComposer;
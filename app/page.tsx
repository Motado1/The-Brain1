import CanvasScene from './components/CanvasScene';
import InfoPanel from './components/InfoPanel';

export default function BrainPage() {
  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <CanvasScene className="flex-grow" />
      <div className="w-80 bg-gray-900 bg-opacity-80 text-gray-100 backdrop-blur-md">
        <InfoPanel />
      </div>
    </div>
  );
}
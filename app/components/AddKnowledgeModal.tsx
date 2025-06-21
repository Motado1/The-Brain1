"use client";
import { useState, useRef, useEffect } from 'react';
import { supabase } from '@/lib/supabaseClient';
import { useBrainStore } from '@/lib/store';

interface AddKnowledgeModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface UploadProgress {
  id: string;
  fileName: string;
  title: string;
  status: 'uploading' | 'processing' | 'completed' | 'error';
  progress: number;
  error?: string;
  artifactId?: string;
}

export default function AddKnowledgeModal({ isOpen, onClose }: AddKnowledgeModalProps) {
  const [uploads, setUploads] = useState<UploadProgress[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const addKnowledgeItem = useBrainStore(state => state.addKnowledgeItem);
  const updateKnowledgeItem = useBrainStore(state => state.updateKnowledgeItem);

  // Real-time subscription for artifact status updates
  useEffect(() => {
    if (!isOpen) return;

    console.log('🔄 Setting up real-time subscription for artifacts...');

    const channel = supabase
      .channel('artifacts_changes')
      .on(
        'postgres_changes',
        {
          event: 'UPDATE',
          schema: 'public',
          table: 'artifacts'
        },
        (payload) => {
          console.log('📡 Artifact updated:', payload);
          handleArtifactStatusUpdate(payload.new);
        }
      )
      .subscribe((status) => {
        console.log('📡 Artifacts subscription status:', status);
      });

    return () => {
      console.log('🛑 Cleaning up artifacts subscription');
      supabase.removeChannel(channel);
    };
  }, [isOpen]);

  const handleArtifactStatusUpdate = (updatedArtifact: any) => {
    console.log('🔄 Processing artifact update:', updatedArtifact);
    
    // Update upload progress if this artifact is in our current uploads
    setUploads(prev => prev.map(upload => {
      if (upload.artifactId === updatedArtifact.id) {
        const newStatus = updatedArtifact.status === 'indexed' ? 'completed' : 
                         updatedArtifact.status === 'failed' ? 'error' : 
                         upload.status;
        
        console.log(`📊 Updating upload ${upload.fileName}: ${upload.status} → ${newStatus}`);
        
        return {
          ...upload,
          status: newStatus,
          progress: newStatus === 'completed' ? 100 : upload.progress
        };
      }
      return upload;
    }));

    // Update Zustand store
    updateKnowledgeItem(updatedArtifact.id, {
      status: updatedArtifact.status,
      processingStatus: updatedArtifact.status === 'indexed' ? 'completed' : 
                       updatedArtifact.status === 'failed' ? 'failed' : 'processing'
    });

    // Show notification
    if (updatedArtifact.status === 'indexed') {
      console.log('✅ Document processing completed:', updatedArtifact.name);
      showNotification('✅ Processing Complete!', `${updatedArtifact.name} is now searchable in The Brain`);
    } else if (updatedArtifact.status === 'failed') {
      console.log('❌ Document processing failed:', updatedArtifact.name);
      showNotification('❌ Processing Failed', `Failed to process ${updatedArtifact.name}`);
    }
  };

  const showNotification = (title: string, message: string) => {
    // Simple console notification - could be enhanced with toast library
    console.log(`🔔 ${title}: ${message}`);
    
    // You could integrate with a toast notification library here
    // For now, we'll update the UI state to show the status
  };

  const handleFileSelect = (files: FileList | null) => {
    if (!files || files.length === 0) return;

    Array.from(files).forEach(file => {
      const uploadId = crypto.randomUUID();
      const upload: UploadProgress = {
        id: uploadId,
        fileName: file.name,
        title: file.name.replace(/\.[^/.]+$/, ''), // Remove extension for default title
        status: 'uploading',
        progress: 0
      };

      setUploads(prev => [...prev, upload]);
      processFileUpload(file, upload);
    });
  };

  const processFileUpload = async (file: File, upload: UploadProgress) => {
    try {
      // Step 1: Upload file to Supabase Storage
      const fileExt = file.name.split('.').pop();
      const fileName = `${crypto.randomUUID()}-${file.name}`;
      const filePath = `raw/${fileName}`;

      console.log('📤 Uploading file:', file.name, 'to', filePath);

      const { data: uploadData, error: uploadError } = await supabase.storage
        .from('knowledge')
        .upload(filePath, file, {
          cacheControl: '3600',
          upsert: false
        });

      if (uploadError) {
        throw new Error(`Upload failed: ${uploadError.message}`);
      }

      // Update progress to show upload complete
      setUploads(prev => prev.map(u => 
        u.id === upload.id 
          ? { ...u, status: 'processing', progress: 50 }
          : u
      ));

      console.log('✅ File uploaded to:', uploadData.path);

      // Step 2: Create artifact record and trigger processing
      const artifactData = {
        name: upload.title,
        description: `Uploaded file: ${file.name}`,
        type: 'file',
        storagePath: `knowledge/${filePath}`,
        contentType: file.type,
        fileSize: file.size,
        originalFileName: file.name
      };

      console.log('📋 Creating artifact:', artifactData);

      const response = await fetch('/api/artifacts', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(artifactData)
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to create artifact');
      }

      const result = await response.json();
      console.log('✅ Artifact created:', result);

      // Update upload status to completed
      setUploads(prev => prev.map(u => 
        u.id === upload.id 
          ? { 
              ...u, 
              status: 'completed', 
              progress: 100,
              artifactId: result.artifact?.id 
            }
          : u
      ));

      // Add to Zustand store for optimistic UI
      if (result.artifact) {
        addKnowledgeItem({
          id: result.artifact.id,
          name: result.artifact.name,
          type: 'file',
          status: 'processing',
          fileName: file.name,
          fileSize: file.size,
          uploadedAt: new Date().toISOString(),
          processingStatus: 'queued'
        });
      }

      console.log('🎉 Upload and processing initiated successfully');

    } catch (error) {
      console.error('❌ Upload failed:', error);
      
      setUploads(prev => prev.map(u => 
        u.id === upload.id 
          ? { 
              ...u, 
              status: 'error', 
              error: error instanceof Error ? error.message : 'Unknown error'
            }
          : u
      ));
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    handleFileSelect(e.dataTransfer.files);
  };

  const clearCompletedUploads = () => {
    setUploads(prev => prev.filter(u => u.status !== 'completed'));
  };

  const updateUploadTitle = (uploadId: string, newTitle: string) => {
    setUploads(prev => prev.map(u => 
      u.id === uploadId ? { ...u, title: newTitle } : u
    ));
  };

  const getStatusIcon = (status: UploadProgress['status']) => {
    switch (status) {
      case 'uploading':
        return '📤';
      case 'processing':
        return '⚙️';
      case 'completed':
        return '✅';
      case 'error':
        return '❌';
      default:
        return '📄';
    }
  };

  const getStatusText = (status: UploadProgress['status']) => {
    switch (status) {
      case 'uploading':
        return 'Uploading file...';
      case 'processing':
        return 'Processing for RAG...';
      case 'completed':
        return 'Ready for search! 🎉';
      case 'error':
        return 'Upload failed';
      default:
        return 'Unknown';
    }
  };

  const getStatusColor = (status: UploadProgress['status']) => {
    switch (status) {
      case 'uploading':
        return 'text-blue-400';
      case 'processing':
        return 'text-yellow-400';
      case 'completed':
        return 'text-green-400';
      case 'error':
        return 'text-red-400';
      default:
        return 'text-gray-400';
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-full max-w-2xl max-h-[80vh] overflow-y-auto">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-xl font-bold text-white">Add Knowledge to The Brain</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white text-2xl"
          >
            ×
          </button>
        </div>

        {/* Upload Area */}
        <div
          className={`border-2 border-dashed rounded-lg p-8 text-center transition-colors ${
            isDragOver 
              ? 'border-blue-400 bg-blue-900 bg-opacity-20' 
              : 'border-gray-600 hover:border-gray-500'
          }`}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
        >
          <div className="mb-4">
            <div className="text-4xl mb-2">📁</div>
            <p className="text-gray-300 mb-2">
              Drag and drop files here, or click to select
            </p>
            <p className="text-sm text-gray-500">
              Supports: PDF, TXT, MD, DOC, DOCX, CSV, JSON
            </p>
          </div>
          
          <button
            onClick={() => fileInputRef.current?.click()}
            className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-lg transition-colors"
          >
            Choose Files
          </button>
          
          <input
            ref={fileInputRef}
            type="file"
            multiple
            accept=".pdf,.txt,.md,.doc,.docx,.csv,.json"
            onChange={(e) => handleFileSelect(e.target.files)}
            className="hidden"
          />
        </div>

        {/* Upload Progress List */}
        {uploads.length > 0 && (
          <div className="mt-6">
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-lg font-semibold text-white">Upload Progress</h3>
              {uploads.some(u => u.status === 'completed') && (
                <button
                  onClick={clearCompletedUploads}
                  className="text-sm text-gray-400 hover:text-white"
                >
                  Clear Completed
                </button>
              )}
            </div>
            
            <div className="space-y-3 max-h-60 overflow-y-auto">
              {uploads.map((upload) => (
                <div key={upload.id} className="bg-gray-700 rounded-lg p-4">
                  <div className="flex items-start justify-between mb-2">
                    <div className="flex items-center space-x-2">
                      <span className={`text-xl ${upload.status === 'processing' ? 'animate-spin' : ''}`}>
                        {getStatusIcon(upload.status)}
                      </span>
                      <div>
                        <input
                          type="text"
                          value={upload.title}
                          onChange={(e) => updateUploadTitle(upload.id, e.target.value)}
                          className="bg-transparent text-white font-medium border-none outline-none hover:bg-gray-600 rounded px-1"
                          disabled={upload.status === 'uploading'}
                        />
                        <p className="text-sm text-gray-400">{upload.fileName}</p>
                      </div>
                    </div>
                    <div className="text-right">
                      <span className={`text-sm font-medium ${getStatusColor(upload.status)}`}>
                        {getStatusText(upload.status)}
                      </span>
                      {upload.status === 'processing' && (
                        <div className="text-xs text-gray-500 mt-1">
                          🤖 AI processing...
                        </div>
                      )}
                      {upload.status === 'completed' && (
                        <div className="text-xs text-green-500 mt-1">
                          🔍 Searchable now!
                        </div>
                      )}
                    </div>
                  </div>
                  
                  {upload.status === 'uploading' || upload.status === 'processing' ? (
                    <div className="w-full bg-gray-600 rounded-full h-2">
                      <div
                        className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                        style={{ width: `${upload.progress}%` }}
                      ></div>
                    </div>
                  ) : upload.status === 'error' ? (
                    <div className="text-red-400 text-sm mt-2">
                      Error: {upload.error}
                    </div>
                  ) : upload.status === 'completed' ? (
                    <div className="text-green-400 text-sm mt-2">
                      Artifact ID: {upload.artifactId}
                    </div>
                  ) : null}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Instructions */}
        <div className="mt-6 p-4 bg-gray-700 rounded-lg">
          <h4 className="text-sm font-semibold text-white mb-2">What happens next?</h4>
          <ul className="text-sm text-gray-300 space-y-1">
            <li>1. 📤 Files are uploaded to secure storage</li>
            <li>2. ⚡ Processing starts immediately (no waiting!)</li>
            <li>3. ⚙️ Text is extracted and analyzed by AI</li>
            <li>4. 🧠 Knowledge is embedded for semantic search</li>
            <li>5. ✨ Content becomes instantly searchable</li>
          </ul>
          <div className="mt-3 text-xs text-blue-300 bg-blue-900 bg-opacity-30 p-2 rounded">
            💡 <strong>Instant Processing:</strong> Your files are processed immediately upon upload, not scheduled for later!
          </div>
        </div>

        {/* Footer */}
        <div className="mt-6 flex justify-end space-x-3">
          <button
            onClick={onClose}
            className="px-4 py-2 text-gray-400 hover:text-white transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
'use client';

import { useState, useEffect } from 'react';
import { useKnowledgeStore } from '@/stores/knowledge';

interface KnowledgeSearchProps {
  onSelect?: (item: any) => void;
  placeholder?: string;
  className?: string;
}

export default function KnowledgeSearch({ 
  onSelect, 
  placeholder = "Search knowledge base...",
  className = ""
}: KnowledgeSearchProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [localQuery, setLocalQuery] = useState('');
  
  const { 
    searchQuery, 
    setSearchQuery, 
    searchItems, 
    items,
    isLoading 
  } = useKnowledgeStore();

  const searchResults = searchItems(localQuery || searchQuery);

  useEffect(() => {
    // Debounce search
    const timeout = setTimeout(() => {
      setSearchQuery(localQuery);
    }, 300);
    
    return () => clearTimeout(timeout);
  }, [localQuery, setSearchQuery]);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setLocalQuery(value);
    setIsOpen(value.length > 0);
  };

  const handleItemSelect = (item: any) => {
    setIsOpen(false);
    setLocalQuery('');
    onSelect?.(item);
  };

  const handleInputFocus = () => {
    if (localQuery.length > 0) {
      setIsOpen(true);
    }
  };

  const handleInputBlur = () => {
    // Delay to allow clicking on results
    setTimeout(() => setIsOpen(false), 200);
  };

  return (
    <div className={`relative ${className}`}>
      <div className="relative">
        <input
          type="text"
          value={localQuery}
          onChange={handleInputChange}
          onFocus={handleInputFocus}
          onBlur={handleInputBlur}
          placeholder={placeholder}
          className="w-full px-4 py-2 pr-10 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        
        {/* Search icon */}
        <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
          <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </div>
      </div>

      {/* Search Results Dropdown */}
      {isOpen && (
        <div className="absolute z-50 w-full mt-1 bg-white border border-gray-200 rounded-lg shadow-lg max-h-96 overflow-y-auto">
          {isLoading ? (
            <div className="p-4 text-center text-gray-500">
              Searching...
            </div>
          ) : searchResults.length > 0 ? (
            <>
              {searchResults.map((item) => (
                <div
                  key={item.id}
                  onClick={() => handleItemSelect(item)}
                  className="p-3 hover:bg-gray-50 cursor-pointer border-b border-gray-100 last:border-b-0"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <h4 className="font-medium text-gray-900 text-sm mb-1">
                        {item.title}
                      </h4>
                      <p className="text-xs text-gray-600 line-clamp-2">
                        {item.content.slice(0, 120)}...
                      </p>
                      
                      {/* Tags */}
                      {item.tags && item.tags.length > 0 && (
                        <div className="flex flex-wrap gap-1 mt-2">
                          {item.tags.slice(0, 3).map((tag: string) => (
                            <span
                              key={tag}
                              className="inline-block px-2 py-1 text-xs bg-blue-100 text-blue-800 rounded-full"
                            >
                              {tag}
                            </span>
                          ))}
                          {item.tags.length > 3 && (
                            <span className="inline-block px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded-full">
                              +{item.tags.length - 3}
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                    
                    <div className="ml-3 flex-shrink-0">
                      <span className="inline-block px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded-full capitalize">
                        {item.type}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </>
          ) : localQuery.length > 0 ? (
            <div className="p-4 text-center text-gray-500">
              No results found for "{localQuery}"
            </div>
          ) : (
            <div className="p-4 text-center text-gray-500">
              Start typing to search...
            </div>
          )}
        </div>
      )}
    </div>
  );
}
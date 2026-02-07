/**
 * DynamicPipelineUI - Runtime JS Module Loader
 * 
 * NO REBUILD REQUIRED when adding new pipeline UIs!
 * 
 * How it works:
 * 1. Each pipeline with a UI has a ui/component.js file
 * 2. ThemeLoader retrieves this JS module from backend
 * 3. This loader executes the module and renders the component
 * 
 * Pipeline UI files are located at:
 *   pipelines/core/{pipeline_name}/ui/component.js
 * 
 * Each JS module exports a render function that receives:
 * - container: DOM element to render into
 * - props: { pipelineId, executePipeline, sharedState, onClose }
 * - React/hooks if needed
 * 
 * The module can use vanilla JS or React - it's fully flexible.
 */

import React, { useEffect, useRef, useState, useCallback } from 'react';

// Module cache to avoid reloading
const moduleCache = new Map<number, PipelineUIModule>();

// Shared state that all tabs can access
let sharedState: Record<string, any> = {};
const stateListeners: Set<(state: Record<string, any>) => void> = new Set();

export function getSharedState() {
  return sharedState;
}

export function setSharedState(updates: Record<string, any>) {
  sharedState = { ...sharedState, ...updates };
  stateListeners.forEach(listener => listener(sharedState));
}

export function subscribeToState(listener: (state: Record<string, any>) => void) {
  stateListeners.add(listener);
  return () => stateListeners.delete(listener);
}

// Pipeline UI module interface
export interface PipelineUIModule {
  // The render function that creates the UI
  render: (
    container: HTMLElement,
    props: PipelineUIProps,
    React: typeof import('react'),
    ReactDOM: typeof import('react-dom/client')
  ) => (() => void) | void;  // Returns optional cleanup function
  
  // Optional metadata
  meta?: {
    title?: string;
    icon?: string;
    version?: string;
  };
  
  // Optional callbacks for tab lifecycle
  onActivate?: () => void;
  onDeactivate?: () => void;
}

export interface PipelineUIProps {
  pipelineId: number;
  executePipeline: (pipelineId: number, input: any) => Promise<any>;
  getSharedState: () => Record<string, any>;
  setSharedState: (updates: Record<string, any>) => void;
  subscribeToState: (listener: (state: Record<string, any>) => void) => () => void;
  onClose?: () => void;
  initialData?: any;
}

/**
 * Load a pipeline UI module from the backend
 */
export async function loadPipelineUIModule(pipelineId: number): Promise<PipelineUIModule | null> {
  // Check cache first
  if (moduleCache.has(pipelineId)) {
    return moduleCache.get(pipelineId)!;
  }
  
  try {
    if (window.ozone?.pipeline?.execute) {
      // v0.4.0: Use GetPipelineUIComponent for component.js files
      const result = await window.ozone.pipeline.execute(2, {
        action: 'GetPipelineUIComponent',
        pipeline_id: pipelineId,
      });
      
      // component_js is the field returned by ThemeLoader
      if (result?.component_js) {
        // Execute the JS code to get the module
        const module = evalPipelineModule(result.component_js, pipelineId);
        if (module) {
          moduleCache.set(pipelineId, module);
          return module;
        }
      }
    }
    
    return null;
  } catch (e) {
    console.error(`Failed to load UI module for pipeline ${pipelineId}:`, e);
    return null;
  }
}

/**
 * Safely evaluate a pipeline UI module
 */
function evalPipelineModule(jsCode: string, pipelineId: number): PipelineUIModule | null {
  try {
    // Create a sandboxed execution context
    const moduleExports: any = {};
    
    // The module code should set module.exports or exports
    const wrappedCode = `
      (function(module, exports, pipelineId) {
        ${jsCode}
      })(moduleExports, moduleExports, ${pipelineId});
    `;
    
    // Execute in context
    const moduleExportsRef = { exports: {} };
    const fn = new Function('moduleExports', 'pipelineId', `
      const module = { exports: moduleExports.exports };
      const exports = module.exports;
      ${jsCode}
      moduleExports.exports = module.exports;
    `);
    
    fn(moduleExportsRef, pipelineId);
    
    const result = moduleExportsRef.exports as any;
    
    // Validate it has a render function
    if (typeof result.render === 'function') {
      return result as PipelineUIModule;
    }
    
    console.warn(`Pipeline ${pipelineId} UI module missing render function`);
    return null;
  } catch (e) {
    console.error(`Failed to evaluate pipeline ${pipelineId} UI module:`, e);
    return null;
  }
}

/**
 * Check if a pipeline has a UI module
 */
export async function hasPipelineUI(pipelineId: number): Promise<boolean> {
  if (moduleCache.has(pipelineId)) {
    return true;
  }
  
  try {
    // Check registry for has_ui flag
    if (window.ozone?.pipeline?.execute) {
      const result = await window.ozone.pipeline.execute(2, {
        action: 'GetPipelineRegistry',
      });
      if (result?.registry) {
        const entry = result.registry.find((p: any) => p.id === pipelineId);
        return entry?.has_ui === true;
      }
    }
    return false;
  } catch {
    return false;
  }
}

/**
 * Clear the module cache (useful when pipelines are updated)
 */
export function clearModuleCache(pipelineId?: number) {
  if (pipelineId !== undefined) {
    moduleCache.delete(pipelineId);
  } else {
    moduleCache.clear();
  }
}

/**
 * React component that renders a dynamic pipeline UI
 */
interface DynamicPipelineUIProps {
  pipelineId: number;
  initialData?: any;
  onClose?: () => void;
}

export function DynamicPipelineUI({ pipelineId, initialData, onClose }: DynamicPipelineUIProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const cleanupRef = useRef<(() => void) | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  const executePipeline = useCallback(async (pid: number, input: any) => {
    if (window.ozone?.pipeline?.execute) {
      return await window.ozone.pipeline.execute(pid, input);
    }
    throw new Error('Pipeline execution not available');
  }, []);
  
  useEffect(() => {
    let mounted = true;
    
    async function loadAndRender() {
      if (!containerRef.current) return;
      
      try {
        const module = await loadPipelineUIModule(pipelineId);
        
        if (!mounted) return;
        
        if (!module) {
          setError(`No UI module found for pipeline ${pipelineId}`);
          setLoading(false);
          return;
        }
        
        // Dynamically import React and ReactDOM for the module
        const React = await import('react');
        const ReactDOM = await import('react-dom/client');
        
        // Call the module's render function
        const cleanup = module.render(
          containerRef.current,
          {
            pipelineId,
            executePipeline,
            getSharedState,
            setSharedState,
            subscribeToState,
            onClose,
            initialData,
          },
          React,
          ReactDOM
        );
        
        if (typeof cleanup === 'function') {
          cleanupRef.current = cleanup;
        }
        
        // Call onActivate if provided
        module.onActivate?.();
        
        setLoading(false);
      } catch (e: any) {
        if (mounted) {
          setError(e.message || 'Failed to load UI module');
          setLoading(false);
        }
      }
    }
    
    loadAndRender();
    
    return () => {
      mounted = false;
      // Run cleanup
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
    };
  }, [pipelineId, executePipeline, onClose, initialData]);
  
  if (loading) {
    return (
      <div className="dynamic-pipeline-ui loading">
        <div className="loading-spinner" />
        <p>Loading pipeline UI...</p>
      </div>
    );
  }
  
  if (error) {
    return (
      <div className="dynamic-pipeline-ui error">
        <div className="ui-error">
          <span>⚠️</span>
          <span>{error}</span>
        </div>
      </div>
    );
  }
  
  return <div ref={containerRef} className="dynamic-pipeline-ui-container" />;
}

export default DynamicPipelineUI;

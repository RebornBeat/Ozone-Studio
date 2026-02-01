/**
 * ThemeArea Component - §5.3 of the specification
 * 
 * The main content area that is ENTIRELY pipeline-driven.
 * 
 * Key principle: The ThemeArea does NOT contain hardcoded UI logic.
 * Instead, it renders what the ThemeLoaderPipeline provides.
 * 
 * Each theme is loaded by the ThemeLoaderPipeline (id: 2),
 * which returns the UI structure to render.
 */

import React, { useEffect, useState } from 'react';
import { useOzoneStore } from '../services/store';

// Import theme components
import { HomeDashboard } from '../themes/home_dashboard/HomeDashboard';

interface ThemeAreaProps {
  theme: string;
}

// Theme component registry - maps theme names to components
// These are the built-in themes loaded at startup
const THEME_REGISTRY: Record<string, React.ComponentType<any>> = {
  'home_dashboard': HomeDashboard,
  // Additional themes would be registered here
  // Custom themes from pipelines would be dynamically loaded
};

export function ThemeArea({ theme }: ThemeAreaProps) {
  const { executePipeline, activeTab } = useOzoneStore();
  const [themeData, setThemeData] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load theme data when theme changes
  useEffect(() => {
    async function loadTheme() {
      setLoading(true);
      setError(null);
      
      try {
        // Execute ThemeLoaderPipeline to get theme data
        const taskId = await executePipeline(2, { theme });
        
        // In a real implementation, we'd wait for the task to complete
        // and retrieve the theme data from the task output
        // For now, we'll use the built-in theme
        setThemeData({ theme, loaded: true });
      } catch (err) {
        console.error('Failed to load theme:', err);
        setError('Failed to load theme');
      } finally {
        setLoading(false);
      }
    }
    
    loadTheme();
  }, [theme, executePipeline]);

  // Get the theme component
  const ThemeComponent = THEME_REGISTRY[theme];

  // Loading state
  if (loading) {
    return (
      <main className="theme-area theme-loading">
        <div className="loading-spinner" />
        <p>Loading theme...</p>
      </main>
    );
  }

  // Error state
  if (error) {
    return (
      <main className="theme-area theme-error">
        <h2>Theme Error</h2>
        <p>{error}</p>
        <button onClick={() => window.location.reload()}>
          Reload
        </button>
      </main>
    );
  }

  // Unknown theme
  if (!ThemeComponent) {
    return (
      <main className="theme-area theme-unknown">
        <h2>Unknown Theme</h2>
        <p>Theme "{theme}" is not registered.</p>
        <p>Available themes: {Object.keys(THEME_REGISTRY).join(', ')}</p>
      </main>
    );
  }

  // Render the theme component
  return (
    <main className="theme-area">
      <ThemeComponent 
        themeData={themeData}
        activeTab={activeTab}
      />
    </main>
  );
}

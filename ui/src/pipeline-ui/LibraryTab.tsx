/**
 * LibraryTab UI Component
 * Pipeline #7 - LibraryTabPipeline
 * 
 * Browse and search the library of:
 * - Pipelines (executable units)
 * - Methodologies (reusable approaches)
 * - Blueprints (task templates)
 * - Categories (organizational structure)
 * 
 * gRPC calls via window.ozone.pipeline.execute(7, input)
 */

import React, { useEffect, useState } from 'react';

interface Category {
  category_id: number;
  name: string;
  description?: string;
  modality: string;
  parent_id?: number;
  child_count: number;
  methodology_count: number;
}

interface MethodologySummary {
  methodology_id: number;
  name: string;
  description: string;
  category_id: number;
  keywords: string[];
  use_count: number;
}

interface BlueprintSummary {
  blueprint_id: number;
  name: string;
  description: string;
  input_types: string[];
  output_type: string;
  step_count: number;
}

interface PipelineSummary {
  pipeline_id: number;
  name: string;
  description: string;
  is_builtin: boolean;
  author: string;
}

type ViewMode = 'overview' | 'pipelines' | 'methodologies' | 'blueprints' | 'categories';

export function LibraryTab() {
  const [viewMode, setViewMode] = useState<ViewMode>('overview');
  const [pipelines, setPipelines] = useState<PipelineSummary[]>([]);
  const [methodologies, setMethodologies] = useState<MethodologySummary[]>([]);
  const [blueprints, setBlueprints] = useState<BlueprintSummary[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Counts for overview
  const [counts, setCounts] = useState({
    pipelines: 54,
    methodologies: 0,
    blueprints: 0,
    categories: 0,
  });

  const executePipeline = async (pipelineId: number, input: any) => {
    if (window.ozone?.pipeline?.execute) {
      return await window.ozone.pipeline.execute(pipelineId, input);
    }
    throw new Error('Pipeline execution not available');
  };

  const loadPipelines = async () => {
    setLoading(true);
    try {
      const result = await executePipeline(7, { action: 'ListPipelines', builtin_only: false });
      if (result?.pipelines) {
        setPipelines(result.pipelines);
        setCounts(prev => ({ ...prev, pipelines: result.pipelines.length }));
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load pipelines');
    }
    setLoading(false);
  };

  const loadMethodologies = async () => {
    setLoading(true);
    try {
      const keywords = searchQuery ? searchQuery.split(' ').filter(k => k.trim()) : [];
      const result = await executePipeline(7, { 
        action: 'SearchMethodologies', 
        keywords,
        limit: 50 
      });
      if (result?.methodologies) {
        setMethodologies(result.methodologies);
        setCounts(prev => ({ ...prev, methodologies: result.methodologies.length }));
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load methodologies');
    }
    setLoading(false);
  };

  const loadBlueprints = async () => {
    setLoading(true);
    try {
      const keywords = searchQuery ? searchQuery.split(' ').filter(k => k.trim()) : [];
      const result = await executePipeline(7, { 
        action: 'SearchBlueprints', 
        keywords,
        limit: 50 
      });
      if (result?.blueprints) {
        setBlueprints(result.blueprints);
        setCounts(prev => ({ ...prev, blueprints: result.blueprints.length }));
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load blueprints');
    }
    setLoading(false);
  };

  const loadCategories = async () => {
    setLoading(true);
    try {
      const result = await executePipeline(7, { action: 'GetCategories' });
      if (result?.categories) {
        setCategories(result.categories);
        setCounts(prev => ({ ...prev, categories: result.categories.length }));
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load categories');
    }
    setLoading(false);
  };

  // Load data when view mode changes
  useEffect(() => {
    switch (viewMode) {
      case 'pipelines':
        loadPipelines();
        break;
      case 'methodologies':
        loadMethodologies();
        break;
      case 'blueprints':
        loadBlueprints();
        break;
      case 'categories':
        loadCategories();
        break;
    }
  }, [viewMode]);

  const handleSearch = () => {
    switch (viewMode) {
      case 'methodologies':
        loadMethodologies();
        break;
      case 'blueprints':
        loadBlueprints();
        break;
    }
  };

  const renderOverview = () => (
    <div className="library-overview">
      <div className="library-cards">
        <div 
          className="library-card"
          onClick={() => setViewMode('pipelines')}
        >
          <span className="lib-icon">🔧</span>
          <div className="lib-content">
            <h3>Pipelines</h3>
            <p className="lib-count">{counts.pipelines} available</p>
            <p className="lib-desc">Execution pipelines for various tasks</p>
          </div>
          <span className="lib-arrow">→</span>
        </div>

        <div 
          className="library-card"
          onClick={() => setViewMode('methodologies')}
        >
          <span className="lib-icon">📐</span>
          <div className="lib-content">
            <h3>Methodologies</h3>
            <p className="lib-count">{counts.methodologies} available</p>
            <p className="lib-desc">Reusable problem-solving approaches</p>
          </div>
          <span className="lib-arrow">→</span>
        </div>

        <div 
          className="library-card"
          onClick={() => setViewMode('blueprints')}
        >
          <span className="lib-icon">📘</span>
          <div className="lib-content">
            <h3>Blueprints</h3>
            <p className="lib-count">{counts.blueprints} available</p>
            <p className="lib-desc">Task templates with defined steps</p>
          </div>
          <span className="lib-arrow">→</span>
        </div>

        <div 
          className="library-card"
          onClick={() => setViewMode('categories')}
        >
          <span className="lib-icon">📂</span>
          <div className="lib-content">
            <h3>Categories</h3>
            <p className="lib-count">{counts.categories} available</p>
            <p className="lib-desc">Organizational structure</p>
          </div>
          <span className="lib-arrow">→</span>
        </div>
      </div>
    </div>
  );

  const renderPipelines = () => (
    <div className="library-list">
      {pipelines.length === 0 && !loading ? (
        <div className="panel-empty-centered">
          <span className="empty-icon">🔧</span>
          <p>No pipelines found</p>
        </div>
      ) : (
        <div className="items-list">
          {pipelines.map((pipeline) => (
            <div key={pipeline.pipeline_id} className="list-item">
              <div className="item-icon">
                {pipeline.is_builtin ? '🔧' : '🔨'}
              </div>
              <div className="item-content">
                <div className="item-header">
                  <span className="item-name">{pipeline.name}</span>
                  <span className="item-id">#{pipeline.pipeline_id}</span>
                </div>
                <p className="item-desc">{pipeline.description}</p>
                <div className="item-meta">
                  <span className={`item-badge ${pipeline.is_builtin ? 'builtin' : 'custom'}`}>
                    {pipeline.is_builtin ? 'Built-in' : 'Custom'}
                  </span>
                  <span className="item-author">by {pipeline.author}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  const renderMethodologies = () => (
    <div className="library-list">
      <div className="search-bar">
        <input
          type="text"
          placeholder="Search methodologies by keywords..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
        />
        <button onClick={handleSearch}>Search</button>
      </div>

      {methodologies.length === 0 && !loading ? (
        <div className="panel-empty-centered">
          <span className="empty-icon">📐</span>
          <p>No methodologies found</p>
          <p className="empty-hint">Try different search keywords</p>
        </div>
      ) : (
        <div className="items-list">
          {methodologies.map((methodology) => (
            <div key={methodology.methodology_id} className="list-item">
              <div className="item-icon">📐</div>
              <div className="item-content">
                <div className="item-header">
                  <span className="item-name">{methodology.name}</span>
                  <span className="item-uses">Used {methodology.use_count}×</span>
                </div>
                <p className="item-desc">{methodology.description}</p>
                <div className="item-tags">
                  {methodology.keywords.map((kw, idx) => (
                    <span key={idx} className="item-tag">{kw}</span>
                  ))}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  const renderBlueprints = () => (
    <div className="library-list">
      <div className="search-bar">
        <input
          type="text"
          placeholder="Search blueprints by keywords..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
        />
        <button onClick={handleSearch}>Search</button>
      </div>

      {blueprints.length === 0 && !loading ? (
        <div className="panel-empty-centered">
          <span className="empty-icon">📘</span>
          <p>No blueprints found</p>
          <p className="empty-hint">Try different search keywords</p>
        </div>
      ) : (
        <div className="items-list">
          {blueprints.map((blueprint) => (
            <div key={blueprint.blueprint_id} className="list-item">
              <div className="item-icon">📘</div>
              <div className="item-content">
                <div className="item-header">
                  <span className="item-name">{blueprint.name}</span>
                  <span className="item-steps">{blueprint.step_count} steps</span>
                </div>
                <p className="item-desc">{blueprint.description}</p>
                <div className="item-meta">
                  <span className="item-types">
                    {blueprint.input_types.join(', ')} → {blueprint.output_type}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  const renderCategories = () => (
    <div className="library-list">
      {categories.length === 0 && !loading ? (
        <div className="panel-empty-centered">
          <span className="empty-icon">📂</span>
          <p>No categories found</p>
        </div>
      ) : (
        <div className="items-list">
          {categories.map((category) => (
            <div key={category.category_id} className="list-item">
              <div className="item-icon">📂</div>
              <div className="item-content">
                <div className="item-header">
                  <span className="item-name">{category.name}</span>
                  <span className="item-modality">{category.modality}</span>
                </div>
                {category.description && (
                  <p className="item-desc">{category.description}</p>
                )}
                <div className="item-meta">
                  <span className="item-count">{category.child_count} subcategories</span>
                  <span className="item-count">{category.methodology_count} methodologies</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  return (
    <div className="library-tab">
      <div className="panel-toolbar">
        {viewMode !== 'overview' && (
          <button className="back-btn" onClick={() => setViewMode('overview')}>
            ← Library
          </button>
        )}
        <h2>
          {viewMode === 'overview' && 'Library'}
          {viewMode === 'pipelines' && 'Pipelines'}
          {viewMode === 'methodologies' && 'Methodologies'}
          {viewMode === 'blueprints' && 'Blueprints'}
          {viewMode === 'categories' && 'Categories'}
        </h2>
      </div>

      {error && (
        <div className="panel-error">
          <span className="error-icon">⚠️</span>
          <p>{error}</p>
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      {loading ? (
        <div className="panel-loading">
          <div className="loading-spinner" />
          <p>Loading...</p>
        </div>
      ) : (
        <>
          {viewMode === 'overview' && renderOverview()}
          {viewMode === 'pipelines' && renderPipelines()}
          {viewMode === 'methodologies' && renderMethodologies()}
          {viewMode === 'blueprints' && renderBlueprints()}
          {viewMode === 'categories' && renderCategories()}
        </>
      )}
    </div>
  );
}

export default LibraryTab;

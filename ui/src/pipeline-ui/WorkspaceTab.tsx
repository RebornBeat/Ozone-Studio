/**
 * WorkspaceTab UI Component
 * Pipeline #6 - WorkspaceTabPipeline
 * 
 * Manages workspaces and projects.
 * - Workspaces organize related projects (e.g., "Coding", "Research")
 * - Projects contain linked files/URLs with scoped chat
 * 
 * gRPC calls via window.ozone.pipeline.execute(6, input)
 */

import React, { useEffect, useState, useRef } from 'react';

interface Workspace {
  workspace_id: number;
  name: string;
  description?: string;
  project_count: number;
  created_at: number;
  updated_at: number;
}

interface Project {
  project_id: number;
  workspace_id: number;
  name: string;
  description?: string;
  file_count: number;
  url_count: number;
  package_count: number;
  created_at: number;
  updated_at: number;
}

interface FileRef {
  file_ref_id: number;
  path: string;
  name: string;
  modality: string;
  size_bytes: number;
  last_modified: number;
}

interface ChatMessage {
  id: number;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
}

// ============================================================================
// ProjectChat Component - Scoped chat within a project
// ============================================================================
interface ProjectChatProps {
  projectId: number;
  workspaceId?: number;
  projectName: string;
}

function ProjectChat({ projectId, workspaceId, projectName }: ProjectChatProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const messagesRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll on new messages
  useEffect(() => {
    if (messagesRef.current) {
      messagesRef.current.scrollTop = messagesRef.current.scrollHeight;
    }
  }, [messages]);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!input.trim() || loading) return;

    const userMessage: ChatMessage = {
      id: Date.now(),
      role: 'user',
      content: input,
      timestamp: Date.now(),
    };
    setMessages(prev => [...prev, userMessage]);
    setInput('');
    setLoading(true);

    try {
      // Use orchestrator with project context
      if ((window as any).ozone?.orchestrate) {
        const result = await (window as any).ozone.orchestrate({
          prompt: input,
          project_id: projectId,
          workspace_id: workspaceId,
          user_id: 1,
          device_id: 1,
          consciousness_enabled: (window as any).ozone?.config?.consciousness_enabled ?? false,
          token_budget: 100000,
        });

        if (result.response) {
          setMessages(prev => [...prev, {
            id: Date.now(),
            role: 'assistant',
            content: result.response,
            timestamp: Date.now(),
          }]);
        } else if (result.error) {
          setMessages(prev => [...prev, {
            id: Date.now(),
            role: 'assistant',
            content: `Error: ${result.error}`,
            timestamp: Date.now(),
          }]);
        }
      } else {
        // Fallback: aggregate context and call prompt directly
        let aggregatedContext = '';
        
        // First get project context
        try {
          const contextResult = await (window as any).ozone?.pipeline?.execute(21, {
            action: 'ForProject',
            project_id: projectId,
            token_budget: 50000,
          });
          aggregatedContext = contextResult?.context?.context_text || '';
        } catch (err) {
          console.warn('Failed to aggregate context:', err);
        }

        // Then call prompt pipeline
        const promptResult = await (window as any).ozone?.pipeline?.execute(9, {
          prompt: input,
          aggregated_context: aggregatedContext,
        });

        if (promptResult?.response) {
          setMessages(prev => [...prev, {
            id: Date.now(),
            role: 'assistant',
            content: promptResult.response,
            timestamp: Date.now(),
          }]);
        }
      }
    } catch (err: any) {
      setMessages(prev => [...prev, {
        id: Date.now(),
        role: 'assistant',
        content: `Error: ${err.message || 'Failed to process request'}`,
        timestamp: Date.now(),
      }]);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="project-chat">
      <div className="chat-header">
        <span className="chat-icon">💬</span>
        <span className="chat-title">Chat with {projectName}</span>
        <span className="chat-scope">Context: {projectName} files & URLs</span>
      </div>

      <div className="chat-messages" ref={messagesRef}>
        {messages.length === 0 ? (
          <div className="chat-empty">
            <span className="empty-icon">✨</span>
            <p>Start chatting within this project's context</p>
            <p className="empty-hint">Your questions will use linked files and URLs as context</p>
          </div>
        ) : (
          messages.map(msg => (
            <div key={msg.id} className={`chat-message ${msg.role}`}>
              <div className="message-header">
                <span className="message-author">{msg.role === 'user' ? 'You' : 'OZONE'}</span>
                <span className="message-time">
                  {new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                </span>
              </div>
              <div className="message-content">{msg.content}</div>
            </div>
          ))
        )}
        {loading && (
          <div className="chat-message assistant loading">
            <div className="message-content">
              <span className="loading-dots">Processing</span>
            </div>
          </div>
        )}
      </div>

      <form className="chat-input-form" onSubmit={handleSubmit}>
        <textarea
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={`Ask about ${projectName}...`}
          rows={1}
          disabled={loading}
        />
        <button type="submit" disabled={!input.trim() || loading}>
          {loading ? '...' : '→'}
        </button>
      </form>
    </div>
  );
}

// ============================================================================
// Main WorkspaceTab Component
// ============================================================================

export function WorkspaceTab() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selectedWorkspace, setSelectedWorkspace] = useState<Workspace | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [files, setFiles] = useState<FileRef[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Dialogs
  const [showNewWorkspace, setShowNewWorkspace] = useState(false);
  const [showNewProject, setShowNewProject] = useState(false);
  const [showLinkUrl, setShowLinkUrl] = useState(false);
  const [newName, setNewName] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [linkUrl, setLinkUrl] = useState('');

  // Load workspaces on mount
  useEffect(() => {
    loadWorkspaces();
  }, []);

  const executePipeline = async (pipelineId: number, input: any) => {
    if (window.ozone?.pipeline?.execute) {
      return await window.ozone.pipeline.execute(pipelineId, input);
    }
    throw new Error('Pipeline execution not available');
  };

  const loadWorkspaces = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await executePipeline(6, { action: 'ListWorkspaces' });
      if (result?.workspaces) {
        setWorkspaces(result.workspaces);
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load workspaces');
    }
    setLoading(false);
  };

  const loadProjects = async (workspaceId: number) => {
    setLoading(true);
    try {
      const result = await executePipeline(6, { 
        action: 'ListProjects', 
        workspace_id: workspaceId 
      });
      if (result?.projects) {
        setProjects(result.projects);
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load projects');
    }
    setLoading(false);
  };

  const loadProjectFiles = async (projectId: number) => {
    try {
      const result = await executePipeline(6, { 
        action: 'GetProjectFiles', 
        project_id: projectId 
      });
      if (result?.files) {
        setFiles(result.files);
      }
    } catch (e: any) {
      console.warn('Failed to load project files:', e);
    }
  };

  const handleCreateWorkspace = async () => {
    if (!newName.trim()) return;
    setLoading(true);
    try {
      await executePipeline(6, { 
        action: 'CreateWorkspace', 
        name: newName.trim(),
        description: newDescription.trim() || null
      });
      setNewName('');
      setNewDescription('');
      setShowNewWorkspace(false);
      await loadWorkspaces();
    } catch (e: any) {
      setError(e.message || 'Failed to create workspace');
    }
    setLoading(false);
  };

  const handleCreateProject = async () => {
    if (!newName.trim() || !selectedWorkspace) return;
    setLoading(true);
    try {
      await executePipeline(6, { 
        action: 'CreateProject', 
        workspace_id: selectedWorkspace.workspace_id,
        name: newName.trim(),
        description: newDescription.trim() || null
      });
      setNewName('');
      setNewDescription('');
      setShowNewProject(false);
      await loadProjects(selectedWorkspace.workspace_id);
    } catch (e: any) {
      setError(e.message || 'Failed to create project');
    }
    setLoading(false);
  };

  const handleLinkFile = async () => {
    if (!selectedProject) return;
    try {
      const filePath = await window.ozone?.system?.selectFile?.();
      if (filePath) {
        // Call FileLinkPipeline (#30)
        await executePipeline(30, {
          action: 'Link',
          project_id: selectedProject.project_id,
          file_path: filePath,
          analyze: true
        });
        await loadProjectFiles(selectedProject.project_id);
        // Refresh project to update counts
        await loadProjects(selectedWorkspace!.workspace_id);
      }
    } catch (e: any) {
      setError(e.message || 'Failed to link file');
    }
  };

  const handleLinkUrl = async () => {
    if (!selectedProject || !linkUrl.trim()) return;
    try {
      // Call URLLinkPipeline (#31)
      await executePipeline(31, {
        action: 'Link',
        project_id: selectedProject.project_id,
        url: linkUrl.trim(),
        analyze: true
      });
      setLinkUrl('');
      setShowLinkUrl(false);
      await loadProjectFiles(selectedProject.project_id);
      await loadProjects(selectedWorkspace!.workspace_id);
    } catch (e: any) {
      setError(e.message || 'Failed to link URL');
    }
  };

  const handleSelectWorkspace = (ws: Workspace) => {
    setSelectedWorkspace(ws);
    setSelectedProject(null);
    setFiles([]);
    loadProjects(ws.workspace_id);
  };

  const handleSelectProject = (proj: Project) => {
    setSelectedProject(proj);
    loadProjectFiles(proj.project_id);
  };

  const handleBack = () => {
    if (selectedProject) {
      setSelectedProject(null);
      setFiles([]);
    } else if (selectedWorkspace) {
      setSelectedWorkspace(null);
      setProjects([]);
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  // Error display
  if (error) {
    return (
      <div className="workspace-tab">
        <div className="panel-error">
          <span className="error-icon">⚠️</span>
          <p>{error}</p>
          <button onClick={() => { setError(null); loadWorkspaces(); }}>Retry</button>
        </div>
      </div>
    );
  }

  // PROJECT VIEW - Show project details and linked files
  if (selectedProject) {
    return (
      <div className="workspace-tab">
        <div className="panel-toolbar">
          <button className="back-btn" onClick={handleBack}>
            ← Back to Projects
          </button>
          <h2>{selectedProject.name}</h2>
          <div className="toolbar-actions">
            <button className="toolbar-btn" onClick={handleLinkFile}>
              📎 Link File
            </button>
            <button className="toolbar-btn" onClick={() => setShowLinkUrl(true)}>
              🔗 Link URL
            </button>
          </div>
        </div>

        {selectedProject.description && (
          <div className="project-description">
            <p>{selectedProject.description}</p>
          </div>
        )}

        <div className="project-stats">
          <span className="stat">📎 {selectedProject.file_count} files</span>
          <span className="stat">🔗 {selectedProject.url_count} URLs</span>
          <span className="stat">📦 {selectedProject.package_count} packages</span>
        </div>

        {/* Project-scoped Chat Section */}
        <ProjectChat 
          projectId={selectedProject.project_id} 
          workspaceId={selectedWorkspace?.workspace_id}
          projectName={selectedProject.name}
        />

        {files.length === 0 ? (
          <div className="panel-empty-centered">
            <span className="empty-icon">📂</span>
            <p>No linked content yet</p>
            <p className="empty-hint">Link files or URLs to build this project's context</p>
          </div>
        ) : (
          <div className="files-list">
            <h3>Linked Content</h3>
            {files.map((file) => (
              <div key={file.file_ref_id} className="file-item">
                <span className="file-icon">
                  {file.modality === 'Code' ? '📄' : file.modality === 'Text' ? '📝' : '📁'}
                </span>
                <div className="file-info">
                  <span className="file-name">{file.name}</span>
                  <span className="file-path">{file.path}</span>
                </div>
                <span className="file-size">
                  {(file.size_bytes / 1024).toFixed(1)} KB
                </span>
              </div>
            ))}
          </div>
        )}

        {/* Link URL Dialog */}
        {showLinkUrl && (
          <div className="dialog-overlay" onClick={() => setShowLinkUrl(false)}>
            <div className="dialog" onClick={e => e.stopPropagation()}>
              <h3>Link URL</h3>
              <p className="dialog-hint">Add a URL to this project's context.</p>
              <input
                type="url"
                placeholder="https://..."
                value={linkUrl}
                onChange={(e) => setLinkUrl(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleLinkUrl()}
                autoFocus
              />
              <div className="dialog-buttons">
                <button className="btn-cancel" onClick={() => setShowLinkUrl(false)}>Cancel</button>
                <button className="btn-confirm" onClick={handleLinkUrl} disabled={!linkUrl.trim()}>Link</button>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  }

  // PROJECTS VIEW - Show projects in selected workspace
  if (selectedWorkspace) {
    return (
      <div className="workspace-tab">
        <div className="panel-toolbar">
          <button className="back-btn" onClick={handleBack}>
            ← Back to Workspaces
          </button>
          <h2>{selectedWorkspace.name}</h2>
          <button className="toolbar-btn primary" onClick={() => setShowNewProject(true)}>
            + New Project
          </button>
        </div>

        {selectedWorkspace.description && (
          <div className="workspace-description">
            <p>{selectedWorkspace.description}</p>
          </div>
        )}

        {loading ? (
          <div className="panel-loading">
            <div className="loading-spinner" />
            <p>Loading projects...</p>
          </div>
        ) : projects.length === 0 ? (
          <div className="panel-empty-centered">
            <span className="empty-icon">📂</span>
            <p>No projects in this workspace</p>
            <p className="empty-hint">Create a project to start working with scoped context</p>
            <button className="empty-action-btn" onClick={() => setShowNewProject(true)}>
              Create Project
            </button>
          </div>
        ) : (
          <div className="items-grid">
            {projects.map((proj) => (
              <div 
                key={proj.project_id} 
                className="item-card"
                onClick={() => handleSelectProject(proj)}
              >
                <div className="card-icon">📂</div>
                <div className="card-content">
                  <h3>{proj.name}</h3>
                  {proj.description && <p className="card-desc">{proj.description}</p>}
                  <div className="card-meta">
                    <span>📎 {proj.file_count}</span>
                    <span>🔗 {proj.url_count}</span>
                    <span>📦 {proj.package_count}</span>
                  </div>
                </div>
                <span className="card-arrow">→</span>
              </div>
            ))}
          </div>
        )}

        {/* New Project Dialog */}
        {showNewProject && (
          <div className="dialog-overlay" onClick={() => setShowNewProject(false)}>
            <div className="dialog" onClick={e => e.stopPropagation()}>
              <h3>Create New Project</h3>
              <p className="dialog-hint">Projects contain linked files/URLs and have their own chat scope.</p>
              <input
                type="text"
                placeholder="Project name (e.g., API Integration)"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                autoFocus
              />
              <input
                type="text"
                placeholder="Description (optional)"
                value={newDescription}
                onChange={(e) => setNewDescription(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleCreateProject()}
              />
              <div className="dialog-buttons">
                <button className="btn-cancel" onClick={() => setShowNewProject(false)}>Cancel</button>
                <button className="btn-confirm" onClick={handleCreateProject} disabled={!newName.trim() || loading}>
                  {loading ? 'Creating...' : 'Create'}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  }

  // WORKSPACES VIEW - List all workspaces
  return (
    <div className="workspace-tab">
      <div className="panel-toolbar">
        <h2>Workspaces</h2>
        <button className="toolbar-btn primary" onClick={() => setShowNewWorkspace(true)}>
          + New Workspace
        </button>
      </div>

      <div className="info-banner">
        <span className="info-icon">💡</span>
        <p>
          A <strong>Workspace</strong> organizes related projects (e.g., "Coding", "Research"). 
          Each workspace can contain multiple <strong>Projects</strong> with their own linked files and chat scope.
        </p>
      </div>

      {loading ? (
        <div className="panel-loading">
          <div className="loading-spinner" />
          <p>Loading workspaces...</p>
        </div>
      ) : workspaces.length === 0 ? (
        <div className="panel-empty-centered">
          <span className="empty-icon">📁</span>
          <p>No workspaces yet</p>
          <p className="empty-hint">Create one to organize your projects</p>
          <button className="empty-action-btn" onClick={() => setShowNewWorkspace(true)}>
            Create Workspace
          </button>
        </div>
      ) : (
        <div className="items-grid">
          {workspaces.map((ws) => (
            <div 
              key={ws.workspace_id} 
              className="item-card"
              onClick={() => handleSelectWorkspace(ws)}
            >
              <div className="card-icon">📁</div>
              <div className="card-content">
                <h3>{ws.name}</h3>
                {ws.description && <p className="card-desc">{ws.description}</p>}
                <div className="card-meta">
                  <span>{ws.project_count} projects</span>
                  <span>•</span>
                  <span>{formatDate(ws.updated_at)}</span>
                </div>
              </div>
              <span className="card-arrow">→</span>
            </div>
          ))}
        </div>
      )}

      {/* New Workspace Dialog */}
      {showNewWorkspace && (
        <div className="dialog-overlay" onClick={() => setShowNewWorkspace(false)}>
          <div className="dialog" onClick={e => e.stopPropagation()}>
            <h3>Create New Workspace</h3>
            <p className="dialog-hint">Workspaces organize related projects together.</p>
            <input
              type="text"
              placeholder="Workspace name (e.g., Web Development)"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              autoFocus
            />
            <input
              type="text"
              placeholder="Description (optional)"
              value={newDescription}
              onChange={(e) => setNewDescription(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleCreateWorkspace()}
            />
            <div className="dialog-buttons">
              <button className="btn-cancel" onClick={() => setShowNewWorkspace(false)}>Cancel</button>
              <button className="btn-confirm" onClick={handleCreateWorkspace} disabled={!newName.trim() || loading}>
                {loading ? 'Creating...' : 'Create'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default WorkspaceTab;

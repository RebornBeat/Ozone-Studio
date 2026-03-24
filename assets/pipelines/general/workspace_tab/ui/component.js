/**
 * WorkspaceTab UI Component (Pipeline #6)
 * 
 * Manages workspaces and projects. This is a CORE TAB - never uninjected.
 * 
 * Features:
 * - Workspace creation and management
 * - Project creation within workspaces
 * - File/URL linking via FileLink (#30) and URLLink (#31) pipelines
 * - Project-scoped chat with context aggregation
 * 
 * FileLink, URLLink, PackageLink are called FROM this tab's UI,
 * they don't need their own separate tabs.
 */

module.exports = {
  meta: {
    title: 'Workspace',
    icon: '📁',
    version: '0.4.0',
  },
  
  render: function(container, props, React, ReactDOM) {
    const { useState, useEffect, useCallback, useRef } = React;
    const { executePipeline, getSharedState, setSharedState } = props;
    
    // ========================================================================
    // ProjectChat Component - Scoped chat within a project
    // ========================================================================
    function ProjectChat({ projectId, workspaceId, projectName }) {
      const [messages, setMessages] = useState([]);
      const [input, setInput] = useState('');
      const [loading, setLoading] = useState(false);
      const messagesRef = useRef(null);
      const inputRef = useRef(null);

      // Auto-scroll on new messages
      useEffect(() => {
        if (messagesRef.current) {
          messagesRef.current.scrollTop = messagesRef.current.scrollHeight;
        }
      }, [messages]);

      const handleSubmit = async (e) => {
        if (e) e.preventDefault();
        if (!input.trim() || loading) return;

        const userMessage = {
          id: Date.now(),
          role: 'user',
          content: input,
          timestamp: Date.now(),
        };
        setMessages(prev => [...prev, userMessage]);
        setInput('');
        setLoading(true);

        try {
          // Use orchestrator with project context if available
          if (window.ozone?.orchestrate) {
            const result = await window.ozone.orchestrate({
              prompt: input,
              project_id: projectId,
              workspace_id: workspaceId,
              user_id: 1,
              device_id: 1,
              consciousness_enabled: window.ozone?.config?.consciousness_enabled ?? false,
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
                content: 'Error: ' + result.error,
                timestamp: Date.now(),
              }]);
            }
          } else {
            // Fallback: aggregate context and call prompt directly
            let aggregatedContext = '';
            
            // First get project context via ContextAggregation (#21)
            try {
              const contextResult = await executePipeline(21, {
                action: 'ForProject',
                project_id: projectId,
                token_budget: 50000,
              });
              aggregatedContext = contextResult?.context?.context_text || '';
            } catch (err) {
              console.warn('Failed to aggregate context:', err);
            }

            // Then call prompt pipeline (#9)
            const promptResult = await executePipeline(9, {
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
        } catch (err) {
          setMessages(prev => [...prev, {
            id: Date.now(),
            role: 'assistant',
            content: 'Error: ' + (err.message || 'Failed to process request'),
            timestamp: Date.now(),
          }]);
        } finally {
          setLoading(false);
        }
      };

      const handleKeyDown = (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
          e.preventDefault();
          handleSubmit();
        }
      };

      const formatTime = (timestamp) => {
        return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      };

      return React.createElement('div', { className: 'project-chat' },
        React.createElement('div', { className: 'chat-header' },
          React.createElement('span', { className: 'chat-icon' }, '💬'),
          React.createElement('span', { className: 'chat-title' }, 'Chat with ' + projectName),
          React.createElement('span', { className: 'chat-scope' }, 'Context: ' + projectName + ' files & URLs')
        ),

        React.createElement('div', { className: 'chat-messages', ref: messagesRef },
          messages.length === 0 ? (
            React.createElement('div', { className: 'chat-empty' },
              React.createElement('span', { className: 'empty-icon' }, '✨'),
              React.createElement('p', null, 'Start chatting within this project\'s context'),
              React.createElement('p', { className: 'empty-hint' }, 'Your questions will use linked files and URLs as context')
            )
          ) : (
            messages.map(msg =>
              React.createElement('div', { key: msg.id, className: 'chat-message ' + msg.role },
                React.createElement('div', { className: 'message-header' },
                  React.createElement('span', { className: 'message-author' }, msg.role === 'user' ? 'You' : 'OZONE'),
                  React.createElement('span', { className: 'message-time' }, formatTime(msg.timestamp))
                ),
                React.createElement('div', { className: 'message-content' }, msg.content)
              )
            )
          ),
          loading && React.createElement('div', { className: 'chat-message assistant loading' },
            React.createElement('div', { className: 'message-content' },
              React.createElement('span', { className: 'loading-dots' }, 'Processing')
            )
          )
        ),

        React.createElement('form', { className: 'chat-input-form', onSubmit: handleSubmit },
          React.createElement('textarea', {
            ref: inputRef,
            value: input,
            onChange: (e) => setInput(e.target.value),
            onKeyDown: handleKeyDown,
            placeholder: 'Ask about ' + projectName + '...',
            rows: 1,
            disabled: loading
          }),
          React.createElement('button', { 
            type: 'submit', 
            disabled: !input.trim() || loading 
          }, loading ? '...' : '→')
        )
      );
    }
    
    // ========================================================================
    // Main WorkspaceUI Component
    // ========================================================================
    function WorkspaceUI() {
      const [workspaces, setWorkspaces] = useState([]);
      const [selectedWorkspace, setSelectedWorkspace] = useState(null);
      const [projects, setProjects] = useState([]);
      const [selectedProject, setSelectedProject] = useState(null);
      const [files, setFiles] = useState([]);
      const [loading, setLoading] = useState(true);
      const [error, setError] = useState(null);
      
      // Dialog states
      const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
      const [showCreateProject, setShowCreateProject] = useState(false);
      const [showLinkUrl, setShowLinkUrl] = useState(false);
      const [newName, setNewName] = useState('');
      const [newDescription, setNewDescription] = useState('');
      const [linkUrl, setLinkUrl] = useState('');
      
      // Load workspaces on mount
      useEffect(() => {
        loadWorkspaces();
      }, []);
      
      const loadWorkspaces = async () => {
        setLoading(true);
        setError(null);
        try {
          const result = await executePipeline(6, { action: 'ListWorkspaces' });
          setWorkspaces(result?.workspaces || []);
          
          // Restore selection from shared state
          const state = getSharedState();
          if (state.selectedWorkspaceId) {
            const ws = result?.workspaces?.find(w => 
              w.id === state.selectedWorkspaceId || w.workspace_id === state.selectedWorkspaceId
            );
            if (ws) {
              setSelectedWorkspace(ws);
              loadProjects(ws.id || ws.workspace_id);
            }
          }
        } catch (e) {
          console.error('Failed to load workspaces:', e);
          setError(e.message || 'Failed to load workspaces');
        } finally {
          setLoading(false);
        }
      };
      
      const loadProjects = async (workspaceId) => {
        try {
          const result = await executePipeline(6, { 
            action: 'ListProjects', 
            workspace_id: workspaceId 
          });
          setProjects(result?.projects || []);
        } catch (e) {
          console.error('Failed to load projects:', e);
          setError(e.message || 'Failed to load projects');
        }
      };
      
      const loadProjectFiles = async (projectId) => {
        try {
          const result = await executePipeline(6, { 
            action: 'GetProjectFiles', 
            project_id: projectId 
          });
          setFiles(result?.files || []);
        } catch (e) {
          console.warn('Failed to load project files:', e);
        }
      };
      
      const handleSelectWorkspace = (workspace) => {
        const wsId = workspace.id || workspace.workspace_id;
        setSelectedWorkspace(workspace);
        setSharedState({ selectedWorkspaceId: wsId });
        loadProjects(wsId);
        setSelectedProject(null);
        setFiles([]);
      };
      
      const handleSelectProject = (project) => {
        const projId = project.id || project.project_id;
        setSelectedProject(project);
        setSharedState({ selectedProjectId: projId });
        loadProjectFiles(projId);
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
      
      const handleCreateWorkspace = async () => {
        if (!newName.trim()) return;
        setLoading(true);
        try {
          await executePipeline(6, {
            action: 'CreateWorkspace',
            name: newName.trim(),
            description: newDescription.trim() || null,
          });
          setShowCreateWorkspace(false);
          setNewName('');
          setNewDescription('');
          await loadWorkspaces();
        } catch (e) {
          console.error('Failed to create workspace:', e);
          setError(e.message || 'Failed to create workspace');
        } finally {
          setLoading(false);
        }
      };
      
      const handleCreateProject = async () => {
        if (!newName.trim() || !selectedWorkspace) return;
        const wsId = selectedWorkspace.id || selectedWorkspace.workspace_id;
        setLoading(true);
        try {
          await executePipeline(6, {
            action: 'CreateProject',
            workspace_id: wsId,
            name: newName.trim(),
            description: newDescription.trim() || null,
          });
          setShowCreateProject(false);
          setNewName('');
          setNewDescription('');
          await loadProjects(wsId);
        } catch (e) {
          console.error('Failed to create project:', e);
          setError(e.message || 'Failed to create project');
        } finally {
          setLoading(false);
        }
      };
      
      // Link file to project (calls FileLink pipeline #30)
      const handleLinkFile = async () => {
        if (!selectedProject) return;
        const projId = selectedProject.id || selectedProject.project_id;
        const path = await window.ozone?.system?.selectFile?.();
        if (path) {
          try {
            await executePipeline(30, {
              action: 'Link',
              project_id: projId,
              file_path: path,
              analyze: true,
            });
            await loadProjectFiles(projId);
            // Refresh projects to update counts
            const wsId = selectedWorkspace.id || selectedWorkspace.workspace_id;
            await loadProjects(wsId);
          } catch (e) {
            console.error('Failed to link file:', e);
            setError(e.message || 'Failed to link file');
          }
        }
      };
      
      // Link URL to project (calls URLLink pipeline #31)
      const handleLinkURL = async () => {
        if (!selectedProject || !linkUrl.trim()) return;
        const projId = selectedProject.id || selectedProject.project_id;
        try {
          await executePipeline(31, {
            action: 'Link',
            project_id: projId,
            url: linkUrl.trim(),
            analyze: true,
          });
          setLinkUrl('');
          setShowLinkUrl(false);
          await loadProjectFiles(projId);
          // Refresh projects to update counts
          const wsId = selectedWorkspace.id || selectedWorkspace.workspace_id;
          await loadProjects(wsId);
        } catch (e) {
          console.error('Failed to link URL:', e);
          setError(e.message || 'Failed to link URL');
        }
      };
      
      const formatDate = (timestamp) => {
        return new Date(timestamp * 1000).toLocaleDateString();
      };
      
      const getFileIcon = (modality) => {
        switch (modality) {
          case 'Code': return '📄';
          case 'Text': return '📝';
          case 'Image': return '🖼️';
          case 'Audio': return '🎵';
          case 'Video': return '🎬';
          default: return '📁';
        }
      };
      
      // Error display
      if (error) {
        return React.createElement('div', { className: 'workspace-tab' },
          React.createElement('div', { className: 'panel-error' },
            React.createElement('span', { className: 'error-icon' }, '⚠️'),
            React.createElement('p', null, error),
            React.createElement('button', { 
              onClick: () => { setError(null); loadWorkspaces(); } 
            }, 'Retry')
          )
        );
      }
      
      // Loading state
      if (loading && workspaces.length === 0) {
        return React.createElement('div', { className: 'workspace-loading' },
          React.createElement('div', { className: 'loading-spinner' }),
          React.createElement('p', null, 'Loading workspaces...')
        );
      }
      
      // PROJECT VIEW - Show project details, linked files, and chat
      if (selectedProject) {
        const projId = selectedProject.id || selectedProject.project_id;
        const wsId = selectedWorkspace?.id || selectedWorkspace?.workspace_id;
        const fileCount = selectedProject.file_count ?? selectedProject.files_count ?? 0;
        const urlCount = selectedProject.url_count ?? 0;
        const packageCount = selectedProject.package_count ?? 0;
        
        return React.createElement('div', { className: 'workspace-tab' },
          React.createElement('div', { className: 'panel-toolbar' },
            React.createElement('button', { className: 'back-btn', onClick: handleBack },
              '← Back to Projects'
            ),
            React.createElement('h2', null, selectedProject.name),
            React.createElement('div', { className: 'toolbar-actions' },
              React.createElement('button', { className: 'toolbar-btn', onClick: handleLinkFile },
                '📎 Link File'
              ),
              React.createElement('button', { className: 'toolbar-btn', onClick: () => setShowLinkUrl(true) },
                '🔗 Link URL'
              )
            )
          ),
          
          selectedProject.description && React.createElement('div', { className: 'project-description' },
            React.createElement('p', null, selectedProject.description)
          ),
          
          React.createElement('div', { className: 'project-stats' },
            React.createElement('span', { className: 'stat' }, '📎 ' + fileCount + ' files'),
            React.createElement('span', { className: 'stat' }, '🔗 ' + urlCount + ' URLs'),
            React.createElement('span', { className: 'stat' }, '📦 ' + packageCount + ' packages')
          ),
          
          // Project-scoped Chat Section
          React.createElement(ProjectChat, {
            projectId: projId,
            workspaceId: wsId,
            projectName: selectedProject.name
          }),
          
          // Linked Content Section
          files.length === 0 ? (
            React.createElement('div', { className: 'panel-empty-centered' },
              React.createElement('span', { className: 'empty-icon' }, '📂'),
              React.createElement('p', null, 'No linked content yet'),
              React.createElement('p', { className: 'empty-hint' }, 'Link files or URLs to build this project\'s context')
            )
          ) : (
            React.createElement('div', { className: 'files-list' },
              React.createElement('h3', null, 'Linked Content'),
              files.map((file) =>
                React.createElement('div', { 
                  key: file.file_ref_id || file.id, 
                  className: 'file-item' 
                },
                  React.createElement('span', { className: 'file-icon' }, 
                    getFileIcon(file.modality)
                  ),
                  React.createElement('div', { className: 'file-info' },
                    React.createElement('span', { className: 'file-name' }, file.name),
                    React.createElement('span', { className: 'file-path' }, file.path)
                  ),
                  React.createElement('span', { className: 'file-size' },
                    ((file.size_bytes || 0) / 1024).toFixed(1) + ' KB'
                  )
                )
              )
            )
          ),
          
          // Link URL Dialog
          showLinkUrl && React.createElement('div', { 
            className: 'modal-overlay',
            onClick: () => setShowLinkUrl(false)
          },
            React.createElement('div', { 
              className: 'modal',
              onClick: (e) => e.stopPropagation()
            },
              React.createElement('h3', null, 'Link URL'),
              React.createElement('p', { className: 'hint' }, 
                'Add a URL to this project\'s context.'
              ),
              React.createElement('input', {
                type: 'url',
                placeholder: 'https://...',
                value: linkUrl,
                onChange: (e) => setLinkUrl(e.target.value),
                onKeyDown: (e) => e.key === 'Enter' && handleLinkURL(),
                autoFocus: true
              }),
              React.createElement('div', { className: 'modal-actions' },
                React.createElement('button', { 
                  onClick: () => setShowLinkUrl(false) 
                }, 'Cancel'),
                React.createElement('button', { 
                  className: 'btn-primary',
                  onClick: handleLinkURL,
                  disabled: !linkUrl.trim()
                }, 'Link')
              )
            )
          )
        );
      }
      
      // PROJECTS VIEW - Show projects in selected workspace
      if (selectedWorkspace) {
        const wsId = selectedWorkspace.id || selectedWorkspace.workspace_id;
        
        return React.createElement('div', { className: 'workspace-tab' },
          React.createElement('div', { className: 'panel-toolbar' },
            React.createElement('button', { className: 'back-btn', onClick: handleBack },
              '← Back to Workspaces'
            ),
            React.createElement('h2', null, selectedWorkspace.name),
            React.createElement('button', { 
              className: 'toolbar-btn primary',
              onClick: () => setShowCreateProject(true)
            }, '+ New Project')
          ),
          
          selectedWorkspace.description && React.createElement('div', { className: 'workspace-description' },
            React.createElement('p', null, selectedWorkspace.description)
          ),
          
          loading ? (
            React.createElement('div', { className: 'panel-loading' },
              React.createElement('div', { className: 'loading-spinner' }),
              React.createElement('p', null, 'Loading projects...')
            )
          ) : projects.length === 0 ? (
            React.createElement('div', { className: 'panel-empty-centered' },
              React.createElement('span', { className: 'empty-icon' }, '📂'),
              React.createElement('p', null, 'No projects in this workspace'),
              React.createElement('p', { className: 'empty-hint' }, 
                'Create a project to start working with scoped context'
              ),
              React.createElement('button', { 
                className: 'btn-primary',
                onClick: () => setShowCreateProject(true)
              }, 'Create Project')
            )
          ) : (
            React.createElement('div', { className: 'items-grid' },
              projects.map((proj) => {
                const fileCount = proj.file_count ?? proj.files_count ?? 0;
                const urlCount = proj.url_count ?? 0;
                const packageCount = proj.package_count ?? 0;
                
                return React.createElement('div', {
                  key: proj.id || proj.project_id,
                  className: 'item-card',
                  onClick: () => handleSelectProject(proj)
                },
                  React.createElement('div', { className: 'card-icon' }, '📂'),
                  React.createElement('div', { className: 'card-content' },
                    React.createElement('h3', null, proj.name),
                    proj.description && React.createElement('p', { className: 'card-desc' }, proj.description),
                    React.createElement('div', { className: 'card-meta' },
                      React.createElement('span', null, '📎 ' + fileCount),
                      React.createElement('span', null, '🔗 ' + urlCount),
                      React.createElement('span', null, '📦 ' + packageCount)
                    )
                  ),
                  React.createElement('span', { className: 'card-arrow' }, '→')
                );
              })
            )
          ),
          
          // Create project modal
          showCreateProject && React.createElement('div', { 
            className: 'modal-overlay',
            onClick: () => setShowCreateProject(false)
          },
            React.createElement('div', { 
              className: 'modal',
              onClick: (e) => e.stopPropagation()
            },
              React.createElement('h3', null, 'Create New Project'),
              React.createElement('p', { className: 'hint' },
                'Projects contain linked files/URLs and have their own chat scope.'
              ),
              React.createElement('input', {
                type: 'text',
                placeholder: 'Project name (e.g., API Integration)',
                value: newName,
                onChange: (e) => setNewName(e.target.value),
                autoFocus: true
              }),
              React.createElement('textarea', {
                placeholder: 'Description (optional)',
                value: newDescription,
                onChange: (e) => setNewDescription(e.target.value)
              }),
              React.createElement('div', { className: 'modal-actions' },
                React.createElement('button', { 
                  onClick: () => setShowCreateProject(false) 
                }, 'Cancel'),
                React.createElement('button', { 
                  className: 'btn-primary',
                  onClick: handleCreateProject,
                  disabled: !newName.trim() || loading
                }, loading ? 'Creating...' : 'Create')
              )
            )
          )
        );
      }
      
      // No workspaces - show create prompt centered
      if (workspaces.length === 0) {
        return React.createElement('div', { className: 'workspace-empty' },
          React.createElement('div', { className: 'empty-state-centered' },
            React.createElement('span', { className: 'empty-icon' }, '📁'),
            React.createElement('h3', null, 'No workspaces yet'),
            React.createElement('p', null, 'Create one to organize your projects'),
            React.createElement('p', { className: 'hint' }, 
              'A workspace groups related projects together - like "Coding", "Research", or "Personal".'
            ),
            React.createElement('button', { 
              className: 'btn-primary',
              onClick: () => setShowCreateWorkspace(true)
            }, '+ Create Workspace')
          )
        );
      }
      
      // WORKSPACES VIEW - List all workspaces (with sidebar)
      return React.createElement('div', { className: 'workspace-panel' },
        // Sidebar: Workspace list
        React.createElement('aside', { className: 'workspace-sidebar' },
          React.createElement('div', { className: 'sidebar-header' },
            React.createElement('h3', null, 'Workspaces'),
            React.createElement('button', { 
              className: 'btn-icon',
              onClick: () => setShowCreateWorkspace(true),
              title: 'Create Workspace'
            }, '+')
          ),
          React.createElement('div', { className: 'workspace-list' },
            workspaces.map(ws => {
              const wsId = ws.id || ws.workspace_id;
              const selectedId = selectedWorkspace?.id || selectedWorkspace?.workspace_id;
              return React.createElement('div', {
                key: wsId,
                className: 'workspace-item' + (selectedId === wsId ? ' selected' : ''),
                onClick: () => handleSelectWorkspace(ws)
              },
                React.createElement('span', { className: 'ws-icon' }, '📁'),
                React.createElement('span', { className: 'ws-name' }, ws.name)
              );
            })
          )
        ),
        
        // Main content - info banner when no workspace selected
        React.createElement('main', { className: 'workspace-content' },
          React.createElement('div', { className: 'info-banner' },
            React.createElement('span', { className: 'info-icon' }, '💡'),
            React.createElement('p', null,
              'A ', React.createElement('strong', null, 'Workspace'),
              ' organizes related projects (e.g., "Coding", "Research"). ',
              'Each workspace can contain multiple ',
              React.createElement('strong', null, 'Projects'),
              ' with their own linked files and chat scope.'
            )
          ),
          React.createElement('div', { className: 'empty-state-centered' },
            React.createElement('p', null, 'Select a workspace to view projects')
          )
        ),
        
        // Create workspace modal
        showCreateWorkspace && React.createElement('div', { 
          className: 'modal-overlay',
          onClick: () => setShowCreateWorkspace(false)
        },
          React.createElement('div', { 
            className: 'modal',
            onClick: (e) => e.stopPropagation()
          },
            React.createElement('h3', null, 'Create Workspace'),
            React.createElement('p', { className: 'hint' },
              'Workspaces help organize related projects. Examples: "Development", "Research", "Personal".'
            ),
            React.createElement('input', {
              type: 'text',
              placeholder: 'Workspace name',
              value: newName,
              onChange: (e) => setNewName(e.target.value),
              autoFocus: true
            }),
            React.createElement('textarea', {
              placeholder: 'Description (optional)',
              value: newDescription,
              onChange: (e) => setNewDescription(e.target.value)
            }),
            React.createElement('div', { className: 'modal-actions' },
              React.createElement('button', { onClick: () => setShowCreateWorkspace(false) }, 'Cancel'),
              React.createElement('button', { 
                className: 'btn-primary',
                onClick: handleCreateWorkspace,
                disabled: !newName.trim() || loading
              }, loading ? 'Creating...' : 'Create')
            )
          )
        )
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(WorkspaceUI));
    
    return () => root.unmount();
  },
  
  onActivate: function() {
    console.log('WorkspaceTab activated');
  },
  
  onDeactivate: function() {
    console.log('WorkspaceTab deactivated');
  },
};

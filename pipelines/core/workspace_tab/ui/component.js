/**
 * WorkspaceTab UI Component (Pipeline #6)
 * 
 * Manages workspaces and projects. This is a CORE TAB - never uninjected.
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
    const { useState, useEffect, useCallback } = React;
    const { executePipeline, getSharedState, setSharedState } = props;
    
    function WorkspaceUI() {
      const [workspaces, setWorkspaces] = useState([]);
      const [selectedWorkspace, setSelectedWorkspace] = useState(null);
      const [projects, setProjects] = useState([]);
      const [selectedProject, setSelectedProject] = useState(null);
      const [loading, setLoading] = useState(true);
      const [showCreateWorkspace, setShowCreateWorkspace] = useState(false);
      const [showCreateProject, setShowCreateProject] = useState(false);
      const [newName, setNewName] = useState('');
      const [newDescription, setNewDescription] = useState('');
      
      // Load workspaces on mount
      useEffect(() => {
        loadWorkspaces();
      }, []);
      
      const loadWorkspaces = async () => {
        setLoading(true);
        try {
          const result = await executePipeline(6, { action: 'ListWorkspaces' });
          setWorkspaces(result?.workspaces || []);
          
          // Restore selection from shared state
          const state = getSharedState();
          if (state.selectedWorkspaceId) {
            const ws = result?.workspaces?.find(w => w.id === state.selectedWorkspaceId);
            if (ws) {
              setSelectedWorkspace(ws);
              loadProjects(ws.id);
            }
          }
        } catch (e) {
          console.error('Failed to load workspaces:', e);
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
        }
      };
      
      const handleSelectWorkspace = (workspace) => {
        setSelectedWorkspace(workspace);
        setSharedState({ selectedWorkspaceId: workspace.id });
        loadProjects(workspace.id);
        setSelectedProject(null);
      };
      
      const handleSelectProject = (project) => {
        setSelectedProject(project);
        setSharedState({ selectedProjectId: project.id });
      };
      
      const handleCreateWorkspace = async () => {
        if (!newName.trim()) return;
        try {
          await executePipeline(6, {
            action: 'CreateWorkspace',
            name: newName,
            description: newDescription,
          });
          setShowCreateWorkspace(false);
          setNewName('');
          setNewDescription('');
          loadWorkspaces();
        } catch (e) {
          console.error('Failed to create workspace:', e);
        }
      };
      
      const handleCreateProject = async () => {
        if (!newName.trim() || !selectedWorkspace) return;
        try {
          await executePipeline(6, {
            action: 'CreateProject',
            workspace_id: selectedWorkspace.id,
            name: newName,
            description: newDescription,
          });
          setShowCreateProject(false);
          setNewName('');
          setNewDescription('');
          loadProjects(selectedWorkspace.id);
        } catch (e) {
          console.error('Failed to create project:', e);
        }
      };
      
      // Link file to project (calls FileLink pipeline #30)
      const handleLinkFile = async () => {
        if (!selectedProject) return;
        const path = await window.ozone?.system?.selectFile?.();
        if (path) {
          try {
            await executePipeline(30, {
              action: 'Link',
              project_id: selectedProject.id,
              file_path: path,
              analyze: true,
            });
            loadProjects(selectedWorkspace.id);
          } catch (e) {
            console.error('Failed to link file:', e);
          }
        }
      };
      
      // Link URL to project (calls URLLink pipeline #31)
      const handleLinkURL = async () => {
        if (!selectedProject) return;
        const url = prompt('Enter URL to link:');
        if (url) {
          try {
            await executePipeline(31, {
              action: 'Link',
              project_id: selectedProject.id,
              url: url,
              analyze: true,
            });
            loadProjects(selectedWorkspace.id);
          } catch (e) {
            console.error('Failed to link URL:', e);
          }
        }
      };
      
      if (loading) {
        return React.createElement('div', { className: 'workspace-loading' },
          React.createElement('div', { className: 'loading-spinner' }),
          React.createElement('p', null, 'Loading workspaces...')
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
            workspaces.map(ws => 
              React.createElement('div', {
                key: ws.id,
                className: `workspace-item ${selectedWorkspace?.id === ws.id ? 'selected' : ''}`,
                onClick: () => handleSelectWorkspace(ws)
              },
                React.createElement('span', { className: 'ws-icon' }, '📁'),
                React.createElement('span', { className: 'ws-name' }, ws.name)
              )
            )
          )
        ),
        
        // Main content
        React.createElement('main', { className: 'workspace-content' },
          selectedWorkspace ? (
            React.createElement('div', { className: 'projects-view' },
              React.createElement('div', { className: 'projects-header' },
                React.createElement('h2', null, selectedWorkspace.name),
                React.createElement('button', {
                  className: 'btn-secondary',
                  onClick: () => setShowCreateProject(true)
                }, '+ New Project')
              ),
              
              projects.length === 0 ? (
                React.createElement('div', { className: 'empty-state-centered' },
                  React.createElement('span', { className: 'empty-icon' }, '📋'),
                  React.createElement('p', null, 'No projects in this workspace'),
                  React.createElement('button', {
                    className: 'btn-primary',
                    onClick: () => setShowCreateProject(true)
                  }, '+ Create Project')
                )
              ) : (
                React.createElement('div', { className: 'projects-grid' },
                  projects.map(proj =>
                    React.createElement('div', {
                      key: proj.id,
                      className: `project-card ${selectedProject?.id === proj.id ? 'selected' : ''}`,
                      onClick: () => handleSelectProject(proj)
                    },
                      React.createElement('h4', null, proj.name),
                      React.createElement('p', null, proj.description || 'No description'),
                      proj.files_count !== undefined && 
                        React.createElement('span', { className: 'files-badge' }, 
                          `${proj.files_count} files`
                        )
                    )
                  )
                )
              ),
              
              // Project actions when a project is selected
              selectedProject && (
                React.createElement('div', { className: 'project-actions' },
                  React.createElement('h4', null, `Actions for: ${selectedProject.name}`),
                  React.createElement('div', { className: 'action-buttons' },
                    React.createElement('button', { onClick: handleLinkFile }, '📎 Link File'),
                    React.createElement('button', { onClick: handleLinkURL }, '🔗 Link URL'),
                    React.createElement('button', { disabled: true }, '📦 Link Package')
                  ),
                  React.createElement('p', { className: 'hint' },
                    'Chat with this project is scoped to its linked context.'
                  )
                )
              )
            )
          ) : (
            React.createElement('div', { className: 'empty-state-centered' },
              React.createElement('p', null, 'Select a workspace to view projects')
            )
          )
        ),
        
        // Create workspace modal
        showCreateWorkspace && React.createElement('div', { className: 'modal-overlay' },
          React.createElement('div', { className: 'modal' },
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
                disabled: !newName.trim()
              }, 'Create')
            )
          )
        ),
        
        // Create project modal
        showCreateProject && React.createElement('div', { className: 'modal-overlay' },
          React.createElement('div', { className: 'modal' },
            React.createElement('h3', null, 'Create Project'),
            React.createElement('p', { className: 'hint' },
              'Projects contain files, URLs, and packages that provide context for your conversations.'
            ),
            React.createElement('input', {
              type: 'text',
              placeholder: 'Project name',
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
              React.createElement('button', { onClick: () => setShowCreateProject(false) }, 'Cancel'),
              React.createElement('button', {
                className: 'btn-primary', 
                onClick: handleCreateProject,
                disabled: !newName.trim()
              }, 'Create')
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

/**
 * FileLinkPipeline (#30) UI Component
 * 
 * This JS file is loaded dynamically at runtime - NO REBUILD NEEDED!
 * 
 * The render function receives:
 * - container: DOM element to render into
 * - props: { pipelineId, executePipeline, getSharedState, setSharedState, subscribeToState, onClose, initialData }
 * - React: The React library
 * - ReactDOM: The ReactDOM library
 * 
 * Return a cleanup function if needed.
 */

module.exports = {
  // Optional metadata
  meta: {
    title: 'Link File',
    icon: '📎',
    version: '0.4.0',
  },
  
  // Main render function
  render: function(container, props, React, ReactDOM) {
    const { useState, useCallback } = React;
    const { executePipeline, onClose, initialData } = props;
    
    // The actual component
    function FileLinkUI() {
      const [filePath, setFilePath] = useState('');
      const [analyze, setAnalyze] = useState(true);
      const [loading, setLoading] = useState(false);
      const [error, setError] = useState(null);
      const [success, setSuccess] = useState(false);
      
      const handleBrowse = useCallback(async () => {
        if (window.ozone?.system?.selectFile) {
          const path = await window.ozone.system.selectFile();
          if (path) setFilePath(path);
        }
      }, []);
      
      const handleSubmit = useCallback(async (e) => {
        e.preventDefault();
        if (!filePath) return;
        
        setLoading(true);
        setError(null);
        
        try {
          await executePipeline(30, {
            action: 'Link',
            file_path: filePath,
            analyze: analyze,
            project_id: initialData?.projectId,
          });
          setSuccess(true);
          setTimeout(() => onClose?.(), 1500);
        } catch (err) {
          setError(err.message || 'Failed to link file');
        } finally {
          setLoading(false);
        }
      }, [filePath, analyze, executePipeline, initialData, onClose]);
      
      if (success) {
        return React.createElement('div', { className: 'file-link-success' },
          React.createElement('span', null, '✓'),
          React.createElement('p', null, 'File linked successfully!')
        );
      }
      
      return React.createElement('div', { className: 'file-link-ui' },
        React.createElement('h3', null, 'Link File to Project'),
        React.createElement('p', { className: 'description' }, 
          'Add a local file to your project\'s context. The file will be analyzed and indexed.'
        ),
        
        error && React.createElement('div', { className: 'error-message' }, error),
        
        React.createElement('form', { onSubmit: handleSubmit },
          React.createElement('div', { className: 'form-field' },
            React.createElement('label', null, 'File Path'),
            React.createElement('div', { className: 'file-input-group' },
              React.createElement('input', {
                type: 'text',
                value: filePath,
                onChange: (e) => setFilePath(e.target.value),
                placeholder: 'Select a file...',
                readOnly: true,
              }),
              React.createElement('button', { type: 'button', onClick: handleBrowse }, 'Browse')
            )
          ),
          
          React.createElement('div', { className: 'form-field checkbox-field' },
            React.createElement('label', null,
              React.createElement('input', {
                type: 'checkbox',
                checked: analyze,
                onChange: (e) => setAnalyze(e.target.checked),
              }),
              'Analyze file content'
            )
          ),
          
          React.createElement('div', { className: 'form-actions' },
            onClose && React.createElement('button', { type: 'button', onClick: onClose, disabled: loading }, 'Cancel'),
            React.createElement('button', { type: 'submit', disabled: loading || !filePath },
              loading ? 'Linking...' : 'Link File'
            )
          )
        )
      );
    }
    
    // Render the component
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(FileLinkUI));
    
    // Return cleanup function
    return () => {
      root.unmount();
    };
  },
  
  // Called when tab becomes active
  onActivate: function() {
    console.log('FileLinkUI activated');
  },
  
  // Called when tab becomes inactive
  onDeactivate: function() {
    console.log('FileLinkUI deactivated');
  },
};

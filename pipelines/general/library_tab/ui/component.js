/**
 * LibraryTab UI Component (Pipeline #7)
 * 
 * Browse pipelines, methodologies, and blueprints. This is a CORE TAB.
 * Provides access to the distributed library of reusable components.
 */

module.exports = {
  meta: {
    title: 'Library',
    icon: '📚',
    version: '0.4.0',
  },
  
  render: function(container, props, React, ReactDOM) {
    const { useState, useEffect } = React;
    const { executePipeline } = props;
    
    function LibraryUI() {
      const [activeSection, setActiveSection] = useState('pipelines');
      const [items, setItems] = useState([]);
      const [searchQuery, setSearchQuery] = useState('');
      const [loading, setLoading] = useState(true);
      const [selectedItem, setSelectedItem] = useState(null);
      
      useEffect(() => {
        loadItems();
      }, [activeSection]);
      
      const loadItems = async () => {
        setLoading(true);
        try {
          let result;
          switch (activeSection) {
            case 'pipelines':
              result = await executePipeline(7, { action: 'ListPipelines' });
              setItems(result?.pipelines || []);
              break;
            case 'methodologies':
              result = await executePipeline(11, { action: 'List' });
              setItems(result?.methodologies || []);
              break;
            case 'blueprints':
              result = await executePipeline(13, { action: 'Search', query: '' });
              setItems(result?.blueprints || []);
              break;
          }
        } catch (e) {
          console.error('Failed to load items:', e);
          setItems([]);
        } finally {
          setLoading(false);
        }
      };
      
      const handleSearch = async () => {
        if (!searchQuery.trim()) {
          loadItems();
          return;
        }
        setLoading(true);
        try {
          let result;
          switch (activeSection) {
            case 'pipelines':
              result = await executePipeline(7, { 
                action: 'SearchPipelines', 
                query: searchQuery 
              });
              setItems(result?.pipelines || []);
              break;
            case 'methodologies':
              result = await executePipeline(11, { 
                action: 'Search', 
                query: searchQuery 
              });
              setItems(result?.methodologies || []);
              break;
            case 'blueprints':
              result = await executePipeline(13, { 
                action: 'Search', 
                query: searchQuery 
              });
              setItems(result?.blueprints || []);
              break;
          }
        } catch (e) {
          console.error('Search failed:', e);
        } finally {
          setLoading(false);
        }
      };
      
      const getSectionIcon = (section) => {
        switch (section) {
          case 'pipelines': return '🔧';
          case 'methodologies': return '📖';
          case 'blueprints': return '📋';
          default: return '📄';
        }
      };
      
      return React.createElement('div', { className: 'library-panel' },
        // Section tabs
        React.createElement('nav', { className: 'library-nav' },
          ['pipelines', 'methodologies', 'blueprints'].map(section =>
            React.createElement('button', {
              key: section,
              className: `nav-btn ${activeSection === section ? 'active' : ''}`,
              onClick: () => {
                setActiveSection(section);
                setSelectedItem(null);
                setSearchQuery('');
              }
            },
              React.createElement('span', { className: 'nav-icon' }, getSectionIcon(section)),
              React.createElement('span', { className: 'nav-label' }, 
                section.charAt(0).toUpperCase() + section.slice(1)
              )
            )
          )
        ),
        
        // Search bar
        React.createElement('div', { className: 'library-search' },
          React.createElement('input', {
            type: 'text',
            placeholder: `Search ${activeSection}...`,
            value: searchQuery,
            onChange: (e) => setSearchQuery(e.target.value),
            onKeyDown: (e) => e.key === 'Enter' && handleSearch()
          }),
          React.createElement('button', { onClick: handleSearch }, '🔍')
        ),
        
        // Content
        React.createElement('div', { className: 'library-content' },
          // Items list
          React.createElement('div', { className: 'items-list' },
            loading ? (
              React.createElement('div', { className: 'loading-state' },
                React.createElement('div', { className: 'loading-spinner' }),
                React.createElement('p', null, 'Loading...')
              )
            ) : items.length === 0 ? (
              React.createElement('div', { className: 'empty-state-centered' },
                React.createElement('span', { className: 'empty-icon' }, getSectionIcon(activeSection)),
                React.createElement('p', null, `No ${activeSection} found`),
                activeSection === 'pipelines' && (
                  React.createElement('p', { className: 'hint' }, 
                    'The system includes 54 built-in pipelines.'
                  )
                )
              )
            ) : (
              items.map((item, idx) =>
                React.createElement('div', {
                  key: item.id || idx,
                  className: `item-card ${selectedItem?.id === item.id ? 'selected' : ''}`,
                  onClick: () => setSelectedItem(item)
                },
                  React.createElement('h4', null, item.name),
                  React.createElement('p', null, item.description || 'No description'),
                  item.author && (
                    React.createElement('span', { className: 'item-author' }, 
                      `by ${item.author}`
                    )
                  )
                )
              )
            )
          ),
          
          // Item details
          React.createElement('div', { className: 'item-details' },
            selectedItem ? (
              React.createElement('div', { className: 'details-content' },
                React.createElement('h3', null, selectedItem.name),
                React.createElement('p', { className: 'description' }, 
                  selectedItem.description || 'No description'
                ),
                selectedItem.author && (
                  React.createElement('div', { className: 'meta-row' },
                    React.createElement('span', { className: 'meta-label' }, 'Author:'),
                    React.createElement('span', { className: 'meta-value' }, selectedItem.author)
                  )
                ),
                selectedItem.version && (
                  React.createElement('div', { className: 'meta-row' },
                    React.createElement('span', { className: 'meta-label' }, 'Version:'),
                    React.createElement('span', { className: 'meta-value' }, selectedItem.version)
                  )
                ),
                activeSection === 'pipelines' && (
                  React.createElement('div', { className: 'detail-actions' },
                    React.createElement('button', { 
                      className: 'btn-secondary',
                      disabled: true 
                    }, 'View Code'),
                    React.createElement('button', { 
                      className: 'btn-primary',
                      disabled: true
                    }, 'Execute')
                  )
                )
              )
            ) : (
              React.createElement('div', { className: 'empty-state-centered' },
                React.createElement('p', null, `Select a ${activeSection.slice(0, -1)} to view details`)
              )
            )
          )
        )
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(LibraryUI));
    
    return () => root.unmount();
  },
};

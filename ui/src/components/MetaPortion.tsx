/**
 * MetaPortion Component - §5.2 of the specification
 * 
 * Always accessible area containing:
 * - Prompt input
 * - Voice controls
 * - Task list
 * - Home button
 * 
 * This component does NOT replace pipeline functionality -
 * it triggers pipelines (Prompt, Voice, TaskManager, HomeReturn)
 */

import React, { useState } from 'react';
import { useOzoneStore } from '../services/store';

interface MetaPortionProps {
  width: number; // Percentage width
}

export function MetaPortion({ width }: MetaPortionProps) {
  const {
    promptInput,
    setPromptInput,
    submitPrompt,
    activeTasks,
    activeTab,
    setActiveTab,
    selectedModel,
    setSelectedModel,
    availableModels,
    executePipeline,
  } = useOzoneStore();
  
  const [voiceActive, setVoiceActive] = useState(false);

  // Handle prompt submission
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    submitPrompt();
  };

  // Handle keyboard shortcuts
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submitPrompt();
    }
  };

  // Toggle voice input (triggers VoicePipeline)
  const toggleVoice = async () => {
    if (!voiceActive) {
      // Start voice recording
      try {
        await executePipeline(10, { action: 'start' }); // VoicePipeline
        setVoiceActive(true);
      } catch (err) {
        console.error('Voice start failed:', err);
      }
    } else {
      // Stop voice recording
      try {
        await executePipeline(10, { action: 'stop' });
        setVoiceActive(false);
      } catch (err) {
        console.error('Voice stop failed:', err);
      }
    }
  };

  // Return home (triggers HomeReturnPipeline)
  const returnHome = async () => {
    try {
      await executePipeline(35, {}); // HomeReturnPipeline
    } catch (err) {
      console.error('Home return failed:', err);
    }
  };

  return (
    <aside className="meta-portion" style={{ width: `${width}%` }}>
      {/* Home Button */}
      <div className="meta-section meta-home">
        <button onClick={returnHome} className="home-button" title="Return to Home">
          <span className="icon">🏠</span>
          <span>Home</span>
        </button>
      </div>

      {/* Navigation Tabs */}
      <nav className="meta-section meta-nav">
        <button
          className={`nav-tab ${activeTab === 'workspace' ? 'active' : ''}`}
          onClick={() => setActiveTab('workspace')}
        >
          Workspace
        </button>
        <button
          className={`nav-tab ${activeTab === 'library' ? 'active' : ''}`}
          onClick={() => setActiveTab('library')}
        >
          Library
        </button>
        <button
          className={`nav-tab ${activeTab === 'settings' ? 'active' : ''}`}
          onClick={() => setActiveTab('settings')}
        >
          Settings
        </button>
      </nav>

      {/* Active Tasks */}
      <div className="meta-section meta-tasks">
        <h3>Active Tasks</h3>
        {activeTasks.length === 0 ? (
          <p className="no-tasks">No active tasks</p>
        ) : (
          <ul className="task-list">
            {activeTasks.map((task) => (
              <li key={task.id} className={`task-item task-${task.status}`}>
                <span className="task-id">#{task.id}</span>
                <span className="task-status">{task.status}</span>
                {task.status === 'running' && (
                  <div className="task-progress">
                    <div
                      className="task-progress-bar"
                      style={{ width: `${task.progress}%` }}
                    />
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Model Selection */}
      <div className="meta-section meta-model">
        <label htmlFor="model-select">Model:</label>
        <select
          id="model-select"
          value={selectedModel}
          onChange={(e) => setSelectedModel(e.target.value)}
        >
          {availableModels.map((model) => (
            <option key={model.identifier} value={model.identifier}>
              {model.name}
            </option>
          ))}
        </select>
      </div>

      {/* Prompt Input */}
      <div className="meta-section meta-prompt">
        <form onSubmit={handleSubmit}>
          <textarea
            value={promptInput}
            onChange={(e) => setPromptInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Enter your prompt..."
            rows={4}
          />
          <div className="prompt-actions">
            <button
              type="button"
              className={`voice-button ${voiceActive ? 'active' : ''}`}
              onClick={toggleVoice}
              title={voiceActive ? 'Stop recording' : 'Start voice input'}
            >
              🎤
            </button>
            <button type="submit" className="submit-button">
              Send
            </button>
          </div>
        </form>
      </div>
    </aside>
  );
}

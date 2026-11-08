/**
 * ProjectChatPanel.tsx (2026-10-18, rebuilt 2026-10-18) — a standalone,
 * project-scoped chat surface. Mounted as a real React sibling by
 * pipeline-ui.tsx's DynamicPipelineUI next to the Workspace tab's legacy
 * container, scoped to pipelineId===6, when a project is selected (see
 * that file and workspace_tab/ui/component.js's handleSelectProject).
 *
 * Deliberately NOT a modification of MetaPortion.tsx — that global sidebar
 * stays fully untouched, always-visible, non-interfering. This is a
 * second, independent chat instance with its own transcript (useChatSession),
 * sharing only the rendering engine (chatRendering.tsx) MetaPortion also uses.
 */

import React, { useEffect, useRef, useState } from 'react';
import { modelStyle, renderMarkdown } from '../chatRendering';
import TranscriptEntryMeta from './TranscriptEntryMeta';
import ThinkingRow from './ThinkingRow';
import PausedBanner from './PausedBanner';
import { OrchestrationPanels, CoordinationStrip } from './OrchestrationPanels';
import { useChatSession } from '../hooks/useChatSession';
import { useOzoneStore } from '../services/store';

export interface ProjectChatPanelProps {
  projectId: number;
  workspaceId?: number;
  projectName: string;
}

export default function ProjectChatPanel({ projectId, workspaceId, projectName }: ProjectChatPanelProps) {
  // Same engine as MetaPortion; only the scope differs (project-scoped,
  // persisted per project+workspace across restarts). The CONTROL SELECTIONS
  // (the model selector) are wired to the SAME store state as MetaPortion —
  // both chats share one selection, captured via the config round-trip.
  const selectedModel = useOzoneStore(st => st.selectedModel);
  const setSelectedModel = useOzoneStore(st => st.setSelectedModel);
  const availableModels = useOzoneStore(st => st.availableModels);
  const modelConfig = selectedModel ? { model_identifier: selectedModel } : undefined;
  const { transcript, isRunning, send, elapsedSeconds, stepInfo, awaiting, paused, resumePaused } = useChatSession({ projectId, workspaceId, modelConfig });
  const [input, setInput] = useState('');
  const transcriptRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    transcriptRef.current?.scrollTo({ top: transcriptRef.current.scrollHeight });
  }, [transcript]);

  const handleSubmit = (e?: React.FormEvent) => {
    e?.preventDefault();
    const text = input;
    if (!text.trim() || isRunning) return;
    setInput('');
    send(text);
  };

  return (
    <div className="project-chat-panel">
      <div className="project-chat-header">
        <span>💬 Chat with {projectName}</span>
        {/* CONTROL SELECTIONS (the operator: captured and selectable in BOTH
           chats): the model selector shares the store state with MetaPortion. */}
        <select
          className="project-chat-model-select"
          value={selectedModel}
          onChange={(e) => setSelectedModel(e.target.value)}
          title="Model for this project's runs (shared with the meta panel)"
          style={{ maxWidth: '45%', fontSize: 11, padding: '2px 6px', borderRadius: 6 }}
        >
          {availableModels.map((m) => (
            <option key={m.identifier} value={m.identifier}>{m.identifier}</option>
          ))}
        </select>
      </div>
      <div className="project-chat-transcript" ref={transcriptRef}>
        {transcript.length === 0 ? (
          <div className="empty-state">
            <span>✨</span>
            <p>
              <span className="empty-text">Start chatting within this project's context</span>
            </p>
            <p className="hint">Your questions will use linked files and URLs as context</p>
          </div>
        ) : (
          transcript.map(entry => {
            const style = entry.role === 'assistant' ? modelStyle(entry.modelUsed) : null;
            return (
              <div key={entry.id} style={{ marginBottom: 10 }}>
                <div style={{ fontSize: 11, opacity: 0.7, marginBottom: 2 }}>
                  {entry.role === 'user' ? 'You' : (
                    <span style={{ color: style?.color }}>{style?.icon} Assistant</span>
                  )}
                </div>
                <div>{renderMarkdown(entry.content)}</div>
                <TranscriptEntryMeta entry={entry} />
              </div>
            );
          })
        )}
        <ThinkingRow isRunning={isRunning} elapsedSeconds={elapsedSeconds} stepInfo={stepInfo} awaiting={awaiting} />
        <PausedBanner paused={paused} onResume={resumePaused} busy={isRunning} />
        <CoordinationStrip />
      </div>
      <OrchestrationPanels isRunning={isRunning} scope={{ projectId, workspaceId }} />
      <form className="project-chat-input" onSubmit={handleSubmit}>
        <textarea
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              handleSubmit();
            }
          }}
          placeholder={`Ask about ${projectName}...`}
          disabled={isRunning}
        />
        <button type="submit" disabled={isRunning || !input.trim()}>→</button>
      </form>
    </div>
  );
}

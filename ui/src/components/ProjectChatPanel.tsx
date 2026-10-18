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
import { useChatSession } from '../hooks/useChatSession';

export interface ProjectChatPanelProps {
  projectId: number;
  workspaceId?: number;
  projectName: string;
}

export default function ProjectChatPanel({ projectId, workspaceId, projectName }: ProjectChatPanelProps) {
  const { transcript, isRunning, send } = useChatSession({ projectId, workspaceId });
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
              </div>
            );
          })
        )}
      </div>
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

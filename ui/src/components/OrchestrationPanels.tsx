/**
 * OrchestrationPanels.tsx — the live-run indicator stack, extracted from
 * MetaPortion.tsx (2026-11-01) so the global chat and the project chat mount
 * the identical set: stage checklist, discovery toasts, MCP activity. The
 * coordination/capture notification strip is exported separately
 * (CoordinationStrip) because it sits inside the scrollable transcript, not
 * above the input.
 *
 * `scope` is carried for the chat that owns the panels. OrchestrationStatusPanel,
 * DiscoveryEffects and McpActivityIndicator currently subscribe to the global
 * event streams and show only while THIS chat's `isRunning` is true, so a chat
 * that is idle shows nothing; scope-keyword filtering inside those components
 * is wired for OrchestrationStatusPanel (stage frames carry project_id /
 * workspace_id). DiscoveryEffects and McpActivityIndicator consume streams
 * whose frames carry no project scope, so they remain unfiltered — two chats
 * running at once can still see each other's discovery/MCP activity.
 */

import React from 'react';
import OrchestrationStatusPanel from './OrchestrationStatusPanel';
import DiscoveryEffects from './DiscoveryEffects';
import McpActivityIndicator from './McpActivityIndicator';
import ChatNotificationStream from '../views/coordination/ChatNotifications';

export interface OrchestrationScope {
  projectId?: number;
  workspaceId?: number;
}

export interface OrchestrationPanelsProps {
  isRunning: boolean;
  scope?: OrchestrationScope;
}

export function OrchestrationPanels({ isRunning, scope }: OrchestrationPanelsProps) {
  return (
    <>
      <OrchestrationStatusPanel isRunning={isRunning} scope={scope ? { projectId: scope.projectId ?? null, workspaceId: scope.workspaceId } : undefined} />
      <DiscoveryEffects isRunning={isRunning} />
      <McpActivityIndicator isRunning={isRunning} />
    </>
  );
}

export function CoordinationStrip() {
  return <ChatNotificationStream />;
}

export default OrchestrationPanels;

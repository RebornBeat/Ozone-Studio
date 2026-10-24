/**
 * ProjectWorkspaceViewer.tsx (2026-10-18) — an interactive, PROJECT-SCOPED
 * inline viewer for the Workspace tab's project view. Replaces what used to
 * be a bare "No linked content yet" placeholder with real content: the
 * same Files/Fabric/Hierarchy capability the global tabs show, but bound to
 * this one project and embedded right here — no navigating away.
 *
 * Deliberately reuses the real, already-working inner view components
 * (CodeViewer/FileReferenceViewer/WorkspaceBrowser/FabricView/AmtTreeView/
 * ForkLineageView/JurisdictionHierarchy) directly by their `projectId` prop
 * — these are the exact same components the global Files/Fabric/Hierarchy
 * tabs render inside PanelShell, just without PanelShell's own chrome (its
 * title bar and global ProjectPicker, which would be redundant here: the
 * project is already fixed by the Workspace tab, not picked from a list).
 *
 * Graph (W7.1): GraphView.tsx now accepts an optional `projectId` prop
 * (undefined when absent keeps its original self-fetching/dropdown
 * behavior unchanged for the existing global Graph tab) — embedded here
 * the same way as Fabric, no sub-tabs.
 *
 * Two id spaces (2026-10-18, cc-fork-unify-fe): `projectId` (this file's
 * own prop) is the Workspace tab's FLAT internal id (local counter,
 * unrelated to the graph) — it stays the one `handleUnlinkFile` uses for
 * the real FileLink pipeline calls (GetProjectFiles #6 / Unlink #30, both
 * flat-id-based). `zseiProjectId` is the real ZSEI graph container id
 * (backend fix, same session: `zsei_container_id` now present on every
 * Workspace/Project object) — it's what every graph-native viewer below
 * actually needs for its own `projectId` prop (yes, literally named
 * `projectId` in THEIR code too — that prop name means "a real container
 * id" there, not "the Workspace tab's flat id"; passing `zseiProjectId`'s
 * VALUE into it is not a rename). Can be null (container creation failed,
 * not yet self-healed) — every one of these components already declares
 * its prop `number | null` and renders a real "nothing selected" state
 * for null, confirmed by reading each before this change, so null is
 * passed through as-is, never substituted with the flat id.
 */

import React, { useState } from 'react';
import { WorkspaceBrowser } from '../views/files/WorkspaceBrowser';
import { FileReferenceViewer } from '../views/files/FileReferenceViewer';
import { CodeViewer } from '../views/files/CodeViewer';
import { FabricView } from '../views/fabric/FabricView';
import { AmtTreeView } from '../views/hierarchy/AmtTreeView';
import { ForkLineageView } from '../views/hierarchy/ForkLineageView';
import { JurisdictionHierarchy } from '../views/hierarchy/JurisdictionHierarchy';
import { GraphView } from './GraphView';
import { ENGINES } from '../panels/EnginesPanel';
import { WorkspaceStatusStrip } from './WorkspaceStatusStrip';
import { useOzoneStore } from '../services/store';

export interface ProjectWorkspaceViewerProps {
  // Flat Workspace-tab id — used ONLY by this component's own
  // handleUnlinkFile (GetProjectFiles #6 / Unlink #30). Never passed to
  // any of the graph-native viewers below.
  projectId: number;
  // Real ZSEI graph container id — used for every graph-native viewer's
  // own `projectId` prop (WorkspaceBrowser/FileReferenceViewer/CodeViewer/
  // FabricView/AmtTreeView/ForkLineageView/JurisdictionHierarchy/
  // GraphView). May be null; those components already handle that.
  zseiProjectId: number | null;
}

type Section = 'files' | 'fabric' | 'hierarchy' | 'graph' | 'engines';
type FilesSub = 'browser' | 'refs' | 'code';
type HierarchySub = 'amt' | 'lineage' | 'jurisdiction';
// Mirrors EnginesPanel.tsx's own registry (same ids) so a project-scoped
// click (e.g. StepDetailSection's ModalityChip jump target) and the global
// Engines tab always land on the same sub-tab id for the same modality.
type EngineSub = typeof ENGINES[number]['id'];

const SECTIONS: { id: Section; label: string }[] = [
  { id: 'files', label: '📁 Files' },
  { id: 'fabric', label: '🧵 Fabric' },
  { id: 'hierarchy', label: '🌳 Hierarchy' },
  { id: 'graph', label: '🕸 Graph' },
  { id: 'engines', label: '⚙️ Engines' },
];

const FILES_SUBS: { id: FilesSub; label: string }[] = [
  { id: 'browser', label: 'Browser' },
  { id: 'refs', label: 'References' },
  { id: 'code', label: 'Code' },
];

const HIERARCHY_SUBS: { id: HierarchySub; label: string }[] = [
  { id: 'amt', label: 'AMT tree' },
  { id: 'lineage', label: 'Fork lineage' },
  { id: 'jurisdiction', label: 'Jurisdiction' },
];

const ENGINE_SUBS: { id: EngineSub; label: string }[] = ENGINES.map(e => ({ id: e.id, label: e.label }));

export default function ProjectWorkspaceViewer({ projectId, zseiProjectId }: ProjectWorkspaceViewerProps) {
  const [section, setSection] = useState<Section>('files');
  const [filesSub, setFilesSub] = useState<FilesSub>('browser');
  const [hierarchySub, setHierarchySub] = useState<HierarchySub>('amt');
  const [enginesSub, setEnginesSub] = useState<EngineSub>(ENGINES[0].id);
  // Live orchestration activity for THIS project (2026-10-18) — real data
  // from an orchestration_stage WS event whose request carried this exact
  // flat projectId (ThemeArea.tsx's subscriber writes it to the store;
  // see store.ts's liveEngineActivity doc comment). Deliberately does NOT
  // change `section`/`filesSub`/`hierarchySub` — surfacing a small, inline
  // indicator here is enough; forcing a tab switch while the operator is
  // mid-read of something else would be the same disruptive mistake
  // ThemeArea.tsx's notifyTabNeedsAttention already avoids for the global
  // case. Naturally self-clears when a DIFFERENT project's activity
  // arrives (the store only ever holds the latest one, and the equality
  // check below stops matching).
  const liveActivity = useOzoneStore((s) => s.liveEngineActivity);
  const myActivity = liveActivity && liveActivity.projectId === projectId ? liveActivity : null;
  // W7.2 — bumped after a successful Unlink so WorkspaceBrowser's Browser
  // tab re-fetches just its file-refs list (not the whole workspaces/
  // projects sidebar tree — see its `refreshToken` prop).
  const [browserRefreshToken, setBrowserRefreshToken] = useState(0);

  const subs = section === 'files' ? FILES_SUBS : section === 'hierarchy' ? HIERARCHY_SUBS : section === 'engines' ? ENGINE_SUBS : null;
  const activeSub = section === 'files' ? filesSub : section === 'hierarchy' ? hierarchySub : section === 'engines' ? enginesSub : null;
  const setSub = section === 'files' ? (id: string) => setFilesSub(id as FilesSub)
    : section === 'hierarchy' ? (id: string) => setHierarchySub(id as HierarchySub)
    : section === 'engines' ? (id: string) => setEnginesSub(id as EngineSub)
    : () => {};
  const activeEngine = ENGINES.find(e => e.id === enginesSub) ?? ENGINES[0];

  // W7.2 — real unlink, called from WorkspaceBrowser's "Remove link"
  // button. WorkspaceBrowser's rows are graph-native FileReference/
  // URLReference/PackageReference containers (`fileRef.containerId` is the
  // ZSEI container id); the FileLink pipeline's real Unlink action (#30)
  // takes the FLAT file_link store's `file_ref_id` instead — a different
  // id space (confirmed against assets/pipelines/general/file_link/main.rs:
  // `Unlink { project_id, file_ref_id }` looks records up by `r.id`, and
  // `workspace_tab`'s own `GetProjectFiles` returns that same flat
  // FileRefInfo-shaped list, each with both `id` and the `zsei_container_id`
  // it was linked to). So: fetch the flat list, resolve this row's real
  // `file_ref_id` by matching `zsei_container_id === fileRef.containerId`,
  // and only then call Unlink — never call it with a guessed/undefined id.
  async function handleUnlinkFile(fileRef: any) {
    const containerId = fileRef?.containerId;
    const execute = window.ozone?.pipeline?.execute;
    if (containerId == null || !execute) return;
    try {
      const flatResult = await execute(6, {
        action: 'GetProjectFiles',
        project_id: projectId,
      });
      const flatFiles: any[] = (flatResult?.output as any)?.files || [];
      const match = flatFiles.find((f) => f?.zsei_container_id === containerId);
      if (!match) {
        console.error(
          `Unlink failed: no flat file_link record found for container ${containerId} ` +
            '(it may predate the zsei_container_id wiring, or the two stores have drifted).',
        );
        return;
      }
      await execute(30, {
        action: 'Unlink',
        project_id: projectId,
        file_ref_id: match.id,
      });
      setBrowserRefreshToken((k) => k + 1);
    } catch (err) {
      console.error('Unlink failed:', err);
    }
  }

  // W7.2 — real open-navigation, called from WorkspaceBrowser's row click
  // (in addition to its own setSelectedFile, which already primes Code/
  // References to show this file). Mirrors the destination logic the old
  // component.js `openFile()` cross-tab-jumped to (Code view for Code
  // modality, else the container/reference view) — but as an in-place
  // sub-tab switch, not a jump away, consistent with this viewer's own
  // "no navigating away" design (see file header).
  function handleOpenFile(fileRef: any) {
    setSection('files');
    setFilesSub(fileRef?.modality === 'Code' ? 'code' : 'refs');
  }

  return (
    // ORDER (2026-10-18, operator-reported, repeated across several
    // rounds): the active section's content renders FIRST, the tab bar
    // (Files/Fabric/Hierarchy/Graph + sub-tabs) LAST — directly above
    // ProjectChatPanel below this viewer, so the tab bar is the thing
    // sitting right on top of the chat title, not the content.
    <div className="project-insights-viewer">
      {/* WORKSPACE STATUS STRIP (wave 7, additive): live P2P trust posture
          (connected ≠ trusted) — indicators from the kit, real values only. */}
      <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 4 }}>
        <WorkspaceStatusStrip />
      </div>
      {myActivity && (
        <div className="project-live-activity" key={myActivity.timestamp}>
          <span className="project-live-activity-dot" />
          <span className="project-live-activity-stage">{myActivity.stageName}</span>
          <span className="project-live-activity-summary">{myActivity.summary}</span>
        </div>
      )}
      <div className="project-insights-content">
        {section === 'files' && filesSub === 'browser' && (
          <WorkspaceBrowser
            projectId={zseiProjectId}
            onUnlink={handleUnlinkFile}
            onOpenFile={handleOpenFile}
            refreshToken={browserRefreshToken}
            embedded
          />
        )}
        {section === 'files' && filesSub === 'refs' && <FileReferenceViewer projectId={zseiProjectId} />}
        {section === 'files' && filesSub === 'code' && <CodeViewer projectId={zseiProjectId} />}
        {section === 'fabric' && <FabricView projectId={zseiProjectId} />}
        {section === 'hierarchy' && hierarchySub === 'amt' && <AmtTreeView projectId={zseiProjectId} />}
        {section === 'hierarchy' && hierarchySub === 'lineage' && <ForkLineageView projectId={zseiProjectId} />}
        {section === 'hierarchy' && hierarchySub === 'jurisdiction' && <JurisdictionHierarchy projectId={zseiProjectId} />}
        {section === 'graph' && <GraphView projectId={zseiProjectId} />}
        {section === 'engines' && activeEngine.render(zseiProjectId)}
      </div>
      <div className="project-insights-tabs">
        {SECTIONS.map(s => (
          <button
            key={s.id}
            className={section === s.id ? 'active' : ''}
            onClick={() => setSection(s.id)}
          >
            {s.label}
          </button>
        ))}
        {subs && (
          <>
            <span className="project-insights-tab-sep" />
            {subs.map(s => (
              <button
                key={s.id}
                className={activeSub === s.id ? 'active' : ''}
                onClick={() => setSub(s.id)}
              >
                {s.label}
              </button>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

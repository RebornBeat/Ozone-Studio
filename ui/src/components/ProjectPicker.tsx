import React, { useEffect, useState } from "react";
import { loadProjectOptions, ProjectOption, useSelectedProject } from "../projectSelection";

/** Shared real workspace → project picker for every Context-Viewer-family panel. */
export const ProjectPicker: React.FC = () => {
  const [selected, setSelected] = useSelectedProject();
  const [options, setOptions] = useState<ProjectOption[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    loadProjectOptions()
      .then((o) => {
        if (cancelled) return;
        setOptions(o);
        if (o.length > 0 && (selected === null || !o.some((p) => p.id === selected))) setSelected(o[0].id);
      })
      .catch((e) => !cancelled && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (error) return <span style={{ color: "#ff8a8a", fontSize: 12.5 }}>Error loading projects: {error}</span>;
  if (options.length === 0) return <span style={{ color: "var(--color-text-muted)", fontSize: 12.5 }}>No workspaces/projects found yet.</span>;
  return (
    <span style={{ display: "inline-flex", gap: 8, alignItems: "center", fontSize: 12.5 }}>
      <label style={{ color: "var(--color-text-muted)" }}>Project:</label>
      <select
        value={selected ?? undefined}
        onChange={(e) => setSelected(Number(e.target.value))}
        style={{ background: "#101724", color: "var(--color-text)", border: "1px solid var(--color-border-faint)", borderRadius: 6, padding: "4px 8px" }}
      >
        {options.map((p) => (
          <option key={p.id} value={p.id}>
            {p.workspaceName} / {p.name}
          </option>
        ))}
      </select>
    </span>
  );
};

export default ProjectPicker;

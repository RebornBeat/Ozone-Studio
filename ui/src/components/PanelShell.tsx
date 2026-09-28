import React, { useState } from "react";
import ProjectPicker from "./ProjectPicker";
import { useSelectedProject } from "../projectSelection";

export interface PanelTab {
  id: string;
  label: string;
  render: (projectId: number | null) => React.ReactNode;
}

/** Common frame for the Context-Viewer-family panels: title, shared project picker, sub-tabs. Scaffold-owned. */
export const PanelShell: React.FC<{ title: string; subtitle?: string; showProjectPicker?: boolean; tabs: PanelTab[] }> = ({
  title,
  subtitle,
  showProjectPicker = true,
  tabs,
}) => {
  const [projectId] = useSelectedProject();
  const [active, setActive] = useState(tabs[0]?.id);
  const current = tabs.find((t) => t.id === active) ?? tabs[0];
  return (
    <div className="opanel">
      <div className="opanel-head">
        <span className="opanel-title">{title}</span>
      </div>
      {subtitle && <p className="opanel-sub">{subtitle}</p>}
      <div style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap", marginBottom: 10 }}>
        {showProjectPicker && <ProjectPicker />}
        {tabs.length > 1 && (
          <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
            {tabs.map((t) => (
              <button
                key={t.id}
                onClick={() => setActive(t.id)}
                style={{
                  background: t.id === current?.id ? "#1e2836" : "transparent",
                  color: t.id === current?.id ? "#dfe7f2" : "#8b98ab",
                  border: "1px solid #1e2836",
                  borderRadius: 6,
                  padding: "3px 10px",
                  fontSize: 12,
                  cursor: "pointer",
                }}
              >
                {t.label}
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="opanel-scroll" style={{ minHeight: 0 }}>
        {current?.render(projectId)}
      </div>
    </div>
  );
};

export default PanelShell;

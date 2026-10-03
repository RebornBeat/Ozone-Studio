import React from "react";
import type { NodeVisual } from "./types";

/** Draws a NodeVisual at the origin of its parent <g>. Owned by the scaffold;
 * renderer forks only return NodeVisual data, they don't draw. */
export const NodeShapeSvg: React.FC<{ visual: NodeVisual; selected: boolean; hovered: boolean }> = ({
  visual,
  selected,
  hovered,
}) => {
  const r = visual.radius + (selected ? 3 : hovered ? 2 : 0);
  const common = {
    fill: visual.fill,
    opacity: visual.opacity ?? 1,
    stroke: selected ? "#ffffff" : visual.stroke ?? "none",
    strokeWidth: selected ? 2 : visual.strokeWidth ?? 1.5,
    strokeDasharray: selected ? undefined : visual.strokeDasharray,
  };
  let shape: React.ReactNode;
  switch (visual.shape) {
    case "square":
      shape = <rect x={-r} y={-r} width={2 * r} height={2 * r} rx={2} {...common} />;
      break;
    case "diamond":
      shape = <polygon points={`0,${-r} ${r},0 0,${r} ${-r},0`} {...common} />;
      break;
    case "triangle":
      shape = <polygon points={`0,${-r} ${r * 0.87},${r * 0.5} ${-r * 0.87},${r * 0.5}`} {...common} />;
      break;
    case "hexagon": {
      const pts = [0, 1, 2, 3, 4, 5]
        .map((i) => `${r * Math.cos((Math.PI / 3) * i)},${r * Math.sin((Math.PI / 3) * i)}`)
        .join(" ");
      shape = <polygon points={pts} {...common} />;
      break;
    }
    default:
      shape = <circle r={r} {...common} />;
  }
  return (
    <>
      {shape}
      {visual.glyph && (
        <text textAnchor="middle" dominantBaseline="central" fontSize={Math.max(8, r)} fill="var(--color-bg)" style={{ pointerEvents: "none", fontWeight: 700 }}>
          {visual.glyph}
        </text>
      )}
    </>
  );
};

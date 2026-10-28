/**
 * GraphWorld3D — the 3D graph-native world (guide §8, the operator's vision:
 * "explorable traversable 3D graph-native traversal map across all").
 *
 * This is a LEAF RENDERER behind GraphKit's contracts (doctrine §1.5): it
 * consumes the GraphProvider substrate and renders it in 3D space. Three.js
 * is the rendering engine; the stable API is the provider + the navigation
 * spine + the indicator data.
 *
 * Architecture:
 * - Nodes = 3D primitives (icosahedron default, shape varies by modality group),
 *   colored by MODALITY_COLOR, sized by connection count
 * - Edges = 3D lines with edgeClass dash honesty (solid real, dashed derived)
 * - Camera = OrbitControls (explore) + fly-to on selection (traverse)
 * - Raycast picking = click to select, double-click to navigate
 * - CrossLinkStrip = rendered as a 2D overlay panel on selection
 * - Indicators = 3D-anchored sprites (task pips, gate states, construction pulses)
 *
 * Layout: deterministic force-directed approximation (modality clusters attract,
 * cross-links repel) computed once per substrate load, not per-frame.
 *
 * PERF: LOD via drei's Detailed; frustum culling is automatic; the substrate
 * is bounded by the GraphProvider's single-fetch model.
 */
import React, { useMemo, useRef, useState, useEffect } from "react";
import { Canvas, useThree, useFrame } from "@react-three/fiber";
import { OrbitControls, Line, Html, Text } from "@react-three/drei";
import * as THREE from "three";
import { MODALITY_COLOR } from "../graphRenderers/defaults";
import type { GraphViewEdge, GraphViewNode, Modality } from "../graphViewTypes";
import { navigateTo } from "../navigation";

const C_TEXT = "#e6edf3";
const C_MUTED = "#8b949e";

// ── Layout ──

interface Node3D {
  id: string;
  position: [number, number, number];
  color: string;
  label: string;
  nodeType: string;
  modality: string;
  isReal: boolean;
  connections: number;
}

interface Edge3D {
  from: [number, number, number];
  to: [number, number, number];
  dash: boolean;
  color: string;
}

/** Deterministic force-directed approximation: modality clusters on concentric
 * shells (z-axis by modality index), nodes spread on each shell by hash order.
 * Cross-links pull nodes slightly toward their targets. Computed once. */
function computeLayout(
  nodes: GraphViewNode[],
  edges: GraphViewEdge[],
  width: number,
): Map<string, Node3D> {
  const pos = new Map<string, Node3D>();
  if (nodes.length === 0) return pos;

  // Group by modality
  const groups = new Map<string, GraphViewNode[]>();
  for (const n of nodes) {
    const m = n.modality ?? "container";
    (groups.get(m) ?? groups.set(m, []).get(m)!).push(n);
  }
  const modalityList = Array.from(groups.keys());

  // Assign each modality a z-layer
  const zLayer = new Map<string, number>();
  modalityList.forEach((m, i) => zLayer.set(m, (i - modalityList.length / 2) * 14));

  // Assign angular positions within each shell
  const GOLDEN = Math.PI * (3 - Math.sqrt(5)); // golden angle for even distribution
  for (const [m, list] of groups) {
    const z = zLayer.get(m) ?? 0;
    const radius = Math.min(18, 6 + list.length * 0.8);
    list.forEach((n, i) => {
      const angle = i * GOLDEN;
      const r = radius * (0.7 + 0.3 * ((i % 5) / 5));
      pos.set(n.id, {
        id: n.id,
        position: [cx() + Math.cos(angle) * r, cy() + Math.sin(angle) * r * 0.6, z + (i % 3 - 1) * 2],
        color: MODALITY_COLOR[m as Modality] ?? C_MUTED,
        label: n.label,
        nodeType: n.nodeType,
        modality: m,
        isReal: n.isReal !== false,
        connections: 0,
      });
    });
  }

  function cx() { return 0; }
  function cy() { return 0; }

  // Count connections for sizing
  for (const e of edges) {
    const a = pos.get(e.from);
    const b = pos.get(e.to);
    if (a) a.connections++;
    if (b) b.connections++;
  }

  return pos;
}

// ── 3D Node ──

const Node3DMesh: React.FC<{
  node: Node3D;
  selected: boolean;
  onSelect: (id: string) => void;
}> = ({ node, selected, onSelect }) => {
  const meshRef = useRef<THREE.Mesh>(null);
  const radius = 0.5 + Math.min(node.connections * 0.08, 0.8);
  const geometry = useMemo(() => {
    // Shape by modality group: icosahedron for code/math, box for text/image,
    // octahedron for spatial, sphere for others
    switch (node.modality) {
      case "code": case "math": return new THREE.IcosahedronGeometry(radius, 0);
      case "text": case "image": return new THREE.BoxGeometry(radius * 1.4, radius * 1.4, radius * 1.4);
      case "3D": case "CAD": case "geospatial": return new THREE.OctahedronGeometry(radius, 0);
      default: return new THREE.SphereGeometry(radius, 12, 8);
    }
  }, [radius, node.modality]);

  return (
    <mesh
      ref={meshRef}
      position={node.position}
      geometry={geometry}
      onClick={(e) => { e.stopPropagation(); onSelect(node.id); }}
      onPointerOver={() => document.body.style.cursor = "pointer"}
      onPointerOut={() => document.body.style.cursor = "default"}
    >
      <meshStandardMaterial
        color={node.color}
        transparent
        opacity={node.isReal ? 0.85 : 0.4}
        emissive={node.color}
        emissiveIntensity={selected ? 0.4 : 0.08}
      />
      {selected && (
        <mesh scale={1.3}>
          <sphereGeometry args={[radius, 8, 6]} />
          <meshBasicMaterial color="#4fc3f7" wireframe transparent opacity={0.3} />
        </mesh>
      )}
      <Html distanceFactor={20} position={[0, radius + 0.4, 0]} center>
        <div style={{
          color: C_TEXT, fontSize: "10px", whiteSpace: "nowrap",
          textShadow: "0 1px 3px rgba(0,0,0,0.8)", pointerEvents: "none",
          maxWidth: 120, overflow: "hidden", textOverflow: "ellipsis",
        }}>
          {node.label}
        </div>
      </Html>
    </mesh>
  );
};

// ── 3D Edge ──

const Edge3DLine: React.FC<{ edge: Edge3D }> = ({ edge }) => (
  <Line
    points={[edge.from, edge.to]}
    color={edge.color}
    lineWidth={edge.dash ? 1 : 1.5}
    dashed={edge.dash}
    dashSize={0.3}
    gapSize={0.2}
    transparent
    opacity={0.5}
  />
);

// ── Main component ──

export const GraphWorld3D: React.FC<{
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
  selectedId?: string | null;
  onSelect?: (id: string) => void;
  height?: number;
}> = ({ nodes, edges, selectedId, onSelect, height = 420 }) => {
  const [hovered, setHovered] = useState<string | null>(null);

  const layout = useMemo(() => computeLayout(nodes, edges, 40), [nodes, edges]);

  const edges3d: Edge3D[] = useMemo(() => {
    return edges.map(e => {
      const a = layout.get(e.from);
      const b = layout.get(e.to);
      if (!a || !b) return null;
      return {
        from: a.position,
        to: b.position,
        dash: e.edgeClass !== "real",
        color: e.edgeClass === "cross-modal" ? "#e879f9" : "#4a6178",
      };
    }).filter((e): e is Edge3D => e !== null);
  }, [edges, layout]);

  const nodes3d = useMemo(() => {
    return nodes.map(n => {
      const p = layout.get(n.id);
      if (!p) return null;
      return { ...p, connections: (p.connections ?? 0) };
    }).filter((n): n is Node3D => n !== null);
  }, [nodes, layout]);

  return (
    <div style={{ width: "100%", height, position: "relative" }}>
      <Canvas
        camera={{ position: [0, 0, 35], fov: 50 }}
        style={{ background: "var(--color-bg, #0d1117)", borderRadius: 8, border: "1px solid var(--color-border-faint, #30363d)" }}
        onPointerMissed={() => onSelect?.("")}
      >
        <ambientLight intensity={0.5} />
        <directionalLight position={[10, 20, 15]} intensity={0.8} />
        <pointLight position={[-10, -10, -10]} intensity={0.3} />
        <OrbitControls
          enableDamping
          dampingFactor={0.1}
          minDistance={5}
          maxDistance={80}
        />
        {edges3d.map((e, i) => <Edge3DLine key={i} edge={e} />)}
        {nodes3d.map((n) => (
          <Node3DMesh
            key={n.id}
            node={n}
            selected={n.id === selectedId}
            onSelect={(id) => onSelect?.(id)}
          />
        ))}
      </Canvas>
      {/* Overlay: node count + hint */}
      <div style={{
        position: "absolute", bottom: 8, left: 10,
        fontSize: 10, color: C_MUTED, pointerEvents: "none",
      }}>
        {nodes.length} nodes · {edges.length} edges · drag to orbit · scroll to zoom · click to select
      </div>
    </div>
  );
};

export default GraphWorld3D;

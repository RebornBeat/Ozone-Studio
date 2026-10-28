/**
 * GraphWorld3D — the explorable, traversable 3D graph-native world (guide §8).
 *
 * A LEAF RENDERER behind GraphKit's contracts: it draws the WorldLayout the
 * pure layout module computed from the GraphProvider substrate, and reports
 * intent (select / open / hover) upward — it owns no data and fetches nothing.
 *
 * What it does:
 *  - Nodes as instanced meshes (one draw call per shape), colored by
 *    MODALITY_COLOR; the lens dims everything outside its highlight set.
 *  - Hubs (one per ModalityGraph container) wrapped in a translucent shell, so
 *    "this cluster is that graph" is visible, not inferred.
 *  - Edges batched per class (in-graph, cross-modal in magenta, file links,
 *    lineage…) — emphasised when the active lens means them, dimmed otherwise.
 *  - Camera fly-to: lens changes, selections and "focus" requests animate the
 *    orbit camera to frame the target; the operator keeps full orbit control.
 *  - Real construction pulses: a node a modality pipeline just constructed
 *    (node event) flashes and breathes for a few seconds.
 *  - Click selects, double-click opens, hover shows an honest tooltip.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Canvas, useFrame, useThree } from "@react-three/fiber";
import type { ThreeEvent } from "@react-three/fiber";
import { Billboard, Html, OrbitControls, Stars } from "@react-three/drei";
import * as THREE from "three";
import type {
  LensView,
  WorldEdgeClass,
  WorldLayout,
  WorldNode,
  WorldShape,
} from "./layout3d";

const PULSE_SECONDS = 5;

export interface FocusRequest {
  /** Bump to re-trigger a fly-to even when the target is unchanged. */
  key: number;
  center: [number, number, number];
  radius: number;
}

export interface GraphWorld3DProps {
  layout: WorldLayout;
  lens: LensView;
  selectedIndex: number | null;
  /** composite node id → unix-ms of its last real construction event */
  pulses: Record<string, number>;
  showLabels: boolean;
  autoRotate: boolean;
  focus: FocusRequest | null;
  onSelect: (index: number | null) => void;
  onOpen: (index: number) => void;
}

// ── Edge styling ──

const EDGE_STYLE: Record<WorldEdgeClass, { color: string; opacity: number; dashed: boolean }> = {
  structural: { color: "#6b8aa8", opacity: 0.6, dashed: false },
  dependency: { color: "#6aa8d8", opacity: 0.6, dashed: false },
  semantic: { color: "#8aa4bd", opacity: 0.5, dashed: true },
  "cross-modal": { color: "#e879f9", opacity: 0.95, dashed: false },
  lineage: { color: "#f59e0b", opacity: 0.8, dashed: false },
  governance: { color: "#34d399", opacity: 0.8, dashed: false },
  link: { color: "#b794f6", opacity: 0.75, dashed: true },
  file: { color: "#4fc3f7", opacity: 0.8, dashed: true },
  root: { color: "#4a5b6e", opacity: 0.4, dashed: false },
};

const EDGE_CLASSES = Object.keys(EDGE_STYLE) as WorldEdgeClass[];

const Edges: React.FC<{ layout: WorldLayout; lens: LensView; selectedIndex: number | null }> = ({ layout, lens, selectedIndex }) => {
  const objects = useMemo(() => {
    const out: THREE.LineSegments[] = [];
    const isActive = (i: number) => lens.active === null || lens.active.has(i);
    for (const cls of EDGE_CLASSES) {
      const style = EDGE_STYLE[cls];
      const emphasised = lens.emphasize.has(cls);
      const bright: number[] = [];
      const dim: number[] = [];
      for (const e of layout.edges) {
        if (e.cls !== cls) continue;
        const a = layout.nodes[e.a].pos;
        const b = layout.nodes[e.b].pos;
        // An edge is "lit" when the lens includes both ends, or when the lens
        // emphasises this class and includes either end.
        const lit =
          lens.active === null ||
          (isActive(e.a) && isActive(e.b)) ||
          (emphasised && (isActive(e.a) || isActive(e.b)));
        (lit ? bright : dim).push(a[0], a[1], a[2], b[0], b[1], b[2]);
      }
      const build = (verts: number[], opacity: number) => {
        if (verts.length === 0) return;
        const geom = new THREE.BufferGeometry();
        geom.setAttribute("position", new THREE.Float32BufferAttribute(verts, 3));
        const mat = style.dashed
          ? new THREE.LineDashedMaterial({
              color: style.color, transparent: true, opacity, dashSize: 0.55, gapSize: 0.4, depthWrite: false,
            })
          : new THREE.LineBasicMaterial({ color: style.color, transparent: true, opacity, depthWrite: false });
        const seg = new THREE.LineSegments(geom, mat);
        if (style.dashed) seg.computeLineDistances();
        seg.frustumCulled = false;
        out.push(seg);
      };
      build(bright, lens.active === null ? style.opacity : Math.min(1, style.opacity + (emphasised ? 0.15 : 0)));
      build(dim, 0.05);
    }
    // Edges touching the selection: drawn solid in the accent colour on top,
    // so "what is this connected to" reads at a glance.
    if (selectedIndex !== null) {
      const sel: number[] = [];
      for (const e of layout.edges) {
        if (e.a !== selectedIndex && e.b !== selectedIndex) continue;
        const a = layout.nodes[e.a].pos;
        const b = layout.nodes[e.b].pos;
        sel.push(a[0], a[1], a[2], b[0], b[1], b[2]);
      }
      if (sel.length > 0) {
        const geom = new THREE.BufferGeometry();
        geom.setAttribute("position", new THREE.Float32BufferAttribute(sel, 3));
        const seg = new THREE.LineSegments(
          geom,
          new THREE.LineBasicMaterial({ color: "#4fc3f7", transparent: true, opacity: 1, depthTest: false }),
        );
        seg.frustumCulled = false;
        seg.renderOrder = 5;
        out.push(seg);
      }
    }
    return out;
  }, [layout, lens, selectedIndex]);

  useEffect(
    () => () => {
      for (const o of objects) {
        o.geometry.dispose();
        (o.material as THREE.Material).dispose();
      }
    },
    [objects],
  );
  return (
    <>
      {objects.map((o, i) => (
        <primitive key={i} object={o} />
      ))}
    </>
  );
};

// ── Hub shells ──

const HubShells: React.FC<{ layout: WorldLayout; lens: LensView }> = ({ layout, lens }) => {
  const shells = useMemo(() => {
    const radius = new Map<number, number>();
    const hubPos = new Map<number, [number, number, number]>();
    for (const n of layout.nodes) if (n.kind === "hub" && n.containerId !== null) hubPos.set(n.containerId, n.pos);
    for (const n of layout.nodes) {
      if (n.kind !== "node" || n.containerId === null) continue;
      const hp = hubPos.get(n.containerId);
      if (!hp) continue;
      const d = Math.hypot(n.pos[0] - hp[0], n.pos[1] - hp[1], n.pos[2] - hp[2]) + n.radius + 0.6;
      radius.set(n.containerId, Math.max(radius.get(n.containerId) ?? 2.2, d));
    }
    return layout.nodes
      .filter((n) => n.kind === "hub")
      .map((n) => ({ node: n, r: radius.get(n.containerId ?? -1) ?? 2.2 }));
  }, [layout]);
  return (
    <>
      {shells.map(({ node, r }) => {
        const lit = lens.active === null || lens.active.has(node.index);
        return (
          <mesh key={node.id} position={node.pos} scale={r}>
            <sphereGeometry args={[1, 24, 16]} />
            <meshBasicMaterial color={node.color} transparent opacity={lit ? 0.07 : 0.02} depthWrite={false} />
          </mesh>
        );
      })}
    </>
  );
};

// ── Instanced node groups (one per shape) ──

const InstancedGroup: React.FC<{
  shape: WorldShape;
  items: WorldNode[];
  layout: WorldLayout;
  lens: LensView;
  selectedIndex: number | null;
  hoveredIndex: number | null;
  pulses: Record<string, number>;
  onSelect: (index: number | null) => void;
  onOpen: (index: number) => void;
  onHover: (index: number | null) => void;
}> = ({ shape, items, layout, lens, selectedIndex, hoveredIndex, pulses, onSelect, onOpen, onHover }) => {
  const ref = useRef<THREE.InstancedMesh>(null);
  const dummy = useMemo(() => new THREE.Object3D(), []);
  const tint = useMemo(() => new THREE.Color(), []);
  const white = useMemo(() => new THREE.Color("#ffffff"), []);

  const geometry = useMemo(() => {
    switch (shape) {
      case "box": return new THREE.BoxGeometry(1.15, 1.15, 1.15);
      case "octa": return new THREE.OctahedronGeometry(0.85, 0);
      case "ico": return new THREE.IcosahedronGeometry(0.8, 0);
      default: return new THREE.SphereGeometry(0.75, 16, 12);
    }
  }, [shape]);
  useEffect(() => () => geometry.dispose(), [geometry]);

  // Which of this group's items are pulsing (real construction events).
  const pulseStart = useMemo(() => {
    const m = new Map<number, number>(); // item slot → start ms
    for (const [id, t] of Object.entries(pulses)) {
      const idx = layout.byId.get(id);
      if (idx === undefined) continue;
      const slot = items.findIndex((it) => it.index === idx);
      if (slot >= 0) m.set(slot, t);
    }
    return m;
  }, [pulses, layout, items]);

  const apply = useCallback(
    (nowMs: number) => {
      const mesh = ref.current;
      if (!mesh) return;
      for (let i = 0; i < items.length; i++) {
        const it = items[i];
        const dim = lens.active !== null && !lens.active.has(it.index);
        const sel = it.index === selectedIndex;
        const hov = it.index === hoveredIndex;
        let scale = it.radius * (sel ? 1.4 : hov ? 1.2 : 1) * (dim ? 0.55 : 1);
        tint.set(it.color);
        if (dim) tint.multiplyScalar(0.22);
        const started = pulseStart.get(i);
        if (started !== undefined) {
          const age = (nowMs - started) / 1000;
          if (age >= 0 && age < PULSE_SECONDS) {
            const k = 1 - age / PULSE_SECONDS;
            scale *= 1 + 0.9 * k * (0.55 + 0.45 * Math.sin(age * 9));
            tint.lerp(white, 0.75 * k);
          }
        }
        dummy.position.set(it.pos[0], it.pos[1], it.pos[2]);
        dummy.scale.setScalar(scale);
        dummy.updateMatrix();
        mesh.setMatrixAt(i, dummy.matrix);
        mesh.setColorAt(i, tint);
      }
      mesh.instanceMatrix.needsUpdate = true;
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    },
    [items, lens, selectedIndex, hoveredIndex, pulseStart, dummy, tint, white],
  );

  useEffect(() => {
    apply(Date.now());
  }, [apply]);

  const wasPulsing = useRef(false);
  useFrame(() => {
    if (pulseStart.size === 0) {
      if (wasPulsing.current) {
        wasPulsing.current = false;
        apply(Date.now());
      }
      return;
    }
    const now = Date.now();
    let live = false;
    for (const t of pulseStart.values()) if ((now - t) / 1000 < PULSE_SECONDS) { live = true; break; }
    if (live) {
      wasPulsing.current = true;
      apply(now);
    } else if (wasPulsing.current) {
      wasPulsing.current = false;
      apply(now);
    }
  });

  const slotOf = (e: ThreeEvent<MouseEvent | PointerEvent>): number | null =>
    typeof e.instanceId === "number" && e.instanceId < items.length ? items[e.instanceId].index : null;

  if (items.length === 0) return null;
  return (
    <instancedMesh
      key={`${shape}:${items.length}`}
      ref={ref}
      args={[geometry, undefined as unknown as THREE.Material, items.length]}
      frustumCulled={false}
      onClick={(e) => {
        e.stopPropagation();
        const idx = slotOf(e);
        if (idx !== null) onSelect(idx);
      }}
      onDoubleClick={(e) => {
        e.stopPropagation();
        const idx = slotOf(e);
        if (idx !== null) onOpen(idx);
      }}
      onPointerMove={(e) => {
        e.stopPropagation();
        const idx = slotOf(e);
        if (idx !== hoveredIndex) onHover(idx);
        document.body.style.cursor = idx === null ? "default" : "pointer";
      }}
      onPointerOut={() => {
        onHover(null);
        document.body.style.cursor = "default";
      }}
    >
      <meshLambertMaterial />
    </instancedMesh>
  );
};

// ── Camera rig (fly-to) ──

const ease = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2);

const CameraRig: React.FC<{ focus: FocusRequest | null; controlsRef: React.MutableRefObject<any> }> = ({
  focus,
  controlsRef,
}) => {
  const { camera } = useThree();
  const anim = useRef<{
    t: number;
    fromPos: THREE.Vector3;
    toPos: THREE.Vector3;
    fromTarget: THREE.Vector3;
    toTarget: THREE.Vector3;
  } | null>(null);

  useEffect(() => {
    const controls = controlsRef.current;
    if (!focus || !controls) return;
    const toTarget = new THREE.Vector3(...focus.center);
    const dir = camera.position.clone().sub(controls.target);
    if (dir.lengthSq() < 1e-6) dir.set(0, 0.25, 1);
    dir.normalize();
    // Frame the target to fill the view: a narrow aspect needs more distance
    // to fit the same width.
    const aspect = (camera as THREE.PerspectiveCamera).aspect || 1;
    const distance = Math.min(150, Math.max(10, focus.radius * 2.0 * Math.max(1, 1.2 / aspect)));
    anim.current = {
      t: 0,
      fromPos: camera.position.clone(),
      toPos: toTarget.clone().add(dir.multiplyScalar(distance)),
      fromTarget: controls.target.clone(),
      toTarget,
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focus?.key]);

  useFrame((_, dt) => {
    const a = anim.current;
    const controls = controlsRef.current;
    if (!a || !controls) return;
    a.t = Math.min(1, a.t + dt / 0.85);
    const k = ease(a.t);
    camera.position.lerpVectors(a.fromPos, a.toPos, k);
    controls.target.lerpVectors(a.fromTarget, a.toTarget, k);
    controls.update();
    if (a.t >= 1) anim.current = null;
  });
  return null;
};

// ── Labels, selection ring, tooltip ──

const Label: React.FC<{ node: WorldNode; strong?: boolean }> = ({ node, strong }) => (
  <Html position={[node.pos[0], node.pos[1] + Math.max(node.radius + 0.7, node.shell + 0.6), node.pos[2]]} center zIndexRange={[20, 0]} style={{ pointerEvents: "none" }}>
    <div className={strong ? "gnw3d-label gnw3d-label-strong" : "gnw3d-label"}>{node.label}</div>
  </Html>
);

const SelectionRing: React.FC<{ node: WorldNode }> = ({ node }) => {
  const ref = useRef<THREE.Mesh>(null);
  useFrame((_, dt) => {
    if (ref.current) ref.current.rotation.z += dt * 0.8;
  });
  return (
    <Billboard position={node.pos}>
      <mesh ref={ref}>
        <torusGeometry args={[node.radius * 1.9 + 0.3, 0.07, 8, 48]} />
        <meshBasicMaterial color="#4fc3f7" transparent opacity={0.9} />
      </mesh>
    </Billboard>
  );
};

const Tooltip: React.FC<{ node: WorldNode }> = ({ node }) => (
  <Html position={[node.pos[0], node.pos[1] - node.radius - 0.5, node.pos[2]]} center zIndexRange={[40, 0]} style={{ pointerEvents: "none" }}>
    <div className="gnw3d-tip">
      <div className="gnw3d-tip-title">{node.label}</div>
      <div className="gnw3d-tip-sub">
        {node.nodeType}
        {node.modality !== "container" ? ` · ${node.modality}` : ""}
        {!node.isReal ? " · derived" : ""}
        {node.degree > 0 ? ` · ${node.degree} link${node.degree === 1 ? "" : "s"}` : ""}
      </div>
    </div>
  </Html>
);

// ── Main ──

export const GraphWorld3D: React.FC<GraphWorld3DProps> = ({
  layout, lens, selectedIndex, pulses, showLabels, autoRotate, focus, onSelect, onOpen,
}) => {
  const controlsRef = useRef<any>(null);
  const [hovered, setHovered] = useState<number | null>(null);

  const groups = useMemo(() => {
    const by: Record<WorldShape, WorldNode[]> = { sphere: [], box: [], octa: [], ico: [] };
    for (const n of layout.nodes) by[n.shape].push(n);
    return by;
  }, [layout]);

  // Which nodes carry a text label: the selection and hover always; hubs and
  // the root when the lens includes them; everything when the world is small.
  const labelled = useMemo(() => {
    const set = new Set<number>();
    const small = layout.nodes.length <= 70;
    for (const n of layout.nodes) {
      const lit = lens.active === null || lens.active.has(n.index);
      if (!lit) continue;
      if (n.kind === "root" || n.kind === "hub") set.add(n.index);
      else if (small && showLabels) set.add(n.index);
    }
    return set;
  }, [layout, lens, showLabels]);

  const selected = selectedIndex !== null ? layout.nodes[selectedIndex] : null;
  const hoveredNode = hovered !== null ? layout.nodes[hovered] : null;

  return (
    <Canvas
      camera={{ position: [0, layout.radius * 0.4, layout.radius * 1.9 + 8], fov: 50, near: 0.1, far: 1200 }}
      dpr={[1, 2]}
      gl={{ antialias: true }}
      onPointerMissed={() => onSelect(null)}
    >
      <color attach="background" args={["#0a1017"]} />
      <fog attach="fog" args={["#0a1017", layout.radius * 2.2, layout.radius * 7 + 60]} />
      <ambientLight intensity={0.85} />
      <directionalLight position={[12, 24, 16]} intensity={0.9} />
      <pointLight position={[-14, -10, -12]} intensity={0.35} />
      <Stars radius={Math.max(260, layout.radius * 4)} depth={60} count={1400} factor={3} saturation={0} fade speed={0.4} />
      <OrbitControls
        ref={controlsRef}
        makeDefault
        enableDamping
        dampingFactor={0.09}
        minDistance={4}
        maxDistance={Math.max(120, layout.radius * 6)}
        autoRotate={autoRotate}
        autoRotateSpeed={0.5}
      />
      <CameraRig focus={focus} controlsRef={controlsRef} />
      <HubShells layout={layout} lens={lens} />
      <Edges layout={layout} lens={lens} selectedIndex={selectedIndex} />
      {(Object.keys(groups) as WorldShape[]).map((shape) => (
        <InstancedGroup
          key={shape}
          shape={shape}
          items={groups[shape]}
          layout={layout}
          lens={lens}
          selectedIndex={selectedIndex}
          hoveredIndex={hovered}
          pulses={pulses}
          onSelect={onSelect}
          onOpen={onOpen}
          onHover={setHovered}
        />
      ))}
      {Array.from(labelled).map((i) =>
        i === selectedIndex || i === hovered ? null : <Label key={`l${i}`} node={layout.nodes[i]} />,
      )}
      {selected && (
        <>
          <SelectionRing node={selected} />
          <Label node={selected} strong />
        </>
      )}
      {hoveredNode && hoveredNode.index !== selectedIndex && <Tooltip node={hoveredNode} />}
    </Canvas>
  );
};

export default GraphWorld3D;

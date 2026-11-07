# CadQuery (OpenCascade) solid-model clock -> STEP + STL. usage: clock_cad.py <out_dir> [radius_mm]
import sys, os, json, math
import cadquery as cq
out = sys.argv[1]; R = float(sys.argv[2]) if len(sys.argv) > 2 else 150.0
os.makedirs(out, exist_ok=True)
T = 12.0                                    # case depth
case = cq.Workplane("XY").circle(R).extrude(T)
case = case.faces(">Z").workplane().circle(R - 10).cutBlind(-(T - 3))   # recessed face, 3 mm back plate
case = case.faces(">Z").edges().fillet(2.0)
# 12 hour-marker pegs (long at 12/3/6/9)
asm = cq.Assembly(name="clock")
asm.add(case, name="case", color=cq.Color(0.55, 0.38, 0.12))
base = T - (T - 3)                          # floor of the recess
for h in range(12):
    a = math.radians(h * 30); L = 22 if h % 3 == 0 else 12; W = 4 if h % 3 == 0 else 2.5
    rad = R - 14 - L / 2
    m = (cq.Workplane("XY").workplane(offset=base).center(rad * math.sin(a), rad * math.cos(a))
         .rect(W, L).extrude(1.5).rotate((rad * math.sin(a), rad * math.cos(a), 0), (rad * math.sin(a), rad * math.cos(a), 1), -h * 30))
    asm.add(m, name=f"marker_{h or 12}", color=cq.Color(0.05, 0.05, 0.07))
def hand(name, length, width, angle, z, col):
    h = (cq.Workplane("XY").workplane(offset=z).polyline([(-width / 2, -length * 0.12), (width / 2, -length * 0.12), (width / 4, length), (-width / 4, length)]).close().extrude(1.2)
         .rotate((0, 0, 0), (0, 0, 1), -angle))
    asm.add(h, name=name, color=col)
hand("hour", R * 0.50, 9, 10 * 30 + 5, base + 1.5, cq.Color(0.02, 0.02, 0.03))
hand("minute", R * 0.75, 6, 60, base + 2.7, cq.Color(0.02, 0.02, 0.03))
hand("second", R * 0.80, 2.2, 0, base + 3.9, cq.Color(0.75, 0.05, 0.05))
asm.add(cq.Workplane("XY").workplane(offset=base).circle(6).extrude(6), name="hub", color=cq.Color(0.55, 0.38, 0.12))
step = os.path.join(out, "clock.step"); stl = os.path.join(out, "clock.stl")
asm.save(step, "STEP")
comp = asm.toCompound()
cq.exporters.export(cq.Workplane("XY").add(comp), stl)
vol = comp.Volume()
bb = comp.BoundingBox()
print(json.dumps({"success": True, "step": step, "stl": stl, "solids": len(comp.Solids()), "volume_mm3": round(vol, 1),
                  "bbox_mm": [round(bb.xlen, 1), round(bb.ylen, 1), round(bb.zlen, 1)], "bytes": {"step": os.path.getsize(step), "stl": os.path.getsize(stl)}}))

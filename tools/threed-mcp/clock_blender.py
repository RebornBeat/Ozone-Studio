# Headless Blender worker: parametric analog wall clock -> GLB + STL + PNG render.
# usage: blender --background --python clock_blender.py -- <out_dir> <HH:MM> [radius_m]
# Prints ONE JSON line (the MCP parses the last line starting with "{").
import sys, os, json, math
import bpy

argv = sys.argv[sys.argv.index("--") + 1:]
out_dir = argv[0]; hh, mm = [int(x) for x in argv[1].split(":")]
R = float(argv[2]) if len(argv) > 2 else 0.15
os.makedirs(out_dir, exist_ok=True)

bpy.ops.wm.read_factory_settings(use_empty=True)

def mat(name, rgba, metallic=0.0, rough=0.5):
    m = bpy.data.materials.new(name); m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = rgba
    b.inputs["Metallic"].default_value = metallic
    b.inputs["Roughness"].default_value = rough
    m.diffuse_color = rgba
    return m

def assign(o, m): o.data.materials.append(m); return o

parts = []
# face
bpy.ops.mesh.primitive_cylinder_add(radius=R, depth=0.012, vertices=96, location=(0, 0, 0))
face = bpy.context.object; face.name = "ClockFace"; assign(face, mat("Face", (0.95, 0.93, 0.86, 1), 0.0, 0.6)); parts.append(face)
# rim
bpy.ops.mesh.primitive_torus_add(major_radius=R, minor_radius=R * 0.06, major_segments=96, minor_segments=24, location=(0, 0, 0.004))
rim = bpy.context.object; rim.name = "ClockRim"; assign(rim, mat("Rim", (0.55, 0.38, 0.12, 1), 0.9, 0.3)); parts.append(rim)
# hour markers (12; the 12/3/6/9 ones are longer)
dark = mat("Marker", (0.05, 0.05, 0.07, 1), 0.2, 0.4)
for h in range(12):
    a = math.radians(h * 30)
    long_ = h % 3 == 0
    L = R * (0.16 if long_ else 0.09); W = R * (0.035 if long_ else 0.02)
    rad = R * 0.86 - L / 2
    bpy.ops.mesh.primitive_cube_add(size=1, location=(rad * math.sin(a), rad * math.cos(a), 0.009))
    mk = bpy.context.object; mk.name = f"Marker{h or 12}"
    mk.scale = (W, L, 0.004); mk.rotation_euler = (0, 0, -a); assign(mk, dark); parts.append(mk)

def hand(name, length, width, angle_deg, z, m):
    a = math.radians(angle_deg)
    bpy.ops.mesh.primitive_cube_add(size=1, location=(0, 0, z))
    o = bpy.context.object; o.name = name
    o.scale = (width, length, 0.004)
    # pivot at the centre: shift geometry up by half length, then rotate about origin
    for v in o.data.vertices: v.co.y += 0.5
    o.rotation_euler = (0, 0, -a); assign(o, m); parts.append(o); return o

hand_m = mat("Hand", (0.02, 0.02, 0.03, 1), 0.6, 0.35)
sec_m = mat("Second", (0.75, 0.05, 0.05, 1), 0.3, 0.4)
hour_angle = (hh % 12) * 30 + mm * 0.5
min_angle = mm * 6
hand("HourHand", R * 0.50, R * 0.045, hour_angle, 0.014, hand_m)
hand("MinuteHand", R * 0.75, R * 0.03, min_angle, 0.018, hand_m)
hand("SecondHand", R * 0.80, R * 0.012, 0, 0.022, sec_m)
bpy.ops.mesh.primitive_uv_sphere_add(radius=R * 0.04, location=(0, 0, 0.024))
cap = bpy.context.object; cap.name = "CenterCap"; assign(cap, mat("Cap", (0.55, 0.38, 0.12, 1), 0.9, 0.3)); parts.append(cap)

# stand the clock upright (face toward -Y camera): rotate whole assembly 90deg about X
for o in parts: o.select_set(True)
bpy.context.view_layer.objects.active = parts[0]
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
root = bpy.data.objects.new("Clock", None); bpy.context.collection.objects.link(root)
for o in parts: o.parent = root
root.rotation_euler = (math.radians(90), 0, 0)
bpy.context.view_layer.update()

glb = os.path.join(out_dir, f"clock_{hh:02d}{mm:02d}.glb")
stl = os.path.join(out_dir, f"clock_{hh:02d}{mm:02d}.stl")
png = os.path.join(out_dir, f"clock_{hh:02d}{mm:02d}.png")
bpy.ops.export_scene.gltf(filepath=glb, export_format="GLB")
bpy.ops.object.select_all(action="DESELECT")
for o in parts: o.select_set(True)
try:
    bpy.ops.wm.stl_export(filepath=stl, export_selected_objects=True)   # Blender >= 4.1
except AttributeError:
    import addon_utils; addon_utils.enable("io_mesh_stl")
    bpy.ops.export_mesh.stl(filepath=stl, use_selection=True)           # Blender 4.0

# render (Workbench: works headless without GPU)
cam_d = bpy.data.cameras.new("Cam"); cam = bpy.data.objects.new("Cam", cam_d)
bpy.context.collection.objects.link(cam); cam.location = (0, -R * 5, 0); cam.rotation_euler = (math.radians(90), 0, 0)
cam_d.lens = 85; bpy.context.scene.camera = cam
sc = bpy.context.scene; sc.render.engine = "BLENDER_WORKBENCH"
sc.display.shading.light = "STUDIO"; sc.display.shading.color_type = "MATERIAL"
sc.render.resolution_x = 900; sc.render.resolution_y = 900; sc.render.filepath = png
sc.world = bpy.data.worlds.new("W"); sc.world.color = (0.12, 0.13, 0.16)
bpy.ops.render.render(write_still=True)

print(json.dumps({"success": True, "time": f"{hh:02d}:{mm:02d}", "hour_angle_deg": hour_angle, "minute_angle_deg": min_angle,
                  "parts": len(parts), "glb": glb, "stl": stl, "png": png,
                  "bytes": {k: os.path.getsize(p) for k, p in (("glb", glb), ("stl", stl), ("png", png))}}))

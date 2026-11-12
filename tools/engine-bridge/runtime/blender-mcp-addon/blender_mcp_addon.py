# Ozone-Studio Blender bridge addon (server side).
# Runs INSIDE Blender, listens on 127.0.0.1:9876, speaks the newline-delimited
# JSON protocol the gamedev TcpBridge expects:
#   request:  {"id": "<uuid>", "method": "get_scene", "params": {...}}\n
#   response: {"id": "<uuid>", "status": "ok"|"error", "result": ...}\n
# Methods (matching tools/bridges blender tools): get_scene, get_object,
# create_object, delete_object, set_transform, set_material, run_python, export.
# Launch:  blender --background --python blender_mcp_addon.py
# This is Ozone-Studio internal tooling (absorbed bridges package runtime).

import bpy
import json
import threading
import uuid
from mathutils import Euler

HOST, PORT = "127.0.0.1", 9876

def _obj_summary(obj, include_transforms=True):
    d = {
        "name": obj.name,
        "type": obj.type,
        "parent": obj.parent.name if obj.parent else None,
        "children": [c.name for c in obj.children],
    }
    if include_transforms:
        d["location"] = list(obj.location)
        d["rotation_euler"] = list(obj.rotation_euler)
        d["scale"] = list(obj.scale)
    return d

def handle(method, params):
    if method == "get_scene":
        include_transforms = bool(params.get("includeTransforms", True))
        scene = bpy.context.scene
        return {
            "scene": scene.name,
            "frame_current": scene.frame_current,
            "objects": [_obj_summary(o, include_transforms) for o in scene.objects],
        }

    if method == "get_object":
        name = params.get("name")
        obj = bpy.data.objects.get(name)
        if obj is None:
            raise ValueError(f"object not found: {name}")
        return _obj_summary(obj, bool(params.get("includeTransforms", True)))

    if method == "create_object":
        kind = str(params.get("type", "Cube")).lower()
        name = params.get("name") or f"Ozone_{uuid.uuid4().hex[:6]}"
        if "sphere" in kind:
            bpy.ops.mesh.primitive_uv_sphere_add(radius=1)
        elif "cylinder" in kind:
            bpy.ops.mesh.primitive_cylinder_add(radius=1, depth=2)
        elif "plane" in kind:
            bpy.ops.mesh.primitive_plane_add(size=2)
        elif "torus" in kind:
            bpy.ops.mesh.primitive_torus_add()
        else:
            bpy.ops.mesh.primitive_cube_add(size=2)
        # background mode has no context.active_object — view_layer IS there
        obj = bpy.context.view_layer.objects.active
        obj.name = name
        loc = params.get("location")
        if loc:
            obj.location = loc
        return _obj_summary(obj)

    if method == "delete_object":
        name = params.get("name")
        obj = bpy.data.objects.get(name)
        if obj is None:
            raise ValueError(f"object not found: {name}")
        bpy.data.objects.remove(obj, do_unlink=True)
        return {"deleted": name}

    if method == "set_transform":
        name = params.get("name")
        obj = bpy.data.objects.get(name)
        if obj is None:
            raise ValueError(f"object not found: {name}")
        if params.get("location"):
            obj.location = params["location"]
        if params.get("rotation"):
            obj.rotation_euler = Euler(params["rotation"])
        if params.get("scale"):
            obj.scale = params["scale"]
        return _obj_summary(obj)

    if method == "set_material":
        name = params.get("name")
        obj = bpy.data.objects.get(name)
        if obj is None:
            raise ValueError(f"object not found: {name}")
        color = params.get("color", [0.8, 0.8, 0.8, 1.0])
        mat = bpy.data.materials.new(name=f"{name}_mat")
        mat.use_nodes = True
        bsdf = mat.node_tree.nodes.get("Principled BSDF")
        if bsdf:
            bsdf.inputs["Base Color"].default_value = color
        obj.data.materials.clear()
        obj.data.materials.append(mat)
        return {"material": mat.name, "object": name}

    if method == "run_python":
        code = params.get("code", "")
        namespace = {"bpy": bpy}
        exec(code, namespace)  # noqa: S102 - deliberate operator-level capability
        return {"ok": True}

    if method == "export":
        filepath = params.get("path") or "/tmp/ozone_blender_export.glb"
        fmt = str(params.get("format", "glTF")).lower()
        sel = bpy.ops.object.select_all(action="SELECT")
        if "gltf" in fmt or "glb" in fmt:
            bpy.ops.export_scene.gltf(filepath=filepath, export_format="GLB" if filepath.endswith(".glb") else "GLTF_SEPARATE")
        elif "obj" in fmt:
            bpy.ops.wm.obj_export(filepath=filepath)
        else:
            raise ValueError(f"unsupported export format: {params.get('format')}")
        return {"exported": filepath}

    raise ValueError(f"unknown method: {method}")

class Handler(threading.Thread):
    def __init__(self, conn):
        super().__init__(daemon=True)
        self.conn = conn
        self.buffer = ""

    def run(self):
        while True:
            try:
                chunk = self.conn.recv(65536)
            except OSError:
                return
            if not chunk:
                return
            self.buffer += chunk.decode("utf-8", "replace")
            while "\n" in self.buffer:
                line, self.buffer = self.buffer.split("\n", 1)
                line = line.strip()
                if not line:
                    continue
                try:
                    req = json.loads(line)
                    result = handle(req.get("method", ""), req.get("params", {}) or {})
                    resp = {"id": req.get("id"), "status": "ok", "result": result}
                except Exception as e:  # noqa: BLE001 - report, never crash the server
                    resp = {"id": req.get("id"), "status": "error", "error": str(e)}
                try:
                    self.conn.sendall((json.dumps(resp) + "\n").encode("utf-8"))
                except OSError:
                    return

def main():
    import socket
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((HOST, PORT))
    srv.listen(5)
    print(f"[ozone-blender-addon] listening on {HOST}:{PORT}", flush=True)
    while True:
        conn, addr = srv.accept()
        Handler(conn).start()

# Headless background launches still execute this module; start the server
# on a thread and BLOCK the main thread — otherwise `blender --background`
# finishes the script and exits, killing the bridge.
t = threading.Thread(target=main, daemon=True)
t.start()
print("[ozone-blender-addon] bridge thread started", flush=True)
import time
while True:
    time.sleep(3600)

"""Ozone-Studio 3D MCP — eeg_cap_costume_generate tool's Blender worker.

Run headless: `blender --background --python eeg_cap_costume_export.py -- <out.glb>`

CORRECTED SCOPE (operator, 2026-09-29): this is a 3D-MODALITY (109) tool,
NOT part of Cerebrix (the EEG analysis API) — it was originally built
inside tools/cerebrix-mcp/ and moved here once the operator clarified the
boundary. The rat model's real purpose is EEG-CAP COSTUME SIMULATION —
generating an anatomically-measured rat body/head to design/fit a
physical EEG electrode cap against, not a general anatomical reference
and not part of Cerebrix's own signal-processing scope.

Imports the REAL, unmodified AnatomicalRatModel class from CereBrixx/
RatModel.py (the legacy anatomical-rat Blender generator that predates
the Cerebrix repo's Addons/AnimalInterface/Rat/ addon — that addon
folder currently holds only a README, no code yet, confirmed 2026-09-29;
this is the one real, working 3D asset that exists today) and calls its
own real assemble_model() directly — no source changes to RatModel.py.
Exports the result as a real glTF (.glb) mesh + prints a JSON summary
(object/bone counts) on stdout for the MCP server to capture and hand to
the 3D modality graph (109) via the same physical-bridge pattern
documented in docs/MODALITY_EXPANSION_GUIDE.md §3.

KNOWN REAL LIMITATION (found live, 2026-09-29, not silently worked
around): running RatModel.py's full assemble_model() sequence headless
(`blender --background`) hangs somewhere after create_muscular_body()
completes and before create_detailed_head() finishes — confirmed via
isolated testing: skeleton+body alone complete correctly in under 20s;
create_eyes()/create_ears() individually ALSO complete correctly in
isolation (materials, EDIT-mode extrude, all fine); but the full chain
run in sequence does not finish within 30s. Root cause not isolated
further (likely a headless-mode context/selection-state interaction
specific to Blender 4.0.2 that the original script was never tested
against outside the interactive UI) — flagged honestly rather than
guessed at. The MCP server wraps this call with a hard timeout so a
hang fails the tool call cleanly instead of hanging the server.
"""
import sys
import os
import json

RATMODEL_DIR = os.environ.get("RATMODEL_DIR", "/home/rebornbeat/Projects/CereBrixx")
sys.path.insert(0, RATMODEL_DIR)

# Args after Blender's own `--` separator.
argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
out_path = argv[0] if argv else "/tmp/rat_model.glb"

try:
    import bpy
    from RatModel import AnatomicalRatModel
except Exception as e:  # noqa: BLE001
    print(json.dumps({"success": False, "error": f"import failed: {e}"}))
    sys.exit(0)

try:
    generator = AnatomicalRatModel()
    body = generator.assemble_model()  # the real, unmodified method

    bpy.ops.export_scene.gltf(filepath=out_path, export_format="GLB")

    mesh_objects = [o.name for o in bpy.data.objects if o.type == "MESH"]
    armature = generator.armature
    bone_count = len(armature.data.bones) if armature else 0

    print(json.dumps({
        "success": True,
        "output_path": out_path,
        "mesh_objects": mesh_objects,
        "bone_count": bone_count,
        "measurements_cm": {
            "total_length": generator.TOTAL_LENGTH * 10,
            "tail_length": generator.TAIL_LENGTH * 10,
            "body_height": generator.BODY_HEIGHT * 10,
            "head_length": generator.HEAD_LENGTH * 10,
        },
    }))
except Exception as e:  # noqa: BLE001
    print(json.dumps({"success": False, "error": str(e)}))

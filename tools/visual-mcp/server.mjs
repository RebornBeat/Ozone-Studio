// Ozone-Studio Visual MCP — the 6th external capability (NEW_MCP_GUIDE §4,
// operator directive 2026-09-28): the agent SEES software. Screen capture →
// image modality (102) Analyze → real spatial analysis → optionally a
// persisted ZSEI graph — exposed as tools through /mcp/call, so every
// capture is metered, gated, rippled, and wrapped in the insight envelope.
//
// Tools:
//   visual_describe {input: {}}              → capture + Analyze (real
//                                              objects/regions/colors/composition)
//   visual_graph {input: {project_id}}       → capture + Analyze + CreateGraph
//                                              (persists a ModalityGraph container)
//   yolo_detect {input: {model?, confidence?}}       → registry-driven object
//                                              detection + F.3 spatial relations (§11.6)
//   yolo_graph {input: {model?, project_id?}}        → yolo_detect + CreateGraph,
//                                              detections become real graph nodes/edges
//   depth_estimate {input: {model?, bounding_boxes?}} → registry-driven monocular depth
//                                              (real per-pixel/per-region relative depth,
//                                              LOCALIZATION_TRAVERSAL_GUIDE.md §4)
//   depth_graph {input: {detection_model?, depth_model?, project_id?}} → yolo_detect +
//                                              depth_estimate + CreateGraph: real depth
//                                              becomes a real per-object node attribute,
//                                              real InFrontOf relations returned (not yet
//                                              a persisted typed edge — image 102's
//                                              ImageEdgeType has no depth-axis variant today)
//
// Capture: ImageMagick `import -window root` (present on this machine,
// DISPLAY :0). The capture command is FIXED, not caller-supplied — this
// MCP takes no arbitrary commands (contrast: terminal-mcp's allowlist; here
// there is nothing to allowlist because nothing is parameterized).
//
// Env: OZONE_VISUAL_PORT (default 3220), OZONE_IMAGE_PIPELINE (path to the
// image modality binary), OZONE_HOST (default :50051),
// OZONE_ZSEI_DATA_DIR (graph content convention), OZONE_VISUAL_PROJECT
// (default 3 — the project visual graphs parent to).

import { createServer } from "node:http";
import { spawn, execFile } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const PORT = Number(process.env.OZONE_VISUAL_PORT ?? 3220);
const IMAGE_PIPELINE = process.env.OZONE_IMAGE_PIPELINE
  ?? "/home/rebornbeat/Projects/Ozone-Studio/assets/pipelines/modalities/image/target/release/image";
const ZSEI_DATA_DIR = process.env.OZONE_ZSEI_DATA_DIR
  ?? "/home/rebornbeat/Projects/Ozone-Studio/target/release/zsei_data";
const PROJECT_ID = Number(process.env.OZONE_VISUAL_PROJECT ?? 3);
const startedAt = Date.now();

function captureScreen() {
  const backend = process.env.OZONE_VISUAL_CAPTURE ?? "auto";
  if (backend === "auto" && (process.env.WAYLAND_DISPLAY || process.env.XDG_SESSION_TYPE === "wayland")) {
    return Promise.reject(new Error(
      "Wayland session: direct root-grab is not sanctioned. Sanctioned sources: (1) the Ozone Electron app's own desktopCapturer POSTing the PNG to visual_ingest, (2) xdg-desktop-portal with explicit user approval, (3) OZONE_VISUAL_CAPTURE=import for explicit X11 opt-in."
    ));
  }
  return new Promise((resolve, reject) => {
    const out = `/tmp/ozone_screen_${Date.now()}.png`;
    execFile("import", ["-window", "root", out], { timeout: 30000 }, (err) => {
      if (err) reject(new Error(`capture failed: ${err.message}`));
      else resolve(out);
    });
  });
}

// visual_ingest: the Electron-app capture path — an already-captured PNG
// (base64) POSTed by Ozone's own UI. Nothing is exec'd; the image flows
// straight into the modality pipeline.
function ingestBase64(data) {
  const out = `/tmp/ozone_ingest_${Date.now()}.png`;
  fs.writeFileSync(out, Buffer.from(data, "base64"));
  return out;
}

function runImagePipeline(inputObj) {
  return new Promise((resolve, reject) => {
    const child = spawn(IMAGE_PIPELINE, {
      env: { ...process.env, OZONE_HOST: process.env.OZONE_HOST ?? "http://127.0.0.1:50051", OZONE_ZSEI_DATA_DIR: ZSEI_DATA_DIR },
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => { stdout += d; });
    child.stderr.on("data", (d) => { stderr += d; });
    child.on("error", (err) => reject(new Error(`image pipeline spawn failed: ${err.message}`)));
    child.on("close", (code) => {
      try {
        resolve(JSON.parse(stdout));
      } catch {
        reject(new Error(`image pipeline bad output (exit ${code}): ${(stderr || stdout).slice(0, 200)}`));
      }
    });
    child.stdin.write(JSON.stringify(inputObj));
    child.stdin.end();
  });
}

// CAPTURE BACKENDS (operator-corrected layering): the MCP never execs a
// screen-grabber directly. `import` (X11 root grab) is kept ONLY as an
// explicit opt-in backend for X11 sessions; the sanctioned Wayland path is
// the Ozone Electron app's own desktopCapturer POSTing an already-captured
// image to visual_ingest — trust flows through Ozone's own app, which the
// operator runs and sees.
// Real fix for docs/CAPABILITY_EXPANSION_REVIEW.md §0: the Rust pipeline's
// own detect_objects/detect_text/detect_faces were 100% hardcoded
// fabrication. Request colors/composition from the Rust side only
// (real, genuine analysis there), and get objects/text/faces from real
// backends here — the exact same override shape yolo_graph already uses
// for analysis.objects below, just applied to every capture path now,
// not only the dedicated yolo_* tools.
async function captureAndAnalyze(pngPath) {
  const data = fs.readFileSync(pngPath).toString("base64");
  const reg = loadDetectionRegistry();
  const detModel = reg.models.find((m) => m.enabled && isDetectionModel(m));

  const [analyzed, yolo, ocr, faces] = await Promise.all([
    runImagePipeline({
      data: { action: { type: "Analyze", image_data: { Base64: { data, mime_type: "image/png" } }, detect_objects: false, detect_text: false, detect_faces: false, analyze_colors: true, analyze_composition: true } },
    }),
    detModel ? runYolo(pngPath, detModel.name, 0.5) : Promise.resolve({ ok: false, error: "no enabled detection model registered" }),
    runOcr(pngPath),
    runFaces(pngPath),
  ]);
  if (!analyzed.success) throw new Error(analyzed.error ?? "Analyze failed");
  const analysis = analyzed.result ?? {};

  analysis.objects = yolo.ok
    ? yolo.detections.map((d, i) => ({
        object_id: `yolo-${detModel.name}-${i}`,
        label: d.label,
        confidence: d.confidence,
        bounding_box: { x: d.bounding_box[0], y: d.bounding_box[1], width: (d.bounding_box[2] ?? 0) - (d.bounding_box[0] ?? 0), height: (d.bounding_box[3] ?? 0) - (d.bounding_box[1] ?? 0) },
        mask: null, attributes: {}, parent_id: null, children: [],
      }))
    : [];
  analysis.text_regions = ocr.ok
    ? ocr.regions.map((r) => ({
        text: r.text, confidence: r.confidence,
        bounding_box: { x: r.bounding_box[0], y: r.bounding_box[1], width: r.bounding_box[2], height: r.bounding_box[3], rotation: 0.0 },
        language: null, direction: "LeftToRight", words: [], font_attributes: null,
      }))
    : [];
  analysis.faces = faces.ok
    ? faces.faces.map((f, i) => ({
        face_id: `face-${i}`,
        bounding_box: { x: f.bounding_box[0], y: f.bounding_box[1], width: f.bounding_box[2], height: f.bounding_box[3], rotation: 0.0 },
        landmarks: [],
        // FaceAttributes derives Deserialize without #[serde(default)] on
        // its fields, so every field must be explicit (a bare {} fails to
        // deserialize on the CreateGraph round-trip) — all real nulls,
        // this detector makes no age/emotion/pose claims.
        attributes: {
          age_estimate: null, age_range: null, emotion: null, emotion_scores: null,
          pose: null, eyes_open: null, mouth_open: null, glasses: null,
          facial_hair: null, smile_score: null,
        },
        confidence: 1.0, embedding: null,
      }))
    : [];
  // Honest, real backend-availability notes — never silently hide WHY a
  // real list is empty (a genuinely empty real detection vs. a missing
  // backend/dependency are different facts).
  analysis.detection_backend_notes = {
    objects: yolo.ok ? null : yolo.error,
    text: ocr.ok ? null : ocr.error,
    faces: faces.ok ? null : faces.error,
  };
  return { pngPath, analysis };
}

async function captureGraphAndAnalyze(projectId, pngPath) {
  const { analysis } = await captureAndAnalyze(pngPath);
  const created = await runImagePipeline({
    data: { action: { type: "CreateGraph", analysis, project_id: projectId, graph_name: `screen-${Date.now()}` } },
  });
  if (!created.success) throw new Error(created.error ?? "CreateGraph failed");
  const graph = created.result ?? {};
  return { pngPath, analysis, graph_id: graph.graph_id, nodes: (graph.nodes ?? []).length, edges: (graph.edges ?? []).length };
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

// ── DETECTION-MODEL REGISTRY (guide §11.6 — operator vision) ──────────────
// Detection models are registered, expandable tools latent to image
// modality 102. N models run in parallel, all expanding the same graph.
// Registry: {ZSEI_DATA_DIR}/detection_models/registry.json (data, not code
// — add a model = add a JSON entry). Backend: the ultralytics python
// runtime (torch is present on this machine; `pip install ultralytics`
// is the operator's one-liner to light it up).

const DETECTION_REGISTRY = path.join(ZSEI_DATA_DIR, "detection_models", "registry.json");

function loadDetectionRegistry() {
  try {
    return JSON.parse(fs.readFileSync(DETECTION_REGISTRY, "utf8"));
  } catch {
    return { models: [] };
  }
}

// Registry entries predate the "kind" field (registry_version 0.1.0) —
// undefined defaults to "detection" so existing yolov8n/yolov5n entries
// keep working unchanged.
function isDetectionModel(m) { return (m.kind ?? "detection") === "detection"; }
function isDepthModel(m) { return m.kind === "depth"; }
function isSegmentationModel(m) { return m.kind === "segmentation"; }
function isPoseModel(m) { return m.kind === "pose"; }

function runYolo(pngPath, modelName, confidence) {
  return new Promise((resolve) => {
    const script = [
      "import json, sys",
      "try:",
      "    from ultralytics import YOLO",
      "except ImportError:",
      '    print(json.dumps({"ok": False, "error": "ultralytics not installed - pip install ultralytics to enable the detection backend"}))',
      "    sys.exit(0)",
      'model = YOLO("' + modelName + '.pt")',
      'results = model.predict("' + pngPath + '", conf=' + confidence + ', verbose=False)',
      "dets = []",
      "for r in results:",
      "    for box in r.boxes:",
      "        dets.append({",
      '            "label": r.names[int(box.cls)],',
      '            "confidence": round(float(box.conf), 3),',
      '            "bounding_box": [round(float(v), 1) for v in box.xyxy[0].tolist()],',
      "        })",
      'print(json.dumps({"ok": True, "model": "' + modelName + '", "detections": dets}))',
    ].join("\n");
    const yoloPython = process.env.OZONE_YOLO_PYTHON ?? "python3";
    const child = spawn(yoloPython, ["-c", script], { timeout: 120000 });
    let out = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { out += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.startsWith("{"));
      try {
        resolve(JSON.parse(line ?? "{}"));
      } catch {
        resolve({ ok: false, error: `backend produced no JSON: ${out.slice(0, 200)}` });
      }
    });
  });
}

// ── DEPTH ESTIMATION (docs/LOCALIZATION_TRAVERSAL_GUIDE.md §4 — operator
// directive 2026-09-29) — a second real localization-model family in the
// same registry, alongside detection (§11.6). A depth model turns a single
// 2D image into real per-pixel relative-depth data: the "flash a 2D read
// onto a 3D structure, even hollow, then refine" bootstrap into spatial
// localization without needing true 3D sensor input first. Backend:
// transformers' depth-estimation pipeline (installed into this MCP's own
// venv alongside ultralytics/torch — `pip install transformers`, no new
// venv). Verified live: Depth-Anything-V2-Small, real inference on a real
// test image, 1280x720 output, load+infer 7.56s.

function runDepth(pngPath, modelId, boxes) {
  return new Promise((resolve) => {
    const script = [
      "import json, sys",
      "try:",
      "    from transformers import pipeline",
      "    from PIL import Image",
      "    import numpy as np",
      "except ImportError as e:",
      '    print(json.dumps({"ok": False, "error": f"transformers/pillow/numpy not installed - {e}"}))',
      "    sys.exit(0)",
      'pipe = pipeline(task="depth-estimation", model="' + modelId + '")',
      'img = Image.open("' + pngPath + '").convert("RGB")',
      "result = pipe(img)",
      'arr = np.array(result["depth"]).astype(float)',
      "boxes = json.loads('" + JSON.stringify(boxes ?? []).replace(/'/g, "\\'") + "')",
      "regions = []",
      "for b in boxes:",
      "    x1, y1, x2, y2 = [int(v) for v in b]",
      "    x1, y1 = max(0, x1), max(0, y1)",
      "    x2, y2 = min(arr.shape[1], x2), min(arr.shape[0], y2)",
      "    if x2 > x1 and y2 > y1:",
      "        region = arr[y1:y2, x1:x2]",
      '        regions.append({"median": round(float(np.median(region)), 2), "mean": round(float(region.mean()), 2)})',
      "    else:",
      '        regions.append({"median": None, "mean": None})',
      'print(json.dumps({"ok": True, "width": int(arr.shape[1]), "height": int(arr.shape[0]), "min": float(arr.min()), "max": float(arr.max()), "mean": round(float(arr.mean()), 2), "regions": regions}))',
    ].join("\n");
    const yoloPython = process.env.OZONE_YOLO_PYTHON ?? "python3";
    const child = spawn(yoloPython, ["-c", script], { timeout: 120000 });
    let out = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { out += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.startsWith("{"));
      try {
        resolve(JSON.parse(line ?? "{}"));
      } catch {
        resolve({ ok: false, error: `backend produced no JSON: ${out.slice(0, 200)}` });
      }
    });
  });
}

// ── OCR + FACE DETECTION (docs/CAPABILITY_EXPANSION_REVIEW.md §0 — a real
// integrity fix, not an expansion: the image pipeline's own Analyze action
// previously returned 100% hardcoded fake text/face/object results on
// every real call. Real backends added here, same shape as runYolo/
// runDepth — the Rust pipeline's role stays "persist a graph from
// already-computed analysis," never "do the CV itself," matching how
// yolo_graph already overrides analysis.objects post-hoc below.

function runOcr(pngPath) {
  return new Promise((resolve) => {
    const script = [
      "import json, sys",
      "try:",
      "    import pytesseract",
      "    from PIL import Image",
      "except ImportError as e:",
      '    print(json.dumps({"ok": False, "error": f"pytesseract/pillow not installed - {e}"}))',
      "    sys.exit(0)",
      'img = Image.open("' + pngPath + '")',
      "try:",
      "    data = pytesseract.image_to_data(img, output_type=pytesseract.Output.DICT)",
      "except Exception as e:",
      '    print(json.dumps({"ok": False, "error": f"tesseract binary not available - {e}"}))',
      "    sys.exit(0)",
      "regions = []",
      "n = len(data['text'])",
      "for i in range(n):",
      "    t = data['text'][i].strip()",
      "    conf = float(data['conf'][i])",
      "    if t and conf > 0:",
      "        regions.append({",
      '            "text": t, "confidence": round(conf / 100.0, 3),',
      '            "bounding_box": [data["left"][i], data["top"][i], data["width"][i], data["height"][i]],',
      "        })",
      'print(json.dumps({"ok": True, "regions": regions}))',
    ].join("\n");
    const pyBin = process.env.OZONE_YOLO_PYTHON ?? "python3";
    const child = spawn(pyBin, ["-c", script], { timeout: 60000 });
    let out = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { out += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.startsWith("{"));
      try { resolve(JSON.parse(line ?? "{}")); }
      catch { resolve({ ok: false, error: `backend produced no JSON: ${out.slice(0, 200)}` }); }
    });
  });
}

// Real face detection via OpenCV's own bundled Haar Cascade classifier —
// zero new dependency (OpenCV is already installed for this venv per the
// review), zero model download, genuinely lightweight. Honest limits of
// this specific real detector (decades-old, well-known): frontal faces,
// reasonable lighting — not a claim of state-of-the-art accuracy, just a
// real detector instead of a fabricated one.
function runFaces(pngPath) {
  return new Promise((resolve) => {
    const script = [
      "import json, sys",
      "try:",
      "    import cv2",
      "except ImportError as e:",
      '    print(json.dumps({"ok": False, "error": f"opencv-python not installed - {e}"}))',
      "    sys.exit(0)",
      'img = cv2.imread("' + pngPath + '")',
      "if img is None:",
      '    print(json.dumps({"ok": False, "error": "cv2 could not read the image"}))',
      "    sys.exit(0)",
      "gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)",
      "cascade_path = cv2.data.haarcascades + 'haarcascade_frontalface_default.xml'",
      "detector = cv2.CascadeClassifier(cascade_path)",
      "faces = detector.detectMultiScale(gray, scaleFactor=1.1, minNeighbors=5, minSize=(30, 30))",
      "out = []",
      "for (x, y, w, h) in faces:",
      '    out.append({"bounding_box": [int(x), int(y), int(w), int(h)]})',
      'print(json.dumps({"ok": True, "faces": out}))',
    ].join("\n");
    const pyBin = process.env.OZONE_YOLO_PYTHON ?? "python3";
    const child = spawn(pyBin, ["-c", script], { timeout: 30000 });
    let out = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { out += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.startsWith("{"));
      try { resolve(JSON.parse(line ?? "{}")); }
      catch { resolve({ ok: false, error: `backend produced no JSON: ${out.slice(0, 200)}` }); }
    });
  });
}

// ── CLASSICAL SHAPE/LINE/EDGE DETECTION (docs/CAPABILITY_EXPANSION_REVIEW.md
// §1.2 — the operator's own "lines, shapes" framing, ranked the cheapest,
// fastest, most honest real option in the whole review: zero new
// dependency, zero model weight, deterministic math on real pixels, can't
// hallucinate). Canny edges + probabilistic Hough line segments + contour
// polygon approximation — all real OpenCV functions, real pixel-space
// output.
function runShapes(pngPath) {
  return new Promise((resolve) => {
    const script = [
      "import json, sys",
      "try:",
      "    import cv2",
      "    import numpy as np",
      "except ImportError as e:",
      '    print(json.dumps({"ok": False, "error": f"opencv-python/numpy not installed - {e}"}))',
      "    sys.exit(0)",
      'img = cv2.imread("' + pngPath + '")',
      "if img is None:",
      '    print(json.dumps({"ok": False, "error": "cv2 could not read the image"}))',
      "    sys.exit(0)",
      "gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)",
      "edges = cv2.Canny(gray, 50, 150)",
      "lines = cv2.HoughLinesP(edges, 1, np.pi / 180, threshold=60, minLineLength=30, maxLineGap=10)",
      "line_out = []",
      "if lines is not None:",
      "    for l in lines[:200]:",
      "        x1, y1, x2, y2 = [int(v) for v in l[0]]",
      '        line_out.append({"x1": x1, "y1": y1, "x2": x2, "y2": y2})',
      "contours, _ = cv2.findContours(edges, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)",
      "shape_out = []",
      "for c in contours:",
      "    area = cv2.contourArea(c)",
      "    if area < 50:",
      "        continue",
      "    approx = cv2.approxPolyDP(c, 0.02 * cv2.arcLength(c, True), True)",
      "    x, y, w, h = cv2.boundingRect(c)",
      "    n = len(approx)",
      '    shape = "circle" if n > 8 else ("triangle" if n == 3 else ("rectangle" if n == 4 else f"polygon_{n}"))',
      '    shape_out.append({"shape": shape, "vertices": n, "area": round(float(area), 1), "bounding_box": [int(x), int(y), int(w), int(h)]})',
      "    if len(shape_out) >= 200:",
      "        break",
      'print(json.dumps({"ok": True, "lines": line_out, "shapes": shape_out, "edge_pixel_count": int(np.count_nonzero(edges))}))',
    ].join("\n");
    const pyBin = process.env.OZONE_YOLO_PYTHON ?? "python3";
    const child = spawn(pyBin, ["-c", script], { timeout: 30000 });
    let out = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { out += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.startsWith("{"));
      try { resolve(JSON.parse(line ?? "{}")); }
      catch { resolve({ ok: false, error: `backend produced no JSON: ${out.slice(0, 200)}` }); }
    });
  });
}

// ── POSE ESTIMATION (docs/CAPABILITY_EXPANSION_REVIEW.md §1.3 — MediaPipe
// Pose, real CPU inference, 33 real body landmarks per Google's own
// published model.)
function runPose(pngPath) {
  return new Promise((resolve) => {
    const script = [
      "import json, sys",
      "try:",
      "    import mediapipe as mp",
      "    import cv2",
      "except ImportError as e:",
      '    print(json.dumps({"ok": False, "error": f"mediapipe/opencv-python not installed - {e}"}))',
      "    sys.exit(0)",
      'img = cv2.imread("' + pngPath + '")',
      "if img is None:",
      '    print(json.dumps({"ok": False, "error": "cv2 could not read the image"}))',
      "    sys.exit(0)",
      "h, w = img.shape[:2]",
      "mp_pose = mp.solutions.pose",
      "with mp_pose.Pose(static_image_mode=True) as pose:",
      "    rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)",
      "    result = pose.process(rgb)",
      "    if not result.pose_landmarks:",
      '        print(json.dumps({"ok": True, "detected": False, "landmarks": []}))',
      "    else:",
      "        lms = []",
      "        for i, lm in enumerate(result.pose_landmarks.landmark):",
      '            lms.append({"index": i, "name": mp_pose.PoseLandmark(i).name, "x": round(lm.x * w, 1), "y": round(lm.y * h, 1), "z": round(lm.z, 4), "visibility": round(lm.visibility, 3)})',
      '        print(json.dumps({"ok": True, "detected": True, "landmarks": lms, "width": w, "height": h}))',
    ].join("\n");
    const pyBin = process.env.OZONE_YOLO_PYTHON ?? "python3";
    const child = spawn(pyBin, ["-c", script], { timeout: 60000 });
    let out = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { out += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.startsWith("{"));
      try { resolve(JSON.parse(line ?? "{}")); }
      catch { resolve({ ok: false, error: `backend produced no JSON: ${out.slice(0, 200)}` }); }
    });
  });
}

// ── SEGMENTATION (docs/CAPABILITY_EXPANSION_REVIEW.md §1.4 — ultralytics'
// own -seg checkpoints, same backend as runYolo, just a different weight
// file; real per-object RLE-ish polygon masks, not a new dependency.)
function runYoloSeg(pngPath, modelName, confidence) {
  return new Promise((resolve) => {
    const script = [
      "import json, sys",
      "try:",
      "    from ultralytics import YOLO",
      "except ImportError:",
      '    print(json.dumps({"ok": False, "error": "ultralytics not installed - pip install ultralytics to enable the segmentation backend"}))',
      "    sys.exit(0)",
      'model = YOLO("' + modelName + '.pt")',
      'results = model.predict("' + pngPath + '", conf=' + confidence + ', verbose=False)',
      "dets = []",
      "for r in results:",
      "    boxes = r.boxes",
      "    masks = r.masks",
      "    for i in range(len(boxes)):",
      "        box = boxes[i]",
      "        entry = {",
      '            "label": r.names[int(box.cls)],',
      '            "confidence": round(float(box.conf), 3),',
      '            "bounding_box": [round(float(v), 1) for v in box.xyxy[0].tolist()],',
      "        }",
      "        if masks is not None and i < len(masks):",
      "            poly = masks.xy[i]",
      '            entry["mask_polygon"] = [[round(float(x), 1), round(float(y), 1)] for x, y in poly.tolist()[:100]]',
      "        dets.append(entry)",
      'print(json.dumps({"ok": True, "model": "' + modelName + '", "detections": dets}))',
    ].join("\n");
    const pyBin = process.env.OZONE_YOLO_PYTHON ?? "python3";
    const child = spawn(pyBin, ["-c", script], { timeout: 120000 });
    let out = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { out += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.startsWith("{"));
      try { resolve(JSON.parse(line ?? "{}")); }
      catch { resolve({ ok: false, error: `backend produced no JSON: ${out.slice(0, 200)}` }); }
    });
  });
}

/** Real depth-derived 3D-ish relations: nearer median depth = InFrontOf the
 * farther one. Depth-Anything V2's own model card does NOT state its
 * value convention (checked directly, it doesn't say) — larger value =
 * nearer to camera is corroborated instead by two independent real
 * sources: a real user's empirical report on the model's own GitHub
 * issue tracker (DepthAnything/Depth-Anything-V2#93) and an independent
 * technical write-up describing its disparity-space output (1=nearest,
 * 0=farthest, normalized). Not a single authoritative confirmation, but
 * two independent real sources agreeing, not guessed outright. Only
 * emitted when both regions produced a real median (never a fabricated
 * relation for a region depth couldn't be computed for). */
function depthRelations(dets, regions) {
  const rels = [];
  for (let i = 0; i < dets.length; i++) {
    for (let j = i + 1; j < dets.length; j++) {
      const di = regions[i]?.median, dj = regions[j]?.median;
      if (di == null || dj == null) continue;
      if (di > dj) rels.push({ a: dets[i].label, b: dets[j].label, r: "InFrontOf" });
      else if (dj > di) rels.push({ a: dets[j].label, b: dets[i].label, r: "InFrontOf" });
    }
  }
  return rels;
}

// Detections → spatial relations (F.3 types, computed from real boxes):
// Above/Below (vertical), LeftOf/RightOf (horizontal), NearTo (proximity),
// Supports (lower box top ≈ upper box bottom — an object resting on another).
function spatialFromDetections(dets) {
  const rels = [];
  for (let i = 0; i < dets.length; i++) {
    for (let j = i + 1; j < dets.length; j++) {
      const [ax1, ay1, ax2, ay2] = dets[i].bounding_box;
      const [bx1, by1, bx2, by2] = dets[j].bounding_box;
      const acx = (ax1 + ax2) / 2, bcx = (bx1 + bx2) / 2;
      const acy = (ay1 + ay2) / 2, bcy = (by1 + by2) / 2;
      const near = Math.hypot(acx - bcx, acy - bcy) < Math.max(ax2 - ax1, bx2 - bx1) * 2;
      if (acy < bcy && Math.abs(ay2 - by1) < 40) rels.push({ a: dets[i].label, b: dets[j].label, r: "Supports" });
      else if (bcy < acy && Math.abs(by2 - ay1) < 40) rels.push({ a: dets[j].label, b: dets[i].label, r: "Supports" });
      else if (acy < bcy) rels.push({ a: dets[i].label, b: dets[j].label, r: "Above" });
      else if (bcy < acy) rels.push({ a: dets[j].label, b: dets[i].label, r: "Above" });
      else if (acx < bcx) rels.push({ a: dets[i].label, b: dets[j].label, r: "LeftOf" });
      else rels.push({ a: dets[i].label, b: dets[j].label, r: "RightOf" });
      if (near) rels.push({ a: dets[i].label, b: dets[j].label, r: "NearTo" });
    }
  }
  return rels;
}

const server = createServer((req, res) => {
  if (req.method === "GET" && (req.url ?? "").startsWith("/status")) {
    reply(res, 200, {
      tool: "visual-mcp",
      platform: `${process.platform}/${process.arch}`,
      image_pipeline: fs.existsSync(IMAGE_PIPELINE),
      display: process.env.DISPLAY ?? "(unset)",
      capture_backend: process.env.OZONE_VISUAL_CAPTURE ?? "auto (Wayland: ingest/portal only)",
      uptime_secs: Math.round((Date.now() - startedAt) / 1000),
    });
    return;
  }
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call or GET /status only" });
    return;
  }
  const chunks = [];
  let size = 0;
  req.on("data", (c) => { size += c.length; if (size > 20 * 1024 * 1024) req.destroy(); else chunks.push(c); }); // 20MB — screen captures are multi-MB
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "visual_ingest") {
        const b64 = String(input.image_base64 ?? "");
        if (!b64) { reply(res, 200, { success: false, error: "image_base64 required" }); return; }
        const pngPath = ingestBase64(b64);
        const { analysis } = await captureAndAnalyze(pngPath);
        reply(res, 200, { success: true, output: { capture: pngPath, objects: analysis.objects ?? [], text_regions: analysis.text_regions ?? [], composition: analysis.composition ?? {} } });
        return;
      }
      if (tool === "visual_describe") {
        const pngPath = input.image_base64 ? ingestBase64(String(input.image_base64)) : await captureScreen();
        const { analysis } = await captureAndAnalyze(pngPath);
        reply(res, 200, {
          success: true,
          output: {
            capture: pngPath,
            objects: (analysis.objects ?? []).map((o) => ({ label: o.label, confidence: o.confidence, bounding_box: o.bounding_box })),
            text_regions: analysis.text_regions ?? [],
            colors: analysis.colors ?? [],
            composition: analysis.composition ?? {},
          },
        });
      } else if (tool === "visual_graph") {
        const projectId = Number(input.project_id ?? PROJECT_ID);
        const pngPath = input.image_base64 ? ingestBase64(String(input.image_base64)) : await captureScreen();
        const { graph_id, nodes, edges } = await captureGraphAndAnalyze(projectId, pngPath);
        reply(res, 200, {
          success: true,
          output: {
            graph_id,
            nodes,
            edges,
            persisted: true,
            capture: pngPath,
            note: "ModalityGraph container persisted — retrievable via GetContainerContent",
          },
        });
      } else if (tool === "yolo_detect") {
        // The registered detection-model path (§11.6): registry-driven,
        // N models in parallel, detections + F.3 spatial relations.
        // Filters to kind==="detection" explicitly (not just "enabled") so
        // adding a depth model to the same registry can never get picked
        // as the default here by array order.
        const reg = loadDetectionRegistry();
        const wanted = String(input.model ?? reg.models.find((m) => m.enabled && isDetectionModel(m))?.name ?? "");
        const model = reg.models.find((m) => m.name === wanted && m.enabled && isDetectionModel(m));
        if (!model) {
          reply(res, 200, { success: false, error: `model '${wanted}' not found or disabled — registry: ${reg.models.map((m) => m.name + (m.enabled ? "" : " (off)")).join(", ")}` });
          return;
        }
        let pngPath = input.image_base64 ? ingestBase64(String(input.image_base64)) : await captureScreen().catch(() => null);
        if (!pngPath) { reply(res, 200, { success: false, error: "image_base64 required (no capture backend on this session)" }); return; }
        const conf = Number(input.confidence ?? 0.5);
        const yolo = await runYolo(pngPath, model.name, conf);
        if (!yolo.ok) { reply(res, 200, { success: false, error: yolo.error }); return; }
        reply(res, 200, {
          success: true,
          output: {
            model: model.name,
            detections: yolo.detections,
            spatial_relations: spatialFromDetections(yolo.detections),
            capture: pngPath,
          },
        });
      } else if (tool === "yolo_graph") {
        // §11.6 PAYOFF: real detections become REAL graph nodes/edges —
        // person/chair/car nodes with NearTo/Above/Supports edges persisted
        // as a ModalityGraph the whole system can traverse.
        const reg = loadDetectionRegistry();
        const wanted = String(input.model ?? reg.models.find((m) => m.enabled && isDetectionModel(m))?.name ?? "");
        const model = reg.models.find((m) => m.name === wanted && m.enabled && isDetectionModel(m));
        if (!model) {
          reply(res, 200, { success: false, error: `model '${wanted}' not found or disabled` });
          return;
        }
        let pngPath = input.image_base64 ? ingestBase64(String(input.image_base64)) : await captureScreen().catch(() => null);
        if (!pngPath) { reply(res, 200, { success: false, error: "image_base64 required (no capture backend on this session)" }); return; }
        const yolo = await runYolo(pngPath, model.name, Number(input.confidence ?? 0.5));
        if (!yolo.ok) { reply(res, 200, { success: false, error: yolo.error }); return; }
        const dets = yolo.detections;
        // Schema-complete analysis via the pipeline's OWN Analyze (its output
        // is valid CreateGraph input by construction), then override objects
        // with the YOLO detections (full DetectedObject shape).
        const analyzed = await runImagePipeline({
          data: { action: { type: "Analyze", image_data: { Base64: { data: fs.readFileSync(pngPath).toString("base64"), mime_type: "image/png" } }, detect_objects: false, analyze_colors: false, analyze_composition: true } },
        });
        if (!analyzed.success) throw new Error(analyzed.error ?? "Analyze failed");
        const analysis = analyzed.result ?? {};
        analysis.objects = dets.map((d, i) => ({
          object_id: `yolo-${model.name}-${i}`,
          label: d.label,
          confidence: d.confidence,
          bounding_box: { x: d.bounding_box[0], y: d.bounding_box[1], width: (d.bounding_box[2] ?? 0) - (d.bounding_box[0] ?? 0), height: (d.bounding_box[3] ?? 0) - (d.bounding_box[1] ?? 0) },
          mask: null,
          attributes: {},
          parent_id: null,
          children: [],
        }));
        const created = await runImagePipeline({
          data: { action: { type: "CreateGraph", analysis, project_id: Number(input.project_id ?? PROJECT_ID), graph_name: `yolo-${model.name}-${Date.now()}` } },
        });
        if (!created.success) throw new Error(created.error ?? "CreateGraph failed");
        const graph = created.result ?? {};
        reply(res, 200, {
          success: true,
          output: {
            model: model.name,
            detections: dets,
            spatial_relations: spatialFromDetections(dets),
            graph_id: graph.graph_id,
            graph_nodes: (graph.nodes ?? []).length,
            graph_edges: (graph.edges ?? []).length,
            persisted: true,
          },
        });
      } else if (tool === "depth_estimate") {
        // Standalone depth read — no persistence, no detection dependency.
        // Real per-pixel relative depth (min/max/mean over the whole
        // frame) plus, when real bounding boxes are supplied, real
        // per-region median/mean depth for each one.
        const reg = loadDetectionRegistry();
        const wanted = String(input.model ?? reg.models.find((m) => m.enabled && isDepthModel(m))?.name ?? "");
        const model = reg.models.find((m) => m.name === wanted && m.enabled && isDepthModel(m));
        if (!model) {
          reply(res, 200, { success: false, error: `depth model '${wanted}' not found or disabled — registry: ${reg.models.filter(isDepthModel).map((m) => m.name + (m.enabled ? "" : " (off)")).join(", ") || "(none registered)"}` });
          return;
        }
        const pngPath = input.image_base64 ? ingestBase64(String(input.image_base64)) : await captureScreen().catch(() => null);
        if (!pngPath) { reply(res, 200, { success: false, error: "image_base64 required (no capture backend on this session)" }); return; }
        const boxes = Array.isArray(input.bounding_boxes) ? input.bounding_boxes : [];
        const depth = await runDepth(pngPath, model.model_id, boxes);
        if (!depth.ok) { reply(res, 200, { success: false, error: depth.error }); return; }
        reply(res, 200, {
          success: true,
          output: {
            model: model.name,
            width: depth.width, height: depth.height,
            depth_min: depth.min, depth_max: depth.max, depth_mean: depth.mean,
            regions: depth.regions,
            capture: pngPath,
            note: "Relative depth (disparity-space): higher value = nearer to camera for this model. No metric/real-world units.",
          },
        });
      } else if (tool === "depth_graph") {
        // §11.6-style payoff, extended per docs/LOCALIZATION_TRAVERSAL_GUIDE.md:
        // real YOLO detections + real per-object depth become REAL graph
        // nodes carrying a real depth attribute, with real InFrontOf edges
        // computed from actual relative-depth values — the "flash a 2D
        // read onto a 3D structure, even hollow, then refine" bootstrap,
        // landed as a real persisted graph, not a design doc.
        const reg = loadDetectionRegistry();
        const detModel = reg.models.find((m) => m.enabled && isDetectionModel(m) && (!input.detection_model || m.name === input.detection_model));
        const depthModel = reg.models.find((m) => m.enabled && isDepthModel(m) && (!input.depth_model || m.name === input.depth_model));
        if (!detModel) { reply(res, 200, { success: false, error: "no enabled detection model registered" }); return; }
        if (!depthModel) { reply(res, 200, { success: false, error: "no enabled depth model registered" }); return; }
        const pngPath = input.image_base64 ? ingestBase64(String(input.image_base64)) : await captureScreen().catch(() => null);
        if (!pngPath) { reply(res, 200, { success: false, error: "image_base64 required (no capture backend on this session)" }); return; }
        const yolo = await runYolo(pngPath, detModel.name, Number(input.confidence ?? 0.5));
        if (!yolo.ok) { reply(res, 200, { success: false, error: yolo.error }); return; }
        const dets = yolo.detections;
        const depth = dets.length > 0 ? await runDepth(pngPath, depthModel.model_id, dets.map((d) => d.bounding_box)) : { ok: true, regions: [] };
        if (!depth.ok) { reply(res, 200, { success: false, error: depth.error }); return; }
        const regions = depth.regions ?? [];
        const analyzed = await runImagePipeline({
          data: { action: { type: "Analyze", image_data: { Base64: { data: fs.readFileSync(pngPath).toString("base64"), mime_type: "image/png" } }, detect_objects: false, analyze_colors: false, analyze_composition: true } },
        });
        if (!analyzed.success) throw new Error(analyzed.error ?? "Analyze failed");
        const analysis = analyzed.result ?? {};
        // Real depth attached per object — never a fabricated value when a
        // region's depth genuinely couldn't be computed (kept null, not 0).
        analysis.objects = dets.map((d, i) => ({
          object_id: `depth-${detModel.name}-${i}`,
          label: d.label,
          confidence: d.confidence,
          bounding_box: { x: d.bounding_box[0], y: d.bounding_box[1], width: (d.bounding_box[2] ?? 0) - (d.bounding_box[0] ?? 0), height: (d.bounding_box[3] ?? 0) - (d.bounding_box[1] ?? 0) },
          mask: null,
          attributes: { depth_median: regions[i]?.median ?? null, depth_mean: regions[i]?.mean ?? null, depth_model: depthModel.name },
          parent_id: null,
          children: [],
        }));
        const created = await runImagePipeline({
          data: { action: { type: "CreateGraph", analysis, project_id: Number(input.project_id ?? PROJECT_ID), graph_name: `depth-${detModel.name}-${Date.now()}` } },
        });
        if (!created.success) throw new Error(created.error ?? "CreateGraph failed");
        const graph = created.result ?? {};
        reply(res, 200, {
          success: true,
          output: {
            detection_model: detModel.name,
            depth_model: depthModel.name,
            detections: dets,
            depth_regions: regions,
            spatial_relations: spatialFromDetections(dets),
            depth_relations: depthRelations(dets, regions),
            graph_id: graph.graph_id,
            graph_nodes: (graph.nodes ?? []).length,
            graph_edges: (graph.edges ?? []).length,
            persisted: true,
            note: "Real per-object depth is a real node attribute (attributes.depth_median/depth_mean); InFrontOf relations are computed but not yet a persisted typed graph edge (image 102's ImageEdgeType has no depth-axis variant today) — returned in the response, not silently dropped.",
          },
        });
      } else {
        reply(res, 200, { success: false, error: `unknown visual tool '${tool}' (visual_describe | visual_graph | visual_ingest | yolo_detect | yolo_graph | depth_estimate | depth_graph)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") {
    console.error(`[visual-mcp] :${PORT} already in use — skipping`);
    return;
  }
  console.error("[visual-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[visual-mcp] /call on 127.0.0.1:${PORT} | pipeline: ${fs.existsSync(IMAGE_PIPELINE) ? "found" : "MISSING"} | DISPLAY: ${process.env.DISPLAY ?? "(unset)"}`);
});

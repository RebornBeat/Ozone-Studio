/**
 * F1 — real workspace/project/file-reference data, over the generic
 * `/zsei/query` surface (B12/B13). Container shapes confirmed directly
 * against `assets/pipelines/general/{file_link,url_link,package_link}/main.rs`
 * (`link_reference_to_graph`/`create_file_ref`/`create_url_ref`/the package
 * info builder), NOT guessed:
 *
 *   - FileReference:    `metadata.name` = `"File: {path}"`,
 *                        `context.keywords` = `[filename.lower(), modality.lower()]`
 *   - URLReference:     `metadata.name` = `"URL: {url}"`,
 *                        `context.keywords` = `[domain.lower()]`
 *   - PackageReference: `metadata.name` = `"Package: {registry}/{name}"`,
 *                        `context.keywords` = `[name.lower(), registry.lower()]`
 *
 * `metadata.modality` on these containers is ALWAYS the literal string
 * `"Unknown"` (set that way at creation time, container.rs's `Modality` enum
 * has no per-reference-type variant here) — the real per-file modality for a
 * FileReference only lives in `keywords[1]`, nowhere else. `object_store_path`
 * is always `null` on these containers too — no content is stored on them;
 * real file content is `data/fileContent.ts` (F0)'s job, not this module's.
 *
 * Deliberately NOT `workspace_tab`'s disconnected flat-JSON file list (A5) —
 * these are the real graph-native containers, bidirectionally linked to
 * their project via a real `Relation` edge (`Contains`/`PartOf`).
 */
import { zseiQuery } from "../ozoneClient";

export interface WorkspaceSummary {
  containerId: number;
  name: string;
  projectIds: number[];
}
export interface ProjectSummary {
  containerId: number;
  name: string;
  workspaceId: number;
}
export type FileRefKind = "file" | "url" | "package";
export interface FileRef {
  containerId: number;
  kind: FileRefKind;
  name: string;
  path?: string;
  url?: string;
  modality?: string;
  keywords: string[];
  /** Untouched container for anything not promoted to a named field. */
  raw: unknown;
}

interface RawContainer {
  global_state: { container_id: number; child_ids: number[] };
  local_state: {
    metadata: { container_type: string; name?: string | null };
    context: { keywords: string[] };
  };
}

async function getContainer(containerId: number): Promise<RawContainer | null> {
  const result = await zseiQuery<any>({ GetContainer: { container_id: containerId } });
  return (result?.Container as RawContainer | undefined) ?? null;
}

function stripPrefix(name: string, prefix: string): string | null {
  return name.startsWith(prefix) ? name.slice(prefix.length) : null;
}

function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}

/** Extension fallback for legacy FileReference containers whose keywords lack
 * the modality entry. Lower-case bucket names, matching what file_link now
 * writes to keywords[1] (shared table assets/pipelines/shared/modality_ext.rs,
 * 2026-11-01). Deliberately small: anything unlisted stays undefined (honest
 * unknown), never guessed. */
const EXT_FALLBACK: Record<string, string> = {
  html: "uidocument", htm: "uidocument", xhtml: "uidocument", css: "uidocument",
  scss: "uidocument", sass: "uidocument", less: "uidocument", svg: "uidocument",
  pdf: "text", doc: "text", docx: "text", rtf: "text", odt: "text",
  txt: "text", md: "text", rst: "text",
  rs: "code", py: "code", js: "code", ts: "code", tsx: "code", jsx: "code",
  json: "code", yaml: "code", yml: "code", toml: "code", xml: "code",
  png: "image", jpg: "image", jpeg: "image", gif: "image", webp: "image", bmp: "image",
  mp3: "audio", wav: "audio", flac: "audio", ogg: "audio",
  mp4: "video", mov: "video", mkv: "video", webm: "video", avi: "video",
  csv: "data", tsv: "data", zip: "archive", tar: "archive", gz: "archive",
  xls: "document", xlsx: "document", ppt: "document", pptx: "document",
};

function modalityForFileRef(keywords: string[], path: string | null): string | undefined {
  const kw = keywords[1];
  if (kw && kw !== "unknown") return kw;
  if (!path) return undefined;
  const dot = path.lastIndexOf(".");
  return dot >= 0 ? EXT_FALLBACK[path.slice(dot + 1).toLowerCase()] : undefined;
}

const KIND_BY_CONTAINER_TYPE: Record<string, FileRefKind> = {
  FileReference: "file",
  URLReference: "url",
  PackageReference: "package",
};

function toFileRef(containerId: number, c: RawContainer): FileRef | null {
  const kind = KIND_BY_CONTAINER_TYPE[c.local_state.metadata.container_type];
  if (!kind) return null;
  const rawName = c.local_state.metadata.name ?? "";
  const keywords = c.local_state.context.keywords ?? [];

  if (kind === "file") {
    const path = stripPrefix(rawName, "File: ");
    return {
      containerId,
      kind,
      name: path ? basename(path) : rawName || `File ${containerId}`,
      path: path ?? undefined,
      // Real per-file modality lives ONLY here (see file header) — never
      // read metadata.modality for these containers, it's always "Unknown".
      modality: modalityForFileRef(keywords, path),
      keywords,
      raw: c,
    };
  }
  if (kind === "url") {
    const url = stripPrefix(rawName, "URL: ");
    return {
      containerId,
      kind,
      name: url ?? (rawName || `URL ${containerId}`),
      url: url ?? undefined,
      keywords,
      raw: c,
    };
  }
  // package
  const label = stripPrefix(rawName, "Package: ");
  return {
    containerId,
    kind,
    name: label ?? (rawName || `Package ${containerId}`),
    keywords,
    raw: c,
  };
}

export async function loadWorkspaces(userId: number): Promise<WorkspaceSummary[]> {
  const ws = await zseiQuery<any>({ GetUserWorkspaces: { user_id: userId } });
  const ids: number[] = ws?.Containers ?? [];
  const out: WorkspaceSummary[] = [];
  for (const id of ids) {
    const c = await getContainer(id);
    if (!c) continue;
    const projectIds: number[] = [];
    for (const childId of c.global_state.child_ids) {
      const child = await getContainer(childId);
      if (child?.local_state.metadata.container_type === "Project") projectIds.push(childId);
    }
    out.push({ containerId: id, name: c.local_state.metadata.name || `Workspace ${id}`, projectIds });
  }
  return out;
}

export async function loadProjects(workspaceId: number): Promise<ProjectSummary[]> {
  const c = await getContainer(workspaceId);
  if (!c) return [];
  const out: ProjectSummary[] = [];
  for (const childId of c.global_state.child_ids) {
    const child = await getContainer(childId);
    if (child?.local_state.metadata.container_type !== "Project") continue;
    out.push({ containerId: childId, name: child.local_state.metadata.name || `Project ${childId}`, workspaceId });
  }
  return out;
}

export async function loadProjectFileRefs(projectId: number): Promise<FileRef[]> {
  const c = await getContainer(projectId);
  if (!c) return [];
  const out: FileRef[] = [];
  for (const childId of c.global_state.child_ids) {
    const child = await getContainer(childId);
    if (!child) continue;
    const ref = toFileRef(childId, child);
    if (ref) out.push(ref);
  }
  return out;
}

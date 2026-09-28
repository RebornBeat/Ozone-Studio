import { OZONE_HOST } from "../ozoneClient";

/** GET a host path — Electron bridge first (renderer on file:// can't fetch localhost), direct fetch fallback. */
export async function getJson<T>(path: string): Promise<T> {
  const oz = (window as any).ozone;
  if (oz?.http?.get) return (await oz.http.get(path)) as T;
  const res = await fetch(`${OZONE_HOST}${path}`);
  if (!res.ok) throw new Error(`ozone host ${path} → HTTP ${res.status}`);
  return (await res.json()) as T;
}

/** Build a query string, skipping undefined/null values. */
export function qs(params: Record<string, string | number | boolean | null | undefined>): string {
  const parts: string[] = [];
  for (const [k, v] of Object.entries(params)) {
    if (v === undefined || v === null || v === "") continue;
    parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
  }
  return parts.length ? `?${parts.join("&")}` : "";
}

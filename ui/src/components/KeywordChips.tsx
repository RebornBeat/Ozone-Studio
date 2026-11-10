/**
 * KeywordChips — one way to list a container's keywords everywhere (docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §3, §17, §24).
 *
 * Two different things share the `keywords` field and must not look alike:
 *   • MARKERS (`ns:value`, no whitespace — `scope:global`, `proj:3`, `ws:1`, `kind:tool_call`, `agent:sensor-tick`, `node:element`, `tool:…`,
 *     `span:run`, `loop:…`, `session:…`): scope / provenance data. They never take part in a content search against meaning terms. Shown as
 *     muted "scope" chips under their own label.
 *   • MEANING keywords: what a content search matches. When the container's `keyword_meta` is known, each shows its validation state as a TEXT
 *     label — "validated" (the gate / a review said yes), "rejected" (said no; normally already removed), or "pending" (not judged yet).
 *     Yes / no / pending only — no scores, no confidence numbers.
 * The marker rule mirrors src/shared_contracts/keyword_model.rs `is_marker`: one token, a ":" that is neither first nor last.
 */
import React from "react";

export interface KeywordMetaEntry {
  validated?: boolean | null;
  source?: string;
}

export interface KeywordChipsProps {
  keywords: readonly string[];
  /** `context.keyword_meta` when the caller has it (absent = validation state unknown, no state label is shown) */
  meta?: Readonly<Record<string, KeywordMetaEntry | undefined>> | null;
}

/** one token, a ":" that is neither the first nor the last character */
export function isMarkerKeyword(term: string): boolean {
  const t = term.trim();
  if (t === "" || /\s/.test(t)) return false;
  const i = t.indexOf(":");
  return i >= 1 && i < t.length - 1;
}

type Validation = "validated" | "rejected" | "pending";

function validationOf(meta: KeywordChipsProps["meta"], term: string): Validation | null {
  if (!meta) return null;
  const m = meta[term];
  if (m === undefined || m === null) return "pending";
  if (m.validated === true) return "validated";
  if (m.validated === false) return "rejected";
  return "pending";
}

export const KeywordChips: React.FC<KeywordChipsProps> = ({ keywords, meta }) => {
  const list: string[] = Array.isArray(keywords) ? keywords.map((k) => String(k)) : [];
  const markers = list.filter(isMarkerKeyword);
  const meaning = list.filter((k) => !isMarkerKeyword(k));
  if (list.length === 0) return null;
  return (
    <span className="okw">
      {meaning.length > 0 && (
        <span className="okw-group" aria-label="keywords">
          {meaning.map((k, i) => {
            const v = validationOf(meta, k);
            const src: string | undefined = meta ? meta[k]?.source : undefined;
            return (
              <span key={`${k}:${i}`} className={`okw-chip${v ? ` okw-${v}` : ""}`} title={v ? `${v}${src ? ` · source: ${src}` : ""}` : undefined}>
                {k}
                {v ? <span className="okw-state"> {v}</span> : null}
              </span>
            );
          })}
        </span>
      )}
      {markers.length > 0 && (
        <span className="okw-group okw-scope" aria-label="scope and provenance markers">
          <span className="okw-label">scope</span>
          {markers.map((k, i) => (
            <span key={`${k}:${i}`} className="okw-chip okw-marker" title="scope / provenance marker — not a meaning keyword">{k}</span>
          ))}
        </span>
      )}
    </span>
  );
};

export default KeywordChips;

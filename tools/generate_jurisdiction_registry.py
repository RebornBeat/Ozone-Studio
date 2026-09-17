#!/usr/bin/env python3
"""Generate the jurisdiction source registry from the real, currently-shipped
content under assets/jurisdiction/ — never hand-typed, always regenerated
from the actual files, so it can't silently drift out of sync as content
grows. Run this after any jurisdiction content change:

    python3 tools/generate_jurisdiction_registry.py

Produces:
  assets/jurisdiction/SOURCE_REGISTRY.md    (human-readable, grouped by scope)
  assets/jurisdiction/SOURCE_REGISTRY.json  (machine-readable index)

Also mirrors both into target/release/zsei_data/jurisdiction/ when that
directory exists, matching the established mirror convention for all other
jurisdiction content this session.
"""
import json
import os
import sys
from pathlib import Path
from datetime import datetime, timezone

REPO_ROOT = Path(__file__).resolve().parent.parent
JURISDICTION_DIR = REPO_ROOT / "assets" / "jurisdiction"
MIRROR_DIR = REPO_ROOT / "target" / "release" / "zsei_data" / "jurisdiction"


def load_jurisdiction_files():
    files = [JURISDICTION_DIR / "global.json"]
    national_dir = JURISDICTION_DIR / "national"
    if national_dir.is_dir():
        files.extend(sorted(national_dir.glob("*.json")))
    out = []
    for path in files:
        if not path.is_file():
            continue
        try:
            data = json.loads(path.read_text())
        except json.JSONDecodeError as e:
            print(f"WARN: {path} failed to parse: {e}", file=sys.stderr)
            continue
        out.append((path, data))
    return out


def scope_label(path: Path) -> str:
    if path.name == "global.json":
        return "global"
    return path.stem  # e.g. "jp" for national/jp.json


def build_registry():
    files = load_jurisdiction_files()
    entries = []
    scopes = {}
    gap_no_official = []
    gap_no_snippet = []
    non_log_actions = []

    for path, data in files:
        scope = scope_label(path)
        rules = data.get("rules", [])
        disclaimer_present = bool(data.get("disclaimer", "").strip())
        scope_entry = scopes.setdefault(scope, {
            "rule_count": 0,
            "disclaimer_present": disclaimer_present,
            "topics": [],
        })
        for rule in rules:
            condition = rule.get("condition", "")
            action = rule.get("action", "")
            source = rule.get("source", "")
            official = rule.get("official_source")
            snippet = rule.get("retrieved_snippet")

            entry = {
                "scope": scope,
                "condition": condition,
                "action": action,
                "source": source,
                "official_source": official,
                "has_retrieved_snippet": bool(snippet),
            }
            entries.append(entry)
            scope_entry["rule_count"] += 1
            scope_entry["topics"].append(condition)

            if official is not True:
                gap_no_official.append(entry)
            if not snippet:
                gap_no_snippet.append(entry)
            if scope != "global" and action != "Log":
                non_log_actions.append(entry)

    return entries, scopes, {
        "no_official_source": gap_no_official,
        "no_retrieved_snippet": gap_no_snippet,
        "non_log_national_actions": non_log_actions,
    }


def write_json(entries, scopes, gaps, out_path: Path):
    payload = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "generator": "tools/generate_jurisdiction_registry.py",
        "total_scopes": len(scopes),
        "total_rules": len(entries),
        "scopes": scopes,
        "entries": entries,
        "gaps": {k: len(v) for k, v in gaps.items()},
    }
    out_path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n")


def write_markdown(entries, scopes, gaps, out_path: Path):
    lines = []
    lines.append("# Jurisdiction Source Registry")
    lines.append("")
    lines.append(
        "Generated from real, currently-shipped content under `assets/jurisdiction/` "
        "by `tools/generate_jurisdiction_registry.py` — do not hand-edit, regenerate "
        "instead. This is the \"approved sources\" list: before sourcing new content "
        "for a country/topic, check here first for an already-cited, already-used "
        "source for that scope."
    )
    lines.append("")
    lines.append(f"Generated: {datetime.now(timezone.utc).isoformat()}")
    lines.append(f"Total scopes: {len(scopes)} · Total rules: {len(entries)}")
    lines.append("")
    lines.append("## Gap summary")
    lines.append("")
    lines.append(
        f"- {len(gaps['no_official_source'])} rules where `official_source` is not "
        f"confirmed `true` (not necessarily wrong — many are legitimately hand-curated "
        f"or a reputable secondary source per method 21 — but worth a look before "
        f"treating as a primary official citation)."
    )
    lines.append(
        f"- {len(gaps['no_retrieved_snippet'])} rules with no `retrieved_snippet` "
        f"recorded (older content that predates this field being populated, or "
        f"hand-curated content like global.json that cites an article directly)."
    )
    lines.append(
        f"- {len(gaps['non_log_national_actions'])} national-scope rules with a "
        f"non-Log action (should be 0 — national content is always Log-only per "
        f"method 21; global.json is the sole documented exception and is excluded "
        f"from this check)."
    )
    lines.append("")
    lines.append("## By scope")
    lines.append("")
    for scope in sorted(scopes.keys()):
        info = scopes[scope]
        lines.append(f"### {scope}")
        lines.append("")
        lines.append(
            f"{info['rule_count']} rule(s) · disclaimer present: {info['disclaimer_present']}"
        )
        lines.append("")
        lines.append("| Topic | Action | Source | Official | Snippet |")
        lines.append("|---|---|---|---|---|")
        for e in entries:
            if e["scope"] != scope:
                continue
            official = "yes" if e["official_source"] is True else (
                "no" if e["official_source"] is False else "—"
            )
            snippet = "yes" if e["has_retrieved_snippet"] else "no"
            source_cell = e["source"].replace("|", "\\|")
            lines.append(
                f"| {e['condition']} | {e['action']} | {source_cell} | {official} | {snippet} |"
            )
        lines.append("")
    out_path.write_text("\n".join(lines) + "\n")


def main():
    entries, scopes, gaps = build_registry()
    md_path = JURISDICTION_DIR / "SOURCE_REGISTRY.md"
    json_path = JURISDICTION_DIR / "SOURCE_REGISTRY.json"
    write_markdown(entries, scopes, gaps, md_path)
    write_json(entries, scopes, gaps, json_path)
    print(f"Wrote {md_path} ({len(entries)} rules, {len(scopes)} scopes)")
    print(f"Wrote {json_path}")

    if MIRROR_DIR.is_dir():
        import shutil
        shutil.copy(md_path, MIRROR_DIR / "SOURCE_REGISTRY.md")
        shutil.copy(json_path, MIRROR_DIR / "SOURCE_REGISTRY.json")
        print(f"Mirrored both into {MIRROR_DIR}")

    print("")
    print("Gap summary:")
    for k, v in gaps.items():
        print(f"  {k}: {len(v)}")


if __name__ == "__main__":
    main()

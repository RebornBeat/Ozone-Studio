#!/usr/bin/env python3
"""Preview / apply a sync of shipped jurisdiction content into a data dir.

The host only copies a shipped file into <data_dir>/jurisdiction/ when the
file is absent (older binaries) or, with the 2026-10-27 build, when the data-dir
copy is unmodified since the last sync (blake3 manifest) or
OZONE_JURISDICTION_FORCE_SYNC=1 is set. This script lets an operator see what
differs and apply it WITHOUT a rebuild or restart-time flag:

    python3 tools/sync_jurisdiction_content.py                      # dry run
    python3 tools/sync_jurisdiction_content.py --apply              # copy NEW files only
    python3 tools/sync_jurisdiction_content.py --apply --overwrite  # also replace differing
                                                                    # files (old copy -> .bak)

Default data dir is target/release/zsei_data (the local host's). Pass
--data-dir for another (e.g. a copy pulled from the EU VPS). Existing
operator elevations live in BOTH places (assets + data dir copy); review the
"DIFFERS" list before using --overwrite, because a differing file may carry an
operator edit that is not in assets/.

New files only become rule sets after the host restarts (registration happens
at boot). Existing rule sets pick up changed file contents on the next
orchestration request (rules are re-read from the file at stage 0).
"""
import argparse
import hashlib
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "assets" / "jurisdiction"


def digest(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("--data-dir", default=str(REPO / "target" / "release" / "zsei_data"))
    ap.add_argument("--apply", action="store_true", help="actually copy (default: dry run)")
    ap.add_argument("--overwrite", action="store_true", help="with --apply: replace differing files (old copy saved as .bak)")
    args = ap.parse_args()

    dst_root = Path(args.data_dir) / "jurisdiction"
    if not SRC.is_dir():
        print(f"missing source dir {SRC}", file=sys.stderr)
        return 2

    new, same, differs, copied, replaced = [], [], [], 0, 0
    for src in sorted(p for p in SRC.rglob("*") if p.is_file()):
        rel = src.relative_to(SRC)
        if rel.name.startswith(".") or rel.suffix not in (".json", ".md"):
            continue
        dst = dst_root / rel
        if not dst.exists():
            new.append(rel)
            if args.apply:
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src, dst)
                copied += 1
        elif digest(src) == digest(dst):
            same.append(rel)
        else:
            differs.append(rel)
            if args.apply and args.overwrite:
                shutil.copy2(dst, dst.with_name(dst.name + ".bak"))
                shutil.copy2(src, dst)
                replaced += 1

    print(f"data dir: {dst_root}")
    print(f"NEW      ({len(new)}): {', '.join(str(r) for r in new[:12])}{' ...' if len(new) > 12 else ''}")
    print(f"IDENTICAL({len(same)})")
    print(f"DIFFERS  ({len(differs)}): {', '.join(str(r) for r in differs[:12])}{' ...' if len(differs) > 12 else ''}")
    if args.apply:
        print(f"copied {copied} new file(s); replaced {replaced} differing file(s) (.bak kept)")
    else:
        print("dry run — pass --apply to copy NEW files, add --overwrite to replace DIFFERS")
    return 0


if __name__ == "__main__":
    sys.exit(main())

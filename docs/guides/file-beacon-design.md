# File Beacon Design (R2)

Full findings and design from the read-only review fork dispatched to
design a real, non-intrusive file-change notification mechanism for
files registered around a workspace/project. Back to [README](README.md) ·
[master guide](living-graph-field-guide.md).

## The ask, precisely

Files registered to a workspace/project should notify the live graph when
they change on disk — a plain external edit, not an API call. It should be
**non-intrusive**: explicitly *not* like `~/Projects/MetaStrata`'s
`.stratum` model (confirmed via its own README: original file +
`metadata.json` + `provenance.json` + `tags.json` + `manifest.json`, all
inside a ZIP wrapper, plus a whole Scheme Bank/Feature Bank system on top —
"way too much" per the person who asked for this). The request: one beacon,
and the whole live graph moves — nothing wrapped around the user's real
file at all.

## What's real today

- `FileRefInfo` (`assets/pipelines/general/file_link/main.rs:185-192`)
  already stores a real `modified: u64` mtime, set at registration time by
  `get_file_metadata()` (`file_link/main.rs:331-345`).
- A second real re-check site exists at `file_link/main.rs:637-641` — but
  it's a manual refresh path, not a background poll. Worth confirming
  exactly what UI action triggers it before building on top of it.
- Two real precedents for a boot-spawned background loop already exist in
  this codebase:
  - `src/zsei/hooks.rs:279` — a plain `tokio::time::interval` loop.
  - `src/lib.rs:1037` and `:1050` — spawn
    `amt_loop::run_amt_reexpansion_loop` and
    `amt_loop::spawn_graph_ripple_sync` respectively at boot
    (`amt_loop.rs:45-62`), a hub-subscriber consumer pattern. This is the
    exact wiring convention a new file-watcher loop would follow.
- **No filesystem-watching crate exists anywhere in the workspace.** Every
  `Cargo.toml` in the repo was checked directly — no `notify` crate or
  equivalent. This is the one genuinely new dependency this design needs.

## The good news: the ripple mechanism already works

`ZSEIQuery::UpdateContainer` (`src/types/zsei.rs:198`) already ripples
correctly — confirmed as this session's own earlier fix (referred to
elsewhere as "B18"). A detected file change only needs to become **one**
real `UpdateContainer` call touching the changed fields. No new event
type is needed, no new WebSocket plumbing — the beacon is entirely a
detection problem, not a propagation problem.

## Proposed shape

A new `src/file_beacon.rs`, boot-spawned the same way as `amt_loop` —
right next to its two existing spawns in `src/lib.rs`.

On a real interval (start at 30–60 seconds; tune from measured cost, never
leave a guessed constant untuned — per this project's own methodology-38
doctrine of measuring rather than guessing):

1. Walk a small cache of `{container_id → last-known mtime/size}` for
   every live file-reference container. Built once at boot the same way
   this session's own UI data layer already does it: `GetContainer{project_id}`
   → `child_ids`.
2. Stat each real path.
3. Any real delta (mtime or size changed) fires one real `UpdateContainer`
   call, touching only the storage fields that actually changed. Never
   fabricate content — if the file's content itself needs re-reading,
   that's a separate, explicit step, not assumed from a changed mtime.

## Where "non-intrusive" actually lives

The cache stays entirely in the host's own state — in memory, or a new
`zsei_data/file_beacon_cache.json` rebuilt at boot from real containers —
**never** a file next to the user's real file, never a wrapping archive,
the original file never opened for writing or renamed.

This is the real, load-bearing difference from MetaStrata's model: not a
*lighter* wrapper, but no wrapper at all. The beacon lives entirely on
Ozone-Studio's side of the relationship; the user's file is only ever
read (for a `stat`, never a full read unless content-hashing is later
adopted — see open questions below).

## Open questions — deliberately not decided here

This design intentionally stops short of deciding these; they need live
tuning or a separate scope decision, not a guess baked in now:

1. **The real poll interval.** 30–60s is a starting point, not a
   recommendation — needs tuning against real usage once running.
2. **mtime/size vs. hashing.** mtime/size is cheap but can false-negative
   (a save that touches mtime without changing content) or, depending on
   the editor, false-positive. Whether hashing is worth the added I/O cost
   is an open trade-off.
3. **Chat `attached_files`.** Whether this mechanism should ever reach
   into a chat request's attached files is out of scope for this design —
   those aren't `FileReference` containers today, and pulling them in is a
   separate scope decision, not an extension of this one.

## Implementation checklist — DONE, 2026-09-29

- [x] Decided: plain 45s interval + `stat`, no new dependency (`notify`
      crate not needed — the interval-vs-cost trade-off resolved in favor
      of the simpler option; can revisit if 45s proves too coarse once
      there's real usage to tune against).
- [x] `src/file_beacon.rs`: boot-spawned cache build + interval loop.
- [x] Wired into `src/lib.rs` next to the two existing `amt_loop` spawns.
- [x] One real `UpdateContainer` call per real detected delta (bumps
      `metadata.updated_at`, read-modify-write since the `ContainerUpdate`
      contract replaces sub-structs wholesale, not a sparse patch).
- [x] Verified live: created a real `FileReference` container (id 40380,
      via `/zsei/query`'s `CreateContainer` directly — `GetFileReferences`
      turned out to be schema-only, no handler anywhere), let the beacon
      establish a baseline on its first poll, touched the real file
      externally, confirmed on the next poll that `updated_at` genuinely
      advanced and `version` incremented 1→2 — a database-level fact, not
      just a log line.

One design assumption corrected during implementation: this doc originally
cited `FileRefInfo`'s real `modified: u64` field as existing state to
build on — that field is real, but it lives in `file_link`'s own separate
flat-file store (`load_file_refs`), not on the ZSEI container itself. A
`FileReference` container has no structured mtime/size field at all
(confirmed directly against `link_reference_to_graph` — `object_store_path`
is null, only `metadata.name` holds anything real). The beacon's own
in-memory cache is the only place mtime/size is tracked; this doesn't
change the design's "non-intrusive" property (still nothing written next
to the user's file, still in-memory only), just corrects which existing
piece of state the beacon does and doesn't build on.

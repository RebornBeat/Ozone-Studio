#!/usr/bin/env bash
# Third-party verification run (outside client, public HTTP surface only). NOT part of the build.
# usage: tools/third-party-verify/run-all.sh [script ...]   default: 01 02 03 04 05 07  (06-walk is opt-in: OZONE_TPV_WALK=1)
cd "$(dirname "$0")/../.." || exit 1
OUT=docs/evidence/third-party-verify; mkdir -p "$OUT"
S=("$@"); [ ${#S[@]} -eq 0 ] && S=(01-surface 02-work-order 03-gaps-and-refinement 04-call-graphs 05-marketplace-views 07-tools-and-coordination)
rc=0
for s in "${S[@]}"; do timeout "${SUITE_TIMEOUT:-900}" node "tools/third-party-verify/$s.mjs" || rc=1; done
exit $rc

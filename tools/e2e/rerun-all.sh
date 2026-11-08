#!/usr/bin/env bash
# E2E RERUN (operator 2026-11-06): run every suite against the live host, capture each result, write one summary.
# usage: tools/e2e/rerun-all.sh [suite ...]   (default: the full ordered list)  — needs the host up on :50051
cd "$(dirname "$0")/../.." || exit 1
OUT=docs/evidence/test-runs; mkdir -p "$OUT"
STAMP=$(date -u +%F)
SUMMARY="$OUT/$STAMP-rerun-summary.md"
SUITES=("$@"); [ ${#SUITES[@]} -eq 0 ] && SUITES=(modalities mcp-contract jurisdiction marketplace fanout meta-ripple methodology-sufficiency provider-step keyword-program span-walk walk-completion)
{
  echo "# E2E rerun summary — $STAMP (started $(date -u +%T)Z)"
  echo; echo "host: $(curl -s -m5 localhost:50051/health | head -c 120)"
  echo; echo "| suite | result | seconds | log |"; echo "|---|---|---|---|"
} > "$SUMMARY"
for s in "${SUITES[@]}"; do
  t0=$(date +%s); LOG="$OUT/$STAMP-rerun-$s.log"
  timeout "${SUITE_TIMEOUT:-3000}" node "tools/e2e/$s.mjs" > "$LOG" 2>&1; rc=$?
  line=$(grep -E "^e2e-.*: PASS" "$LOG" | tail -1)
  [ -z "$line" ] && line="no summary line (exit $rc)"
  echo "| $s | ${line#*: } | $(( $(date +%s) - t0 )) | $LOG |" >> "$SUMMARY"
done
echo >> "$SUMMARY"; echo "finished $(date -u +%T)Z" >> "$SUMMARY"
echo DONE >> "$SUMMARY"

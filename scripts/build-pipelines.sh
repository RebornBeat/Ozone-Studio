#!/usr/bin/env bash
# Build every pipeline that already has a release binary into one shared target
# dir. One job at a time caps peak RAM; shared deps (tokio, serde, ...) compile once.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/target/pipelines"
export CARGO_BUILD_JOBS=1
ok=0
fail=0
for dir in "$ROOT"/assets/pipelines/*/*/; do
  dir="${dir%/}"
  name="$(basename "$dir")"
  [ -f "$dir/Cargo.toml" ] || continue
  [ -f "$dir/target/release/$name" ] || continue
  if cargo build --release --manifest-path "$dir/Cargo.toml"; then
    ok=$((ok + 1))
    echo "OK   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name"
  fi
done
echo "built=$ok failed=$fail"

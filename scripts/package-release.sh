#!/usr/bin/env bash
# Package a deployable Ozone Studio release for Ubuntu x64:
#   dist/ozone-studio-<version>-linux-x64.tar.gz
# Contains: the host binary (host + full CLI), config.example.toml,
# systemd unit, and the deploy README. Pipelines are NOT included —
# copy target/pipelines/release/ separately if the VPS should run
# the full modality fleet (see DEPLOY.md).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VER="$(grep -m1 '^version' "$ROOT/Cargo.toml" | cut -d'"' -f2)"
OUT="$ROOT/dist"
NAME="ozone-studio-${VER}-linux-x64"
STAGE="$OUT/$NAME"

mkdir -p "$STAGE"
cp "$ROOT/target/release/ozone-studio" "$STAGE/"
cp "$ROOT/config.example.toml" "$STAGE/" 2>/dev/null || cp "$ROOT/config.toml" "$STAGE/config.example.toml"
cp "$ROOT/deploy/ozone-studio.service" "$STAGE/"
cp "$ROOT/deploy/DEPLOY.md" "$STAGE/"
tar -czf "$OUT/$NAME.tar.gz" -C "$OUT" "$NAME"
echo "packaged: $OUT/$NAME.tar.gz ($(du -h "$OUT/$NAME.tar.gz" | cut -f1))"

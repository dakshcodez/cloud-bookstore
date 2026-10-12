#!/bin/bash
# Builds a static (musl) Linux binary and packages it with the frontend.
# Uses Docker so it works regardless of local toolchain / glibc version.
# Output: dist/bookstore-release.tar.gz  -> upload to S3.
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p dist/stage
docker run --rm -v "$PWD":/work -w /work/backend rust:alpine sh -c '
  apk add --no-cache musl-dev gcc make perl >/dev/null &&
  CARGO_TARGET_DIR=/work/dist/target cargo build --release'

cp dist/target/release/bookstore dist/stage/bookstore
rm -rf dist/stage/frontend && cp -r frontend dist/stage/frontend
tar -czf dist/bookstore-release.tar.gz -C dist/stage .
ls -lh dist/bookstore-release.tar.gz

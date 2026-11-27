#!/usr/bin/env bash
# 릴리스 빌드 (Apple Silicon).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --target aarch64-apple-darwin -p fruit-star-app
ls -lh target/aarch64-apple-darwin/release/fruit-star-app

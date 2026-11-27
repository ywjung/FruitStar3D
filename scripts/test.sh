#!/usr/bin/env bash
# 품질 검사 전체: 형식, 린트, 테스트, 릴 배열 보장 조건 검증.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
if command -v cargo-nextest >/dev/null 2>&1; then
    cargo nextest run --workspace
else
    cargo test --workspace
fi
cargo run --release -q -p fruit-star-game --example strip_search
if command -v cargo-audit >/dev/null 2>&1; then
    cargo audit
else
    echo "cargo-audit이 없어 의존성 취약점 검사는 건너뜁니다."
fi

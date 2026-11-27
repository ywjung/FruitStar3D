#!/usr/bin/env bash
# 개발 환경 준비 (macOS, Apple Silicon).
# Rust 툴체인은 rust-toolchain.toml에 고정되어 있어 cargo를 처음 실행할 때 자동으로 설치된다.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! xcode-select -p >/dev/null 2>&1; then
    echo "Xcode Command Line Tools가 필요합니다: xcode-select --install" >&2
    exit 1
fi
if ! command -v rustup >/dev/null 2>&1; then
    echo "rustup이 필요합니다: https://rustup.rs 의 안내에 따라 설치하세요." >&2
    exit 1
fi

rustup show active-toolchain
cargo --version
command -v cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest --locked
# 선택 도구 (없어도 빌드와 테스트는 가능하다)
command -v cargo-audit >/dev/null 2>&1 || echo "선택: cargo install cargo-audit --locked"
cargo fetch
echo "준비 완료. scripts/run.sh 로 실행하세요."

#!/usr/bin/env bash
# 게임 실행. 인자는 그대로 앱에 전달된다 (예: scripts/run.sh --mode practice).
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run --release -p fruit-star-app -- "$@"

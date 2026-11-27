#!/usr/bin/env bash
# 성능 측정.
#   1) 도메인 로직 벤치마크 (Criterion)
#   2) 실제 앱을 자동 플레이로 실행해 프레임 시간을 로그로 남긴다 (창이 열린다)
set -euo pipefail
cd "$(dirname "$0")/.."
cargo bench -p fruit-star-game
SECONDS_TO_RUN="${1:-30}"
cargo run --release -q -p fruit-star-app -- \
    --mode free_play --db :memory: --seed 1 --auto-play --exit-after "$SECONDS_TO_RUN" 2>&1 |
    grep -E "성능 측정|ERROR|WARN" || true

# TEST_PLAN

계획서 15절의 시험 항목(TC-001 ~ TC-020)을 저장소에 있는 자동화 테스트와 대응시킨다.
이 문서는 "무엇으로 검증하는가"만 적는다. 실행 결과와 측정값은 `review/TEST_RESULT.md`,
`review/PERFORMANCE_REPORT.md`에 따로 기록한다.

## 1. 실행 방법

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace                 # nextest가 없으면 cargo test --workspace
cargo run --release -p fruit-star-game --example strip_search   # 릴 배열 전수 검증
cargo bench -p fruit-star-game                # 도메인 벤치마크 (Criterion)
```

`scripts/test.sh`가 앞의 네 가지(와 설치되어 있으면 `cargo audit`)를 순서대로 실행한다.

## 2. 자동화 테스트의 위치

| 파일 | 대상 | 실행 환경 |
|---|---|---|
| `crates/fruit-star-core/tests/reel.rs` | 릴 운동 모델, 상태 전이 | 순수 Rust |
| `crates/fruit-star-core/tests/credit.rs` | 메달 원장, 난수 범위 | 순수 Rust |
| `crates/fruit-star-game/tests/rules.rs` | 프로필 검증, 평가, 추첨, 릴 제어 전수 검사 | 순수 Rust |
| `crates/fruit-star-game/tests/controller.rs` | 한 게임의 흐름, 리플레이, 보너스, 모드 | 순수 Rust |
| `crates/fruit-star-game/tests/simulation.rs` | 장기 시뮬레이션 (60,000게임 / 4,000게임) | 순수 Rust |
| `crates/fruit-star-config/tests/config.rs` | TOML 로딩, 범위 검증, 기본값 복구, 필수 리소스 목록 | 파일 시스템 |
| `crates/fruit-star-storage/tests/store.rs` | 스키마, 트랜잭션, 복구, 설정, 달성 기록 | SQLite (메모리·임시 파일) |
| `crates/fruit-star-machine/tests/logic.rs` | 7세그먼트, 눌림 곡선, LED 패턴, 배치 치수 | 순수 함수 |
| `crates/fruit-star-machine/tests/plugin.rs` | ECS 계층 통합 (명령 → 도메인 → 메시지) | Bevy `MinimalPlugins`, 창 없음 |
| `crates/fruit-star-render/tests/cabinet.rs` | 절차 생성 캐비닛의 부품·연결 | Bevy, 창 없음 |
| `crates/fruit-star-render/src/reel_mesh.rs`, `image_ops.rs` (모듈 내 테스트) | 릴 메시 UV·방향, f16 변환 | 순수 함수 |
| `crates/fruit-star-physics/tests/medals.rs` | 메달 관통, 개수 상한, 정리 | Bevy + Rapier, 창 없음 |
| `crates/fruit-star-input/tests/keymap.rs` | 키 이름 해석, 기본 키 배정 | 순수 함수 |
| `crates/fruit-star-ui/tests/i18n.rs` | 번역표 완전성 | 순수 함수 |
| `crates/fruit-star-app/src/cli.rs` (모듈 내 테스트) | 명령줄 옵션 해석 | 순수 함수 |
| `crates/fruit-star-assetgen/src/*.rs` (모듈 내 테스트) | 생성물의 형식·여백·반복 구간 | 순수 함수 |

자동화 테스트가 없는 곳: `fruit-star-audio`, `fruit-star-render`의 카메라·조명·재질 반영,
`fruit-star-app`의 저장 연동(`persistence.rs`)·창 제어(`window.rs`), 실제 창을 띄운 전체 실행.

## 3. 시험 항목 대응

구분: **자동** = 자동화 테스트가 합격 기준을 직접 검사한다. **부분** = 일부만 자동이고 나머지는 수동 확인이 필요하다.
**수동** = 자동화 테스트가 없다. **미대응** = 수단이 아직 없다.

| ID | 검증 항목 | 구분 | 근거 |
|---|---|---|---|
| TC-001 | 앱 실행, 정상 로딩 | 부분 | `config.rs`: `shipped_files_load_strictly_and_match_builtin_defaults`, `required_assets_are_listed_and_missing_ones_are_reported`. 실제 실행은 4절의 실행 확인 옵션으로 수동 확인 |
| TC-002 | 필수 메시 누락 없음 | 자동 | `cabinet.rs`: `all_required_parts_exist_exactly_once`, `controls_displays_and_lamps_are_wired`. GLB 로딩이 아니라 절차 생성 모델을 검사한다 |
| TC-003 | 텍스처 깨짐 없음 | 부분 | `reel_mesh.rs`: `symbol_centres_map_to_their_texture_cells`, `triangles_face_outward`, `cell_aspect_matches_geometry`. assetgen: `symbols_are_centred_and_fill_the_cell`, `reel_band_tiles_and_is_opaque`, `labels_have_fixed_height_and_white_ink`. 화면에서의 모습은 스크린샷으로 수동 확인 |
| TC-004 | START 입력당 이벤트 1회 | 자동 | `plugin.rs`: `lever_input_produces_exactly_one_event_and_duplicates_are_ignored`. `controller.rs`: `a_full_blank_game_follows_the_expected_flow` |
| TC-005 | 릴 3개 독립 회전 | 자동 | `reel.rs`: `starts_idle_and_reaches_constant_speed`, `acceleration_distance_matches_kinematics`. `cabinet.rs`: `reels_are_independent_drums_at_their_layout_positions` |
| TC-006 | 개별 정지 | 자동 | `reel.rs`: `stops_exactly_on_requested_index_and_passes_through_all_states`, `stop_time_is_bounded_by_slip_distance`. `controller.rs`: `a_full_blank_game_follows_the_expected_flow` (정지 순서 기록 포함) |
| TC-007 | 화면과 내부 결과 일치 | 자동 | `controller.rs`: `displayed_symbols_match_the_internal_result`. `reel.rs`: `result_is_independent_of_frame_rate`, `a_single_huge_time_step_still_lands_on_target`, `window_shows_upper_index_on_top_row`. `reel_mesh.rs`: `rotation_brings_the_requested_symbol_to_the_front`. `store.rs`: `reel_results_match_the_outcome` |
| TC-008 | 중복 입력 처리 없음 | 자동 | `reel.rs`: `duplicate_and_premature_stop_requests_are_rejected`. `plugin.rs`: `lever_input_produces_exactly_one_event_and_duplicates_are_ignored`, `pause_and_gallery_block_machine_input` |
| TC-009 | 결과 평가가 규칙과 일치 | 자동 | `rules.rs`: `three_of_a_kind_on_each_payline_pays_once`, `replay_and_bonus_roles_are_reported_without_payout`, `payout_is_capped_per_game`, `lottery_draw_follows_cumulative_weights`, `reel_control_never_produces_a_disallowed_win`, `grape_and_replay_are_always_pulled_in`, `bonus_symbols_can_be_aligned_when_aimed`. `controller.rs`: `grape_pays_exactly_once`, `replay_gives_a_free_next_game`, `bonus_flag_is_carried_until_aligned_then_bonus_runs_to_its_cap`, `regular_bonus_starts_from_bar`, `sevens_cannot_be_aligned_without_the_bonus_flag` |
| TC-010 | 메달 음수·중복 정산 없음 | 자동 | `credit.rs`: `bet_never_drives_balance_negative`, `a_game_can_be_settled_only_once`, `overflow_is_reported_without_changing_state`. `controller.rs`: `classic_mode_rejects_bet_without_credit_and_never_goes_negative`. `simulation.rs`: `random_player_keeps_ledger_consistent_and_rtp_in_band`. `store.rs`: `games_and_ledger_entries_are_recorded_together`, `duplicate_game_id_is_rejected_and_leaves_no_partial_rows` |
| TC-011 | 음향이 이벤트와 동기화 | 수동 | `fruit-star-audio`에 테스트가 없다. 간접 근거로 이벤트가 정확히 한 번씩 나는지는 TC-004·TC-008이 검사하고, 파일 형식과 반복 구간 연속성은 assetgen의 `every_sound_passes_its_sanity_checks`가 검사한다. 소리를 듣고 확인한 적은 없다 |
| TC-012 | 메달의 비정상 관통 방지 | 자동 | `medals.rs`: `medals_land_in_the_tray_without_tunnelling`, `medal_count_never_exceeds_the_limit_and_oldest_are_reused`, `zero_limit_disables_medals`, `leaving_the_game_clears_medals`. `logic.rs`: `layout_is_physically_consistent` |
| TC-013 | 카메라 표시 및 조작 | 수동 | 카메라 코드에 테스트가 없다. `--camera-at`과 `--screenshot`으로 프리셋 전환 화면을 남겨 확인한다 |
| TC-014 | 전체 화면 전환·복귀 | 수동 | `F` 키로 직접 확인 |
| TC-015 | Retina에서 UI 가독성 | 수동 | 실제 화면에서 확인 |
| TC-016 | 설정 저장, 재실행 시 복원 | 부분 | `store.rs`: `settings_round_trip_and_survive_history_reset`. `config.rs`: `written_defaults_round_trip`. 앱 쪽의 저장·복원(`save_settings`, `apply_saved_settings`)은 재실행으로 수동 확인 |
| TC-017 | 비정상 종료 후 DB 정합성 | 자동 | `store.rs`: `data_survives_reopen_and_unclean_shutdown_is_recovered`, `recording_into_an_unknown_session_rolls_back`, `duplicate_game_id_is_rejected_and_leaves_no_partial_rows`, `newer_schema_is_refused`. 실행 중인 앱을 강제 종료하는 시험은 수동 |
| TC-018 | 24시간 안정성 | 미대응 | 장시간 실행용 테스트나 스크립트가 없다. `--auto-play --exit-after <초>`로 장시간 실행은 할 수 있으나 메모리 추이를 기록하는 수단은 없다 |
| TC-019 | 성능 (기본 60 FPS 목표) | 부분 | 도메인 연산은 `crates/fruit-star-game/benches/rules.rs`로 잰다. 프레임 시간은 `--exit-after`로 끝날 때 로그에 남는 "성능 측정" 줄(프레임 수, 평균, 최대)을 본다 (`scripts/benchmark.sh`). 합격 기준을 자동으로 판정하지는 않는다 |
| TC-020 | 개발 도구 없는 Mac에서 실행 | 수동 | `scripts/package.sh`가 만든 `.app`을 다른 Mac에서 실행해 확인한다. 자동화되어 있지 않다 |

## 4. 실행 확인용 명령줄 옵션

`crates/fruit-star-app/src/cli.rs`의 옵션은 주로 창을 띄운 실행을 사람 손 없이 확인하기 위한 것이다.

| 옵션 | 동작 |
|---|---|
| `--mode <classic\|free_play\|practice>` | 메뉴를 건너뛰고 바로 시작 |
| `--view <play\|mechanical\|gallery>` | 보기 방식 (`--mode`와 함께) |
| `--seed <숫자>` | 추첨 난수 시드 고정 |
| `--db <경로\|:memory:>` | 데이터베이스 위치. `:memory:`면 디스크에 쓰지 않는다 |
| `--auto-play` | 일정한 간격으로 베팅·레버·정지를 자동 입력 |
| `--screenshot <초> <파일.png>` | 지정 시각에 화면 저장 (여러 번 지정 가능) |
| `--camera-at <초>` | 지정 시각에 카메라를 다음 프리셋으로 (여러 번 지정 가능) |
| `--exit-after <초>` | 지정 시간 뒤 종료. 종료 직전에 프레임 시간 통계를 로그로 남긴다 |

시각은 로딩이 끝난 뒤부터 잰다. 프레임 통계는 처음 2초를 제외한다. 예:

```bash
cargo run --release -p fruit-star-app -- \
    --mode free_play --db :memory: --seed 1 --auto-play \
    --screenshot 5 shot.png --exit-after 30
```

기록을 건드리지 않으려면 `--db :memory:`를 함께 쓴다.

## 5. 품질 목표와 현재 수단

| 계획서의 목표 | 수단 | 상태 |
|---|---|---|
| Clippy 경고 0건, 컴파일 오류 0건 | `cargo clippy --workspace --all-targets -- -D warnings` | 명령으로 확인 |
| 핵심 로직 커버리지 90% | — | 커버리지 측정 도구가 설정되어 있지 않다 |
| 메모리 누수·비정상 종료 0건 | — | 전용 검사 수단이 없다 |
| 의존성 점검 | `cargo audit` (`scripts/test.sh`에서 설치되어 있을 때만) | 선택 |

## 6. 미구현 / 계획과의 차이

- 루트의 `tests/`와 `benches/` 디렉터리는 비어 있다. 테스트와 벤치마크는 각 크레이트 안에 있다.
- 창을 띄우는 종단 간 테스트는 자동화 테스트 묶음에 없다. 4절의 옵션을 쓴 실행은 사람이 로그와 스크린샷을 확인해야 한다.
- TC-011, TC-013, TC-014, TC-015, TC-020은 수동 항목이고 TC-018은 수단이 없다.
- 디버그 콘솔(FPS, 프레임 시간, 메모리, 릴 각도, LED 밝기, 와이어프레임, 충돌 영역)은 개발 빌드에서만 켤 수 있다
  ([UI_DESIGN.md](UI_DESIGN.md) 8절).

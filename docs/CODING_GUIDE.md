# CODING_GUIDE

이 저장소의 코드에서 실제로 지켜지고 있는 규칙과, 자주 하는 확장 작업의 절차를 적는다.
구조는 [ARCHITECTURE.md](ARCHITECTURE.md)를 먼저 읽는다.

## 1. 툴체인과 워크스페이스

| 항목 | 값 | 위치 |
|---|---|---|
| Rust | 1.100.0, `rustfmt`·`clippy` 포함, 타깃 `aarch64-apple-darwin` | `rust-toolchain.toml` |
| Edition / resolver | 2024 / 3 | 루트 `Cargo.toml` |
| `rust-version` | 1.97.1 (고정된 툴체인보다 낮은 값으로 적혀 있다) | 루트 `Cargo.toml` |
| 의존성 버전 | `[workspace.dependencies]`에 모으고 `Cargo.lock`으로 고정 | 루트 `Cargo.toml` |
| 프로필 | dev: 자체 코드 `opt-level = 1`, 의존성 `opt-level = 3` / release: thin LTO, `codegen-units = 1`, strip | 루트 `Cargo.toml` |

`fruit-star-assetgen`만 자체 의존성(`anyhow`, `png`, `resvg`)을 크레이트 `Cargo.toml`에 직접 적는다.

## 2. 린트

루트 `Cargo.toml`의 `[workspace.lints]`를 모든 크레이트가 `[lints] workspace = true`로 받는다.

| 린트 | 수준 | 의미 |
|---|---|---|
| `unsafe_code` | forbid | `unsafe`를 쓸 수 없다 |
| `clippy::unwrap_used`, `clippy::expect_used` | warn | 검사 명령이 `-D warnings`라서 운영 코드의 `unwrap`/`expect`는 실패로 처리된다 |
| `clippy::type_complexity`, `clippy::too_many_arguments` | allow | Bevy 시스템의 쿼리·인자 형태 때문에 허용 |

- 테스트 파일은 맨 위에 `#![allow(clippy::unwrap_used, clippy::expect_used)]`를 둔다.
- 운영 코드에서는 `?`, `let … else`, `unwrap_or_default()`, `map_or(…)`로 처리한다.
  "일어날 수 없는" 분기도 패닉 대신 안전한 기본 동작으로 돌아간다 (예: `controller.rs`의 `settle_game`).

통과해야 하는 검사:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo build --release --target aarch64-apple-darwin
```

## 3. 구조 규칙

- **도메인은 Bevy에 의존하지 않는다.** `fruit-star-core`, `fruit-star-game`, `fruit-star-config`,
  `fruit-star-storage`의 `Cargo.toml`에 `bevy`를 넣지 않는다.
- **메달 원장은 도메인에서만 바꾼다.** `CreditLedger`를 변경하는 것은 `GameController`뿐이다.
  ECS 시스템은 `Machine`의 입력 메서드를 호출하거나 값을 읽기만 한다.
- **결과와 연출을 분리한다.** 지급은 정산 시점에 원장에 반영되고, 메달 배출·표시기 증가는
  `PayoutCounter`가 따로 진행한다. 물리 메달은 결과에 영향을 주지 않는다.
- **Presentation 크레이트끼리는 직접 의존하지 않는다.** 주고받을 것은 `fruit-star-machine`의 메시지,
  컴포넌트, `layout` 상수로 둔다.
- **시스템 순서는 `MachineSet`으로 정한다.** 입력은 `Input`, 도메인 호출은 `Simulate`, 논리 상태는 `React`,
  메시·재질·소리·화면은 `Present`에 넣는다.
- **`main.rs`는 조립만 한다.** 기능은 각 크레이트의 플러그인에 둔다.
- **시간은 경과 시간으로 다룬다.** 도메인은 `tick(dt)`만 받는다. 프레임 수에 의존하는 로직을 두지 않는다.
- **난수는 주입한다.** 도메인은 `RandomSource` 트레이트만 안다. 테스트는 `SplitMix64`나 정해진 값을 돌려주는 구현을 쓴다.
- **치수는 한곳에 둔다.** 부품 위치와 크기는 `machine/layout.rs`(미터)에 두고 렌더링·물리·음향이 함께 쓴다.

## 4. 설정값

- 조정할 수 있는 수치는 `config/*.toml`에 둔다. 구조체는 `#[serde(default, deny_unknown_fields)]`를 달아
  빠진 키는 기본값으로 채우고 모르는 키는 오류로 처리한다.
- 구조체마다 `validate()`가 `check_range`로 범위를 검사한다. 오류 메시지는 사용자에게 그대로 보여 줄 수 있게 쓴다.
- `Default` 구현과 저장소에 들어 있는 TOML 파일은 같은 값이어야 한다.
  `config/tests/config.rs`의 `shipped_files_load_strictly_and_match_builtin_defaults`가 이를 검사한다.
- 설정 키는 이 애플리케이션의 스키마다. Bevy API 이름과 맞출 필요가 없다.

## 5. 오류 처리와 로그

- 크레이트마다 `thiserror` 열거형을 둔다: `ReelError`, `CreditError`, `ProfileError`, `GameError`,
  `ConfigError`, `StoreError`. 메시지는 한국어다.
- `GameError`의 대부분은 "지금은 받을 수 없는 입력"이다. `apply_commands`는 이를 `debug!`로만 남긴다.
- 저장 실패 같은 실행 중 오류는 `error!`로 남기고 `Notice` 메시지로 화면에 알린다. 게임은 계속 진행한다.
- 시작 단계의 실패(설정, 저장소, 기종)는 표준 오류에 사유를 쓰고 종료 코드로 끝낸다.
  리소스 누락은 `AppState::Failed` 화면으로 보여 준다.

## 6. 이름과 주석

- 식별자, 파일 이름, 설정 키는 영어다. 설정·DB에 쓰는 문자열은 열거형의 `key()` / `from_key()` 쌍으로 변환한다
  (`Symbol`, `Role`, `Flag`, `BonusKind`, `PlayMode`, `Language`, `Quality`, `Achievement`).
- 문서 주석(`//!`, `///`)과 일반 주석은 한국어로 쓴다. 왜 그렇게 했는지를 적는다.
  예외로 `fruit-star-assetgen`의 주석은 영어다.
- Bevy 0.20의 이름을 따른다: 계층 간 통지는 `Message` / `MessageReader` / `MessageWriter`,
  화면 전환은 `States`, 부품 클릭은 옵저버(`On<PointerPress>`)다.
- 사용자에게 보이는 문자열은 코드에 직접 쓰지 않고 `ui/i18n.rs`의 번역표에 키로 등록한다.

## 7. 테스트 위치

| 크레이트 | 위치 |
|---|---|
| `core`, `game`, `config`, `storage`, `machine`, `render`, `input`, `physics`, `ui` | 각 크레이트의 `tests/*.rs` (통합 테스트) |
| `game` | 공용 도우미는 `tests/common/mod.rs`, 전수 검증 도구는 `examples/strip_search.rs`, 벤치마크는 `benches/rules.rs` |
| `render` | `tests/cabinet.rs` 외에 `src/reel_mesh.rs`, `src/image_ops.rs` 안에 단위 테스트가 있다 |
| `app` | `src/cli.rs` 안의 단위 테스트뿐이다 |
| `assetgen` | 각 모듈 안의 `#[cfg(test)]` |
| `audio` | 테스트가 없다 |

Bevy가 필요한 테스트는 창과 GPU 없이 `MinimalPlugins`로 돌린다. 시간은 `TimeUpdateStrategy::ManualDuration`으로
고정한다 (`machine/tests/plugin.rs`, `physics/tests/medals.rs`). 시험 항목과의 대응은 [TEST_PLAN.md](TEST_PLAN.md)에 있다.

## 8. 확장 절차

### 심볼 추가

1. `core/symbol.rs`: `Symbol`에 값을 추가하고 `ALL`(배열 길이 포함)과 `key()`를 고친다.
2. `game/profile.rs`: 모든 심볼은 정확히 하나의 역할에 대응해야 한다
   (`rules.rs`의 `every_symbol_maps_to_exactly_one_role`). 아래 "역할 추가"를 함께 한다.
3. `assetgen/src/symbols.rs`: `NAMES`와 `build()`에 그림을 추가하고 생성기를 실행해 `assets/symbols/<key>.png`를 만든다.
   필수 리소스 목록은 `Symbol::ALL`에서 자동으로 나온다.
4. 릴 배열(`config/machines/fruit_classic.toml`과 `MachineProfile::fruit_classic()` 양쪽)에 넣고
   `cargo run --release -p fruit-star-game --example strip_search`로 릴 제어 조건을 확인한다.

### 역할 추가

1. `game/profile.rs`: `Role`의 값, `COUNT`, `ALL`, `key()`, `symbol()`, 보너스라면 `bonus_kind()`.
2. 배당은 `PayTable::default()`와 TOML의 `[payouts]`, 추첨 가중치는 `fruit_classic()`과 TOML의 `[lottery.*]`.
   가중치 합은 65536 이하여야 한다. 소역 이름은 `Flag::from_key`가 `Role::key()`로 자동 인식한다.
3. 보너스 역할이면 `game/control.rs`의 `bonus_roles` 대응과 `BonusRules`를 고친다.
4. `ui/lib.rs`의 `flag_key()`와 `ui/i18n.rs`의 `flag.*` 키를 추가한다 (Practice 표시용).
5. `game/tests/rules.rs`의 `all_allowed()`에 넣어 전수 검사 대상에 포함시킨다.

### 효과음 추가

1. `assetgen/src/audio.rs`의 `SPECS`에 사양과 합성 코드를 추가하고 생성기를 실행한다.
2. `config/src/loader.rs`의 `required_assets()` 목록에 이름을 추가한다.
3. `audio/src/lib.rs`: `Sounds`에 필드를 추가하고 `load_sounds`에서 읽는다.
   해당 메시지를 읽는 시스템에서 `play(…, 위치, 배율, 속도)`를 호출한다.
4. [AUDIO_DESIGN.md](AUDIO_DESIGN.md)의 표를 고친다.

### 설정 항목 추가

1. `config/src/*.rs`: 구조체 필드, `Default`, `validate()`의 범위 검사.
2. `config/*.toml`: 같은 키와 기본값을 주석과 함께 추가한다.
3. 사용하는 쪽에서 `Res<Settings>`의 `settings.config.…`로 읽는다.
4. 화면에서 조절하게 하려면: `ui/lib.rs`의 `SettingKind`, `on_activate`, `update_dynamic`,
   `ui/screens.rs`의 `spawn_setting(…)`, `ui/i18n.rs`의 `settings.*` 키, `ui/tests/i18n.rs`의 사용 키 목록.
5. 실행 사이에 유지하려면: `app/src/persistence.rs`의 `KEY_*` 상수, `apply_saved_settings`, `save_settings`.

### 그림·소리 파일

`assets/`와 `design/concept/`의 파일, `assets/ASSET_SOURCES.md`는 `fruit-star-assetgen`의 출력이다.
직접 고치지 말고 생성기 코드를 고친 뒤 `cargo run -p fruit-star-assetgen --release`를 다시 실행한다.

## 9. 하지 않는 것

- 실제 결제, 환전, 외부 서버 통신을 구현하지 않는다.
- 실제 상표·캐릭터·기존 기기의 그림과 확률을 복제하지 않는다.
- 목업을 구현으로 표시하지 않는다. 끝나지 않은 기능은 각 문서의 "미구현 / 계획과의 차이"와 `review/`에 적는다.

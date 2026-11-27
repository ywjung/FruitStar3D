# FRUIT STAR 3D

Rust와 Bevy로 만든 기계식 과일 릴 게임기의 3D 시뮬레이션이다. macOS(Apple Silicon, Metal)에서
오프라인 싱글 플레이로 동작한다. 기종은 FRUIT CLASSIC 한 대(3릴 × 21심볼, 3×3 표시창, 유효 라인 5개)다.

## 가상 메달 전용

이 프로그램은 가상 메달만 사용한다. 실제 화폐, 결제, 환전, 네트워크 통신 기능은 없다.
추첨 확률과 배당은 이 프로젝트가 독자적으로 정한 값이며 실제 기기와 무관하다
([config/machines/fruit_classic.toml](config/machines/fruit_classic.toml)).
통계는 플레이 기록 열람과 검증용이며 당첨 예측 기능은 없다.

## 요구 사항

| 항목 | 내용 |
|---|---|
| 운영체제 | macOS, Apple Silicon (`aarch64-apple-darwin`). `graphics.toml`의 `render.backend`는 `metal`만 허용한다 |
| Rust | `rust-toolchain.toml`이 1.100.0(`rustfmt`, `clippy` 포함)을 고정한다. `cargo`를 처음 실행할 때 rustup이 설치한다 |
| 빌드 도구 | Xcode Command Line Tools, rustup |
| 선택 도구 | `cargo-nextest`(테스트 실행), `cargo-audit`(의존성 점검) |

주요 의존성 버전은 `Cargo.lock`으로 고정되어 있다: Bevy 0.20.1, bevy_rapier3d 0.37.0,
rusqlite 0.43.0(bundled SQLite), rand 0.10, toml 1.2, thiserror 2.

## 빠른 시작

```bash
# 실행 (개발 빌드)
cargo run -p fruit-star-app

# 그림·효과음 재생성 (assets/, design/concept/, assets/ASSET_SOURCES.md를 다시 쓴다)
cargo run -p fruit-star-assetgen --release

# 품질 검사
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# 릴리스 빌드
cargo build --release --target aarch64-apple-darwin
```

`scripts/`에는 위 명령을 묶은 보조 스크립트가 있다.

| 스크립트 | 하는 일 |
|---|---|
| `scripts/setup.sh` | Xcode CLT·rustup 확인, `cargo-nextest` 설치, `cargo fetch` |
| `scripts/run.sh` | 릴리스 빌드로 실행 (인자는 앱에 그대로 전달) |
| `scripts/test.sh` | fmt, clippy, 테스트, 릴 배열 검증(`strip_search`), 선택적으로 `cargo audit` |
| `scripts/build.sh` | `aarch64-apple-darwin` 릴리스 빌드 |
| `scripts/benchmark.sh` | 도메인 벤치마크와 자동 플레이 프레임 시간 측정 |
| `scripts/package.sh` | `dist/`에 `.app`, ZIP, DMG, SHA256SUMS 생성 (ad-hoc 서명, 공증 없음) |

## 조작

키 배정은 [config/controls.toml](config/controls.toml)에서 바꿀 수 있다. 아래는 기본값이다.

| 입력 | 기능 |
|---|---|
| `B` | MAX BET (3메달까지 한 번에) |
| `N` | 1 BET (메달 한 장씩, 최대 3장) |
| `Space` | START 레버 |
| `1` / `2` / `3` 또는 `A` / `S` / `D` | 왼쪽 / 가운데 / 오른쪽 릴 정지 |
| `C` | 가상 메달 추가 (기본 50장) |
| `V` | 카메라 프리셋 전환 |
| `F` | 전체 화면 전환 |
| `M` | 음소거 전환 |
| `P` / `Esc` | 일시정지 및 설정 화면 |
| `F3` | 디버그 표시 (개발 빌드에서만 동작) |
| 부품 클릭 | 레버 손잡이, MAX BET, 1 BET, 정지 버튼(또는 릴 띠), 메달 투입구 |
| 빈 곳 드래그 / 오른쪽 버튼 드래그 | 카메라 회전 |
| 스크롤 / 트랙패드 핀치 | 확대·축소 |

플레이 방법과 설정은 [docs/USER_GUIDE.md](docs/USER_GUIDE.md)에 있다.

## 디렉터리

```text
Cargo.toml, Cargo.lock, rust-toolchain.toml
crates/
  fruit-star-core/      심볼, 릴 운동 모델, 메달 원장, 난수 추상화 (Bevy 비의존)
  fruit-star-game/      추첨, 릴 제어, 조합 평가, 보너스, 게임 컨트롤러 (Bevy 비의존)
  fruit-star-config/    TOML 로딩과 검증
  fruit-star-storage/   SQLite 기록, 통계, 복구
  fruit-star-machine/   도메인과 Bevy ECS의 연결, 표시기·램프·눌림의 논리 상태, 배치 치수
  fruit-star-render/    절차 생성 캐비닛, 릴 메시, 재질, 조명, 카메라
  fruit-star-input/     키보드·마우스·트랙패드
  fruit-star-audio/     기계음, 공간 음향
  fruit-star-physics/   메달 낙하 (bevy_rapier3d)
  fruit-star-ui/        메뉴, HUD, 일시정지·설정, 다국어
  fruit-star-app/       실행 파일: 설정·저장소 준비와 플러그인 조립
  fruit-star-assetgen/  그림과 효과음을 코드로 생성하는 도구
config/                 application / graphics / audio / controls / machines/fruit_classic
assets/                 symbols/, textures/, audio/ (생성물), ASSET_SOURCES.md
design/concept/         심볼·패널의 SVG 원본 (생성물). design/blender/는 비어 있다
docs/                   설계 문서
scripts/                보조 스크립트
review/                 상태·시험·성능 기록 (별도 작성)
```

## 문서

| 문서 | 내용 |
|---|---|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 계층, 크레이트 의존 방향, 프레임 처리 순서, 메시지 목록 |
| [docs/GAME_DESIGN.md](docs/GAME_DESIGN.md) | 규칙, 배당·추첨표, 릴 제어, 보너스, 모드 |
| [docs/GRAPHICS_DESIGN.md](docs/GRAPHICS_DESIGN.md) | 재질, 조명, 후처리, 릴 텍스처, 품질 설정 |
| [docs/MODELING_GUIDE.md](docs/MODELING_GUIDE.md) | 좌표계, 치수, 부품 이름과 피벗 |
| [docs/AUDIO_DESIGN.md](docs/AUDIO_DESIGN.md) | 효과음 목록, 발생 조건, 음량 모델 |
| [docs/UI_DESIGN.md](docs/UI_DESIGN.md) | 화면, HUD, 설정, 다국어, 캐비닛 표시기 |
| [docs/CODING_GUIDE.md](docs/CODING_GUIDE.md) | 코드 규칙, 확장 절차 |
| [docs/TEST_PLAN.md](docs/TEST_PLAN.md) | TC-001 ~ TC-020과 자동화 테스트의 대응 |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | 플레이 방법, 설정, 기록 위치 |
| `PLAN.md` | 개발 계획서 |

구현 상태, 시험 결과, 성능 측정, 미해결 이슈는 이 문서 묶음이 아니라 [review/](review/)에 따로 기록한다
(`IMPLEMENTATION_STATUS.md`, `TEST_RESULT.md`, `PERFORMANCE_REPORT.md`, `ISSUE_LIST.md`,
`RELEASE_CHECKLIST.md`, `RELEASE_NOTES.md`).

## 계획서와 다른 점 (요약)

각 문서의 "미구현 / 계획과의 차이" 절에 자세히 적었다.

- 캐비닛은 Blender/GLB 모델이 아니라 코드로 생성한다 ([MODELING_GUIDE](docs/MODELING_GUIDE.md)).
- 효과음은 모두 절차 합성 WAV이며 사람이 들어 보고 조정하지 않았다 ([AUDIO_DESIGN](docs/AUDIO_DESIGN.md)).
- 기종은 FRUIT CLASSIC 한 대뿐이고 기종 선택 화면이 없다 ([UI_DESIGN](docs/UI_DESIGN.md)).
- `render_scale`은 읽고 검증만 하며 렌더링에 반영하지 않는다. 그래픽 품질 변경은 재시작 후 적용된다
  ([GRAPHICS_DESIGN](docs/GRAPHICS_DESIGN.md)).
- 환경광은 HDR 파일이 아니라 코드로 만든 HDR 큐브맵이다.

## 데이터 저장 위치

기록은 `~/Library/Application Support/FruitStar3D/fruitstar.sqlite3`에 저장된다.
`--db <경로>` 또는 `--db :memory:` 옵션으로 바꿀 수 있다 ([USER_GUIDE](docs/USER_GUIDE.md) 참고).

## iPhone

같은 코드로 iPhone 앱도 만든다. 세로 화면 전용이며 화면 아래의 터치 조작판(+ 메달, 1 BET, MAX BET,
START, STOP×3)으로 플레이한다. 게임기 부품을 직접 눌러도 되고, 빈 곳을 끌면 회전, 두 손가락으로 확대한다.

```bash
scripts/package-ios.sh            # 시뮬레이터용 → dist/ios-sim/FruitStar3D.app
xcrun simctl install booted dist/ios-sim/FruitStar3D.app && xcrun simctl launch booted local.fruitstar3d
```

```bash
SIGN_IDENTITY="Apple Development: …" PROFILE_PATH=<프로파일.mobileprovision> scripts/package-ios.sh device
xcrun devicectl device install app --device <UDID> dist/ios-device/FruitStar3D.app
```

- 그래픽은 `config/platform/ios/graphics.toml`(중간 품질, 60 FPS, 주변광 차폐·모션 블러 끔)을 쓴다.
- **임시 조치**: iOS 27 SDK로 빌드한 앱은 UIScene 생명주기를 써야 하지만 Bevy 0.20의 winit 0.30은 아직
  지원하지 않는다. 스크립트가 실행 파일에 기록된 SDK 버전을 26.0으로 바꿔 실행되게 한다. 개발·개인 설치용이며
  앱 스토어에는 제출할 수 없다.

## 라이선스

- 소스 코드: 워크스페이스 `Cargo.toml`의 `license` 항목은 `MIT`다. 저장소에 별도의 LICENSE 파일은 없다.
- 그림과 효과음: `fruit-star-assetgen`이 생성한 이 프로젝트의 원본이며 제3자 자료를 쓰지 않았다.
  파일별 내용과 출처는 [assets/ASSET_SOURCES.md](assets/ASSET_SOURCES.md)에 있다.
- 서드파티 크레이트의 라이선스 목록은 `scripts/package.sh`가 `dist/LICENSES.md`로 만든다.

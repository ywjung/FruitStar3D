# FRUIT STAR 3D — 상세 개발 계획서

## 1. 프로젝트 개요
Rust와 Bevy를 이용해 실제 일본식 기계식 과일 파치슬롯 게임기를 macOS에서 재현한다. Apple Silicon ARM64, Metal/wgpu, 오프라인 싱글 플레이를 대상으로 한다. 실제 결제·환전 없이 가상 메달만 사용한다. 초기 개발 기간은 10주, 1인 개발과 AI 코딩 도구 병행을 가정한다.

## 2. 제품 요구사항
- 실제 게임기 형태의 3D 캐비닛: 크롬 프레임, 빨간 광택 외장, 전면 유리, 상단 장식, LED, 스피커, 메달 투입구와 배출구.
- 3개 독립 원통형 릴, 릴당 21개 심볼 위치, 3×3 표시창.
- 독자적인 포도, 체리, 빨간 7, 파란 7, BAR, 벨, 수박, 스타, REPLAY 그림. 이모지 대신 고해상도 일러스트 사용.
- START 레버, MAX BET, 3개 STOP 버튼, CREDIT/PAYOUT/COUNT 7세그먼트 표시기, STAR CHANCE 램프.
- 정면 플레이, 좌우 회전, 확대, 내부 구조 관찰 카메라.
- Classic, Free Play, Practice, Mechanical View, Gallery 모드. Debug는 개발 빌드 전용.
- 한국어·일본어·영어, Retina 및 전체 화면 지원.
- FRUIT CLASSIC을 우선 완성하고 GOLD SEVEN, TOKYO NEON, RETRO 1970은 후속 리소스 팩으로 확장.

## 3. 기술 스택
| 기술 | 역할 |
|---|---|
| Rust / Cargo Workspace | 핵심 프로그램 및 모듈 관리 |
| Bevy 3D / ECS | 게임 루프, 상태, 이벤트, UI |
| wgpu / Metal / WGSL | GPU 렌더링 및 셰이더 |
| Blender / glTF / GLB | 3D 모델 제작 및 로딩 |
| Rapier3D | 메달 낙하 및 충돌 효과 |
| Kira 또는 호환 Bevy 오디오 | 기계음 및 공간 음향 |
| Serde / TOML | 설정 관리 |
| rand | 난수 기반 게임 결과 |
| rusqlite | 로컬 기록 |
| tracing / thiserror | 로그 및 오류 처리 |
| Criterion / nextest / Clippy | 성능 및 품질 검증 |

Bevy 및 연동 라이브러리 버전은 실제 호환성 검증 후 Cargo.lock으로 고정한다.

## 4. 게임기 디자인 및 리소스 명세
Blender에서 Cabinet, FrontGlass, ChromeFrame, TopPanel, ReelLeft, ReelCenter, ReelRight, StopButtonLeft, StopButtonCenter, StopButtonRight, StartLever, BetButton, MedalSlot, MedalTray, LEDPanel, Speaker를 독립 메시로 제작한다. 움직이는 부품은 독립 피벗을 갖고, 이름을 GLB 노드에 유지한다. 단위는 미터로 통일하고 변환을 적용한 뒤 내보낸다.

| 재질 | Metallic | Roughness |
|---|---:|---:|
| 크롬 | 1.0 | 0.12 |
| 스테인리스 | 1.0 | 0.30 |
| 빨간 플라스틱 | 0.0 | 0.22 |
| 버튼 | 0.0 | 0.18 |
| 릴 인쇄면 | 0.0 | 0.45 |
| 유리 | 0.0 | 0.05 |
| 메달 | 1.0 | 0.25 |

HDR 환경광, 환경 반사, Bloom, 그림자, Ambient Occlusion, Tone Mapping, 릴 내부 조명을 적용한다. 유리 반사 때문에 심볼 가독성이 저하되지 않도록 검증한다. 실제 상표와 캐릭터를 복제하지 않는다. 모든 리소스의 출처와 라이선스를 기록한다.

## 5. 릴 엔진 및 게임 흐름
게임 시작 준비 → 레버 입력 → 릴 회전 → 개별 정지 → 심볼 조합 평가 → 가상 메달 정산 → 이력 저장 → 대기.

릴 상태: Idle, Accelerating, Spinning, Decelerating, Settling, Stopped. 각 릴은 현재 각도, 각속도, 심볼 배열, 현재 인덱스, 목표 각도 및 상태를 관리한다. 프레임률과 독립적인 시간 기반 업데이트를 사용하고, 정지 후 표시 심볼과 내부 결과가 일치해야 한다. 중복 정지 입력을 무시하고 정산은 게임당 한 번만 수행한다. 게임 결과와 연출을 분리한다. 세부 보상 규칙은 독자적으로 정의하며 실제 기기 확률을 복제하지 않는다.

```rust
pub enum Symbol { Grape, Cherry, RedSeven, BlueSeven, Bar, Bell, Melon, Star, Replay }
pub enum ReelState { Idle, Accelerating, Spinning, Decelerating, Settling, Stopped }
pub struct Reel {
    pub id: u8,
    pub symbols: Vec<Symbol>,
    pub state: ReelState,
    pub current_angle: f32,
    pub angular_velocity: f32,
    pub target_angle: Option<f32>,
    pub current_index: usize,
}
pub enum GameState { Loading, MainMenu, MachineSelect, Ready, Spinning, Stopping, Evaluating, Result, Bonus, Paused, Error }
```

## 6. 입력 및 애니메이션
| 입력 | 기능 |
|---|---|
| B | MAX BET |
| Space | START 레버 |
| 1/2/3 또는 A/S/D | 좌/중/우 릴 정지 |
| C | 가상 메달 추가 |
| V | 카메라 변경 |
| F | 전체 화면 |
| M | 음소거 |
| P / ESC | 일시정지 / 메뉴 |

모든 게임기 조작은 마우스와 트랙패드로도 가능해야 한다. 포커스 상실 시 입력을 안전하게 처리한다.

| 애니메이션 | 초기 목표 |
|---|---:|
| 레버 당기기 및 복귀 | 250ms |
| STOP 버튼 | 120ms |
| MAX BET | 100ms |
| 릴 가속 | 350ms |
| 릴 감속 | 250ms |
| 정지 진동 | 80ms |
| 메달 투입 | 400ms |
| 카메라 전환 | 500ms |

회전 중 모션 블러와 정지 시 미세한 관성을 표현한다. LED 밝기 및 점멸 감소 설정을 제공한다.

## 7. 사운드
power_on.ogg, ambient.ogg, lever_pull.wav, lever_return.wav, reel_start.wav, reel_loop.ogg, reel_stop.wav, button_press.wav, button_release.wav, medal_insert.wav, medal_drop.wav, medal_collision.wav, bonus_start.ogg, bonus_end.ogg, menu_select.wav를 관리한다. 릴별 위치 음향, 효과음·배경음 독립 볼륨, 음소거, 이벤트 동기화를 구현한다. 반복음은 릴 정지 및 화면 전환 시 종료한다.

## 8. 아키텍처
Presentation(3D/UI/입력) → Application(Bevy ECS 및 상태 관리) → Domain(순수 Rust 게임 규칙) → Infrastructure(SQLite/TOML/리소스). 렌더링, 음향, 메달 물리, 입력, 저장은 독립 Plugin으로 구성한다. 핵심 도메인은 Bevy에 의존하지 않으며 헤드리스 테스트가 가능해야 한다.

이벤트: PowerOn, PowerOff, MedalInserted, BetPressed, LeverPulled, ReelStarted(id), StopPressed(id), ReelStopped(id), GameCompleted, BonusStarted, BonusCompleted, MedalDispensed(count). 그래픽과 음향은 이벤트를 구독하고 메달 원장은 도메인에서만 변경한다.

## 9. 디렉터리 구조
```text
fruit-star-3d/
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  README.md
  PLAN.md
  AGENTS.md
  CLAUDE.md
  crates/
    fruit-star-app/
    fruit-star-core/       # symbol, reel, credit, game_state, error
    fruit-star-game/       # controller, evaluator, bonus, events
    fruit-star-render/     # cabinet, material, lighting, camera, reel_mesh
    fruit-star-machine/    # lever, button, medal, display, lamp
    fruit-star-audio/
    fruit-star-physics/
    fruit-star-input/
    fruit-star-ui/
    fruit-star-storage/
    fruit-star-config/
  assets/
    machines/fruit_classic/
    symbols/
    textures/
    materials/
    shaders/
    fonts/
    audio/
    environments/
  config/
    application.toml
    graphics.toml
    audio.toml
    controls.toml
    machines/fruit_classic.toml
  design/blender/
  design/concept/
  docs/
  scripts/
  tests/
  benches/
  dist/
  review/
```

## 10. 설정 파일 예시
```toml
[window]
width = 1920
height = 1080
fullscreen = false
vsync = true
[render]
backend = "metal"
quality = "high"
target_fps = 120
bloom = true
shadows = true
render_scale = 1.0
[reel]
count = 3
symbol_count = 21
visible_rows = 3
acceleration_ms = 350
deceleration_ms = 250
settling_ms = 80
```
설정은 Serde로 읽고 범위와 리소스 존재 여부를 검증한다. 오류 발생 시 명확한 메시지와 기본값 복구 옵션을 제공한다. 위 설정 키는 애플리케이션 자체 스키마이며 Bevy API가 아니다.

## 11. 저장 및 통계
SQLite 테이블: machine_profiles, game_sessions, game_history, reel_results, credit_history, user_settings, achievements, schema_version. 게임 ID를 고유 키로 사용하고 게임 결과와 메달 원장 변경을 동일 트랜잭션으로 기록한다. macOS Application Support 경로에 저장한다. 스키마 마이그레이션, 비정상 종료 복구, 기록 초기화 기능을 제공한다. 통계는 플레이 횟수, 시간, 보너스 기록 및 테스트용이며 당첨 예측이나 금전 수익 기능은 제공하지 않는다.

## 12. macOS 개발 환경
```bash
xcode-select --install
brew install git
brew install --cask blender
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update stable
rustup target add aarch64-apple-darwin
rustup component add rustfmt clippy
cargo install cargo-nextest --locked
cargo install cargo-bundle --locked
cargo install cargo-audit --locked
```
IDE는 IntelliJ IDEA 또는 RustRover를 사용한다. Workspace 생성 후 앱 크레이트와 라이브러리 크레이트를 구성하고 의존성 버전을 검증한다.

```bash
cargo run -p fruit-star-app
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo audit
cargo build --release --target aarch64-apple-darwin
```

## 13. 성능 및 최적화
기본 1920×1080에서 60 FPS, 지원 기기에서 120 FPS를 목표로 한다. 120 FPS 프레임 예산은 8.33ms, 60 FPS는 16.67ms이다. 메모리 2GB 이하 목표, 24시간 안정성 시험을 수행한다. Low/Medium/High/Ultra 품질 프리셋, 1K~4K 텍스처, 렌더 배율, 그림자, Bloom, 모션 블러, 메달 개수 제한을 제공한다. GPU instancing, 텍스처 아틀라스, Object Pool, 비동기 로딩, 고정 물리 업데이트를 적용한다. Instruments 및 Metal 프로파일링으로 실측한다.

## 14. 10주 WBS
| 주차 | 업무 | 완료 기준 |
|---|---|---|
| 1 | 환경, Workspace, 아키텍처 | 기본 3D 화면 실행 |
| 2 | 캐비닛, 릴, 심볼 모델 | 게임기 외관 표시 |
| 3 | 릴 회전 및 정지 | 독립 릴 동작 |
| 4 | 버튼, 레버, 입력, 상태 | 기본 플레이 가능 |
| 5 | 게임 규칙, 결과, LED | 게임 기능 완성 |
| 6 | 기계음, 조명, 재질 | 시청각 통합 |
| 7 | 메달 물리, 카메라, 설정 | 조작 경험 완성 |
| 8 | 저장, 통계, 다국어 | 전체 기능 통합 |
| 9 | 오류 수정, 성능 검증 | 품질 기준 검증 |
| 10 | 패키징, 문서화 | 배포 후보 완성 |

각 주차 종료 시 실행 가능한 빌드, 테스트 결과, 미해결 이슈를 기록한다. 모델링 지연 시 후속 테마와 내부 관찰 모드를 후순위로 조정한다.

## 15. 테스트 계획
| ID | 검증 항목 | 합격 기준 |
|---|---|---|
| TC-001 | 앱 실행 | 정상 로딩 |
| TC-002 | 모델 로딩 | 필수 메시 누락 없음 |
| TC-003 | 텍스처 | 깨짐 없음 |
| TC-004 | START | 입력당 이벤트 1회 |
| TC-005 | 릴 회전 | 3개 독립 회전 |
| TC-006 | 릴 정지 | 개별 정지 가능 |
| TC-007 | 심볼 정렬 | 화면과 내부 결과 일치 |
| TC-008 | 중복 입력 | 중복 처리 없음 |
| TC-009 | 결과 평가 | 규칙과 일치 |
| TC-010 | 메달 정합성 | 음수·중복 정산 없음 |
| TC-011 | 음향 | 이벤트와 동기화 |
| TC-012 | 메달 물리 | 비정상 관통 방지 |
| TC-013 | 카메라 | 정상 표시 및 조작 |
| TC-014 | 전체 화면 | 전환 및 복귀 정상 |
| TC-015 | Retina | UI 가독성 확보 |
| TC-016 | 설정 저장 | 재실행 시 복원 |
| TC-017 | DB 복구 | 비정상 종료 후 정합성 |
| TC-018 | 장시간 실행 | 24시간 안정성 |
| TC-019 | 성능 | 기본 60 FPS 목표 |
| TC-020 | 배포 | 개발 도구 없는 Mac에서 실행 |

핵심 로직 커버리지 90% 목표, Clippy 경고 0건, 컴파일 오류 0건, 발견된 메모리 누수 및 비정상 종료 0건을 목표로 한다. 디버그 콘솔은 FPS, 프레임 시간, 메모리, 릴 각도, LED 밝기, Wireframe, 충돌 영역을 표시한다.

## 16. 배포
Release 빌드 → 리소스 포함 .app 생성 → 코드 서명 → 외부 배포 시 Developer ID 공증 및 stapling → DMG 패키징 → 설치 테스트. DMG 자체의 서명·공증 필요 여부도 배포 방식에 따라 검증한다. 개인용 빌드와 외부 배포용 빌드를 구분한다.

산출물: FruitStar3D.app, FruitStar3D-macOS-arm64.dmg, ZIP, SHA256SUMS, RELEASE_NOTES.md, LICENSES.md.

## 17. 위험 관리
- 3D 모델 품질: 초기 외관 시안 및 릴 프로토타입 승인.
- 라이브러리 호환성: 최소 실행 예제 검증 후 버전 고정.
- 유리 및 금속 표현: HDR 환경광과 재질 테스트.
- Metal 성능 및 발열: 실기기 프로파일링, FPS 제한 및 품질 조절.
- 음향 및 물리 동기화: 이벤트 기반 처리, 객체 개수 제한.
- 저작권: 독자적인 리소스 및 라이선스 기록.
- 범위 확대: 첫 게임기 한 대를 우선 완성.

## 18. AI 개발 지침 (CLAUDE.md / AGENTS.md 작성 기준)
수석 Rust/Bevy 개발자로 행동한다. PLAN.md와 docs를 읽고 작업 단위를 계획한다. 실제 API 및 버전을 확인하고 추측하지 않는다. 게임 로직과 렌더링을 분리하고 main.rs에 기능을 집중시키지 않는다. 불필요한 unsafe 및 운영 코드의 무분별한 unwrap/expect를 금지한다. 설정값은 TOML로 분리한다. 목업과 실제 구현을 구분하며 완료되지 않은 기능을 완료로 표시하지 않는다. 실제 게임기 같은 3D 외관과 기계 조작감을 최우선으로 한다. 외부 서버, 실제 결제, 환전 기능을 구현하지 않는다.

각 단계에서 fmt, clippy, nextest, release build를 실행하고 review/IMPLEMENTATION_STATUS.md, TEST_RESULT.md, PERFORMANCE_REPORT.md, ISSUE_LIST.md, RELEASE_CHECKLIST.md에 결과를 기록한다. 실패, 미구현 기능, 성능 미달 및 제약사항을 명시한다.

## 19. 최종 산출물 체크리스트
- [ ] PLAN.md, CLAUDE.md, AGENTS.md
- [ ] ARCHITECTURE.md, GAME_DESIGN.md, GRAPHICS_DESIGN.md, MODELING_GUIDE.md
- [ ] AUDIO_DESIGN.md, UI_DESIGN.md, CODING_GUIDE.md, TEST_PLAN.md, USER_GUIDE.md
- [ ] Rust Workspace 전체 소스코드
- [ ] 독자적인 GLB 모델, 심볼, 텍스처, 음향
- [ ] 릴 엔진, 게임 규칙, 입력, 메달 물리, 저장 및 통계
- [ ] setup/run/build/test/benchmark/package 스크립트
- [ ] 자동화 테스트 및 성능 보고서
- [ ] macOS APP 및 DMG

## 20. 가정 및 검증 상태
이 문서는 개발 계획이며 실행 가능한 게임 소스나 3D 리소스를 포함하지 않는다. 10주 일정, 릴당 21개 위치, 애니메이션 시간, 성능 목표는 초기 가정이다. 실제 Mac에서 빌드·성능·라이브러리 호환성을 검증하지 않았다. 이전 대화의 시각적 시안은 텍스트 명세로 정리했으며 이미지는 포함하지 않았다.

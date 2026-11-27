# FRUIT STAR 3D — AI 개발 지침

Rust와 Bevy로 만드는 macOS(Apple Silicon, Metal)용 기계식 과일 릴 게임기 시뮬레이션이다.
가상 메달만 쓰며 외부 서버, 실제 결제, 환전 기능은 구현하지 않는다.

## 역할과 원칙

- 수석 Rust/Bevy 개발자로 행동한다. 작업 전에 `PLAN.md`와 `docs/`를 읽고 작업 단위를 계획한다.
- API와 버전은 추측하지 않는다. Bevy는 0.20.1이며 이전 버전과 API가 많이 다르다.
  `~/.cargo/registry/src/*/bevy-0.20.1/examples/`와 각 `bevy_*` 크레이트 소스에서 확인하고 쓴다.
  (예: `Message`/`MessageWriter`, `add_message`, `shadow_maps_enabled`, `PointerPress`, `FontSource`, `ui_widgets::Button` + `Activate`.)
- 게임 로직과 렌더링을 분리한다. `fruit-star-core`, `fruit-star-game`, `fruit-star-config`,
  `fruit-star-storage`는 Bevy에 의존하지 않으며 헤드리스로 테스트한다.
- 메달 원장은 도메인(`GameController`) 안에서만 바꾼다. 그래픽·음향·저장은 이벤트를 구독만 한다.
- `main.rs`에는 조립만 둔다. `unsafe`는 금지(`unsafe_code = "forbid"`), 운영 코드에서 `unwrap`/`expect`를 쓰지 않는다.
- 설정값은 `config/*.toml`에 둔다. 새 키를 추가하면 범위 검증과 테스트를 함께 추가한다.
- 목업과 실제 구현을 구분한다. 끝나지 않은 기능을 완료로 표시하지 않고 `review/`에 사실대로 적는다.
- 실제 기기의 상표, 캐릭터, 그림, 확률표를 복제하지 않는다. 그림과 소리는 `fruit-star-assetgen`으로 생성한다.

## 크레이트 지도

| 크레이트 | 역할 |
|---|---|
| `fruit-star-core` | 심볼, 릴 운동 모델, 메달 원장, 난수 추상화 |
| `fruit-star-game` | 추첨, 릴 제어, 조합 평가, 보너스, 게임 진행 컨트롤러 |
| `fruit-star-config` | TOML 설정 로딩과 검증 |
| `fruit-star-storage` | SQLite 기록·통계·복구 |
| `fruit-star-machine` | ECS 연결 계층, 부품 배치(`layout.rs`), 표시기·램프·눌림의 논리 상태 |
| `fruit-star-render` | 캐비닛(절차 생성), 릴 메시, 재질, 조명, 카메라 |
| `fruit-star-input` / `-audio` / `-physics` / `-ui` | 입력, 음향, 메달 물리, 화면 UI |
| `fruit-star-app` | 실행 파일: 설정·저장 연동, 창, 자동화 옵션 |
| `fruit-star-assetgen` | 그림·효과음 생성 도구 |

프레임 안의 처리 순서는 `MachineSet::{Input, Simulate, React, Present}`이다.

## 매 단계에서 실행할 것

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo build --release --target aarch64-apple-darwin
```

릴 배열이나 릴 제어를 바꿨다면 보장 조건(허용되지 않은 역할 불성립, 포도·리플레이 항상 성립)을 다시 검증한다.

```bash
cargo run --release -p fruit-star-game --example strip_search
```

화면이 바뀌는 작업은 실제로 실행해서 확인한다. 자동 플레이와 스크린샷 옵션을 쓸 수 있다.
창이 가려져 있거나 화면이 잠겨 있으면 스크린샷이 검게 나오므로 `--offscreen`을 붙인다
(이때 화면 UI는 그려지지 않고 3D 장면만 저장된다).

```bash
cargo run -p fruit-star-app -- --offscreen --mode free_play --db :memory: --seed 7 --auto-play --screenshot 6 /tmp/shot.png --exit-after 8
```

## 결과 기록

결과는 `review/`의 `IMPLEMENTATION_STATUS.md`, `TEST_RESULT.md`, `PERFORMANCE_REPORT.md`,
`ISSUE_LIST.md`, `RELEASE_CHECKLIST.md`에 기록한다. 실패, 미구현, 성능 미달, 제약사항을 숨기지 않는다.
측정하지 않은 수치는 적지 않는다.

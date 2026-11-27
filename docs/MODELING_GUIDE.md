# MODELING_GUIDE

현재 캐비닛은 Blender에서 만든 GLB가 아니라 `crates/fruit-star-render/src/cabinet.rs`가 Bevy 기본 도형
(직육면체, 원기둥, 구, 원환, 사각형)으로 조립하는 절차 생성 모델이다. `design/blender/`는 비어 있다.
이 문서는 그 모델의 좌표계·치수·부품 이름을 정리하고, 나중에 GLB로 교체할 때 맞춰야 할 조건을 적는다.

재질과 조명은 [GRAPHICS_DESIGN.md](GRAPHICS_DESIGN.md)에 있다.

## 1. 좌표계와 단위

`crates/fruit-star-machine/src/layout.rs`가 기준이다. 렌더링·물리·음향이 같은 상수를 쓴다.

- 단위: 미터.
- 원점: 게임기 아래 바닥의 중심.
- +X: 플레이어가 볼 때 오른쪽, +Y: 위, +Z: 플레이어 쪽.
- 캐비닛 앞면은 `FRONT_Z = 0.18`에 있다.

## 2. 주요 치수

| 부위 | 상수 | 값 (m) |
|---|---|---|
| 받침대 | `PEDESTAL_SIZE` | 0.60 × 0.42 × 0.46 |
| 캐비닛 본체 | `CABINET_WIDTH` × `HEIGHT` × `DEPTH` | 0.52 × 0.84 × 0.36 (바닥 0.42 ~ 윗면 1.26) |
| 외장 판 두께 | `WALL` | 0.014 |
| 릴 | `REEL_RADIUS`, `REEL_WIDTH`, `REEL_GAP` | 반지름 0.135, 폭 0.100, 간격 0.010 |
| 릴 중심 | `reel_center(i)` | x = (i − 1) × 0.110, y = 0.980, z = 0.015 |
| 표시창 | `WINDOW_WIDTH` × `WINDOW_HEIGHT` | 0.330 × 0.115 |
| 릴 패널 | `REEL_PANEL_BOTTOM` ~ `TOP` | y 0.846 ~ 1.098 |
| 상단 패널 | `TOP_PANEL_BOTTOM` ~ `TOP` | y 1.106 ~ 1.250, 간판 폭 0.352 (가로세로 비 3.2) |
| 하단 장식 패널 | `BELLY_BOTTOM` ~ `TOP`, `BELLY_WIDTH` | y 0.548 ~ 0.750, 폭 0.460 |
| 조작부 선반 | `DECK_BOTTOM` ~ `TOP`, `DECK_DEPTH` | y 0.762 ~ 0.836, 앞으로 0.090 돌출 (`DECK_FRONT_Z = 0.27`) |
| 7세그먼트 줄 | `DISPLAY_Y`, `DISPLAY_*_X` | y 0.878, CREDIT x −0.140 / COUNT 0 / PAYOUT 0.150 |
| 숫자 한 자리 | `DIGIT_WIDTH` × `DIGIT_HEIGHT`, `DIGIT_PITCH` | 0.013 × 0.024, 간격 0.019 |
| STAR CHANCE 램프 | `CHANCE_LAMP_POS`, `CHANCE_LAMP_RADIUS` | (−0.201, 0.992, 0.176), 반지름 0.028 |
| 정지 버튼 | `stop_button_pos(i)`, `STOP_BUTTON_RADIUS` | (릴 i의 x, 0.799, 0.27), 반지름 0.022 |
| START 레버 축 | `LEVER_PIVOT`, `LEVER_LENGTH`, `LEVER_KNOB_RADIUS` | (−0.208, 0.800, 0.27), 길이 0.062, 손잡이 반지름 0.021 |
| MAX BET 버튼 | `BET_BUTTON_POS`, `BET_BUTTON_SIZE` | (−0.120, 0.836, 0.228), 0.066 × 0.010 × 0.036 |
| 메달 투입구 | `MEDAL_SLOT_POS` | (0.175, 0.836, 0.226) |
| 메달 받침 | `TRAY_FLOOR_Y`, `TRAY_HALF_WIDTH`, `TRAY_FRONT_Z`, `TRAY_LIP_HEIGHT` | 바닥 y 0.432, 반폭 0.205, 앞끝 z 0.305, 턱 높이 0.046 |
| 메달 배출구 | `CHUTE_POS` | (0, 0.498, 0.186) |
| 메달 | `MEDAL_RADIUS`, `MEDAL_THICKNESS` | 반지름 0.0125, 두께 0.0018 |
| 스피커 | `SPEAKER_X`, `SPEAKER_RADIUS` | x ±0.208, 반지름 0.024 |
| 외곽 LED | `LED_RADIUS`, `LED_SIDE_COUNT`, `LED_TOP_COUNT` | 반지름 0.0075, 좌우 각 16칸, 상단 11개 |

좌우 LED는 16칸 가운데 선반에 가리는 높이의 칸을 건너뛰고 만든다 (현재 치수에서는 한쪽 14개).
치수 사이의 관계(릴이 표시창 안에 들어오는지, 앞면 구획이 겹치지 않는지 등)는
`machine/tests/logic.rs`의 `layout_is_physically_consistent`가 검사한다.

## 3. 부품 이름과 계층

루트는 `FruitStarMachine`(`CabinetRoot` 컴포넌트)이고 변환은 항등이다. 아래 16개 이름은 계획서의
메시 목록과 같으며 `REQUIRED_PARTS`로 정의되어 있다. 각 이름은 모델 안에 정확히 한 번 있어야 한다
(`render/tests/cabinet.rs`의 `all_required_parts_exist_exactly_once`).

| 계획서 노드 | 절차 모델에서의 형태 | 주요 하위 부품 | 움직임 |
|---|---|---|---|
| `Cabinet` | 그룹 | `Back`, `Bottom`, `SideLeft`/`SideRight`, `Top`, `FrontLower`, `BellyPanel`, `Deck`, `Crown` | 고정 |
| `FrontGlass` | 메시 하나 (두께 0.003의 판) | | 고정 |
| `ChromeFrame` | 그룹 | `RailTop`, `RailMarquee`, `RailDeck`, `RailBelly`, `RailBase`, `BellyTrim` | 고정 |
| `TopPanel` | 그룹 | `TopBacking`, `Marquee`(간판 텍스처) | 고정 |
| `ReelLeft`, `ReelCenter`, `ReelRight` | 그룹, 원점이 `reel_center(i)` | 인쇄 띠와 안쪽 슬리브, 양쪽의 타공 끝판·보강 비드·금속 테·부싱·리벳 | X축 회전 |
| `StopButtonLeft`, `StopButtonCenter`, `StopButtonRight` | 그룹 | `StopCollar`(검은 받침), `StopBezel`(크롬 링), `StopCap` → `StopCapSkirt`(원통 몸통), `StopCapDome`(납작한 돔) | `StopCap`이 −Z로 6.0 mm |
| `StartLever` | 그룹 | `LeverPlate`, `LeverPlateRing`, `LeverBoot`(고무 부트), `LeverPivot` → `LeverShaft`, `LeverCollar`, `LeverKnob` | `LeverPivot`이 X축으로 0.50 rad |
| `BetButton` | 그룹 | `ButtonBezel`, `ButtonWell`, `ButtonCap` → `ButtonCapBody`, `ButtonCapTop`(모따기), `ButtonCapPrint`(윗면 글자). 같은 구조의 `BetOneButton`(1 BET)이 옆에 있다 | `ButtonCap`이 −Y로 5.2 mm |
| `MedalSlot` | 그룹 | `SlotPlate`, `SlotHead`(플레이어 쪽으로 0.30 rad 기운 머리), `SlotOpening`, `SlotGuide` × 2 | 부품은 움직이지 않는다. 투입 연출은 별도의 메달 엔티티가 한다 |
| `MedalTray` | 그룹 | `TrayFloor`, `TrayLip`, `TraySide`, `ChuteOpening`, `ChuteHood` | 고정 |
| `LEDPanel` | 그룹 | `Led` 여러 개 | 고정 |
| `Speaker` | 그룹 | `SpeakerLeft`/`SpeakerRight`, `SpeakerRing`, `SpeakerGrill` | 고정 |

계획서 목록에 없는 추가 그룹: `Pedestal`, `ReelPanel`(표시창 테두리), `ReelUnit`(릴 프레임과 스테핑 모터),
`Displays`(7세그먼트), `StarChanceLamp`, `Interior`(호퍼, 전원부, 기판. Mechanical 보기에서 보인다).
주변 공간(`Room`)은 캐비닛과 별개로 `lighting.rs`가 만든다.

### 피벗

| 부품 | 피벗 위치 | 축과 방향 |
|---|---|---|
| 릴 | `reel_center(i)` | X축. 릴 위치 `p`(스텝)일 때 회전각 `+p × 2π/21`. `p`가 커지면 표면이 아래로 흐른다 |
| 레버 | `LEVER_PIVOT` (`LeverPivot` 노드) | X축. 끝까지 당겼을 때 +0.52 rad. 축은 쉬는 자세에서 +Z를 향한다 |
| 정지 버튼, MAX BET | 각 캡의 쉬는 자세 (`RestPose`) | 평행 이동 |

## 4. 코드가 부품에 붙이는 컴포넌트

GLB로 교체하더라도 아래 컴포넌트는 로딩 후 같은 대상에 붙어야 게임이 동작한다.

| 컴포넌트 | 대상 | 쓰는 쪽 |
|---|---|---|
| `ReelDrum(i)` | 릴 그룹 3개 | `sync_reels`가 회전을 쓴다 |
| `Pressable` + `PressMotion` + `RestPose` | `StopCap` × 3, `LeverPivot`, `BetCap` | 눌림 애니메이션. 시간은 기종 설정의 `[animation]` |
| `Pressable` (움직임 없음) | `SlotBlock` | 투입구 입력의 눌림 상태만 기록 |
| `Clickable(MachineCommand)` + `Pickable` | `StopCap`, `ReelBand`, `ReelFlange`(해당 릴 정지), `LeverKnob`, `BetCap`, `SlotBlock`, 1 BET 캡 | 마우스·트랙패드 클릭. 피킹은 표시가 붙은 메시만 검사한다 |
| `Lamp(kind)` | LED 알, `ChanceLens`, `ReelBand` × 3, `StopCap` × 3, `BetCap`, `Marquee`, `BellyPanel` | `apply_lamps`가 재질의 emissive를 바꾼다. 램프마다 **개별 재질 인스턴스**가 필요하다 |
| `SevenSegment { display, digit, segment }` | 획 63개 (CREDIT 4자리, COUNT 3자리, PAYOUT 2자리 × 7획) | 점등·소등 재질 교체 |
| `ShellPart` | 외장(옆판, 윗판, 유리, 패널, 표시기, LED 띠 등) | Mechanical 보기에서 숨긴다. 릴, 레버, 버튼, 받침에는 붙이지 않는다 |

## 5. 릴 메시의 규약 (`reel_mesh.rs`)

- 인쇄 띠의 축은 X이고 중심이 원점이다. 양쪽 어깨에서 반지름이 작아져 띠 끝이 금속 테 안으로 감긴다. 안쪽에는 두께가 있는 슬리브가 있다.
- 메시 각도 θ는 +Z(정면)에서 +Y(위) 방향으로 잰다. 심볼 `k`의 중심은 `θ = k × 2π/21`.
- 텍스처 이음매는 `θ = −반 칸`(심볼 사이 경계)에 둔다.
- UV: `u`는 −X에서 0, +X에서 1. `v`는 띠 텍스처가 위에서부터 "마지막 인덱스 → 0번" 순으로 쌓여 있다고 보고 정한다.
- 인쇄면은 바깥에서 볼 때 반시계 방향이다. 양쪽 끝판은 두께 4 mm이며 중심 축 구멍과 여섯 개의 원형 타공에 안쪽 벽이 있다.
- 고정 부품인 `ReelUnit`에 공통 축, 양단 베어링 지지대와 모터를 붙인다. 끝판과 부싱, 리벳과 구리색 기준점은 각 드럼의 피벗을 따른다.

메시 단위 테스트는 인쇄 띠의 UV·회전 방향과 끝판 면의 법선을 검사한다. 캐비닛 통합 테스트는 각 끝판과 고정축의 부모 노드를 확인한다.

## 6. GLB로 교체할 때의 조건

1. 단위는 미터, 변환을 적용한 뒤 내보낸다. 축 방향은 1절과 같게 한다.
2. 3절의 16개 노드 이름을 그대로, 각각 한 번씩 둔다. 움직이는 부품(릴 3개, 레버, 정지 버튼 캡, MAX BET 캡)은
   독립 노드로 두고 피벗을 3절의 위치에 맞춘다.
3. 치수를 바꾸면 `layout.rs`를 함께 고친다. 물리 충돌체(받침, 캐비닛 앞면), 효과음 위치, 카메라 주시점,
   메달 배출 위치가 모두 이 상수에서 나온다.
4. 릴을 메시로 교체하면 5절의 UV 규약을 지키거나 `image_ops::compose_strip`의 배치를 함께 바꾼다.
   심볼 한 칸의 가로세로 비는 `cell_aspect()`로 계산하므로 반지름·폭이 바뀌면 텍스처 칸 크기도 달라진다.
5. 로딩 후 4절의 컴포넌트를 노드 이름으로 찾아 붙이는 코드를 `fruit-star-render`에 추가한다.
   램프로 쓰는 메시는 재질을 공유하지 않게 한다.
6. 7세그먼트는 획마다 별도 메시(63개)여야 한다. 한 장의 텍스처로 바꾸려면 `apply_segments`를 고쳐야 한다.
7. `render/tests/cabinet.rs`의 테스트가 계속 통과해야 한다 (필수 부품, 릴 위치와 고정축, 조작부·표시기·램프 연결).
8. 새 리소스의 출처와 라이선스를 `assets/ASSET_SOURCES.md`에 기록한다. 이 파일은 `fruit-star-assetgen`이
   생성하므로 생성기의 목록(`manifest.rs`)에 항목을 추가해야 한다.

## 7. 미구현 / 계획과의 차이

- Blender 원본과 GLB 파일이 없다. `design/blender/`와 `assets/machines/fruit_classic/`은 비어 있다.
- 절차 모델은 기본 도형의 조합이라 모서리 둥글림(베벨), 곡면 외장, 세부 음각 같은 형상이 없다.
- 캐비닛용 노멀 맵·러프니스 맵 같은 PBR 텍스처는 없다. 텍스처는 간판, 장식 패널, 라벨, 릴 띠, 램프 렌즈,
  메달 앞면의 base color(및 emissive)뿐이다.
- `MedalSlot`은 눌림 상태만 있고 움직이는 부품이 없다.

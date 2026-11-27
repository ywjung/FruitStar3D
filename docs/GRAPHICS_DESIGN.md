# GRAPHICS_DESIGN

`fruit-star-render`가 실제로 만드는 재질, 조명, 카메라, 텍스처와 그래픽 설정의 효과를 정리한다.
부품의 치수와 이름은 [MODELING_GUIDE.md](MODELING_GUIDE.md)를 본다.

## 1. 재질 (`materials.rs`)

모두 Bevy `StandardMaterial`이다. 자체 셰이더는 없다.

| 재질 | Metallic | Roughness | 그 밖의 설정 | 주로 쓰는 곳 |
|---|---:|---:|---|---|
| 크롬 `chrome` | 1.0 | 0.12 | | 테두리 레일, 버튼 베젤, 레버 축, 표시창 틀, 받침 턱 |
| 스테인리스 `stainless` | 1.0 | 0.30 | | 릴 테·축·리벳, 베어링 하우징, 받침 바닥, 호퍼 |
| 빨간 플라스틱 `red_plastic` | 0.0 | 0.22 | clearcoat 1.0 (roughness 0.08) | 본체 외장, 선반, 크라운 |
| 버튼 `button_material()` | 0.0 | 0.18 | 램프로 쓰므로 부품마다 개별 인스턴스 | 정지 버튼, MAX BET, 레버 손잡이, LED 알 |
| 릴 인쇄면 `reel_print_material()` | 0.0 | 0.36 | 따뜻한 색의 필름, 띠 텍스처로 내부 발광 표현 | 심볼이 인쇄된 드럼 바깥면 |
| 릴 끝판 | 0.65 | 0.38 | 타공과 원형 보강 비드 | 드럼 양단 |
| 유리 `glass` | 0.0 | 0.05 | base color 알파 0.02, reflectance 0.35, `AlphaMode::Premultiplied` | 표시창 앞 유리 |
| 메달 `medal` | 1.0 | 0.25 | `medal_face.png`를 base color 텍스처로 사용 | 메달 |
| 검은 광택 `black_gloss` | 0.0 | 0.16 | | 릴 패널, 선반 판 |
| 검은 무광 `black_matte` | 0.0 | 0.85 | | 내부 가림판, 투입구·배출구 구멍 |
| 어두운 회색 `dark_grey` | 0.0 | 0.55 | | 받침대, 옆자리 게임기 |
| 7세그먼트 `segment_on` / `segment_off` | 0.0 | 0.40 | 점등 시 emissive (9.0, 0.7, 0.35) | 표시기 획 |
| 백라이트 패널 `backlit_material()` | 0.0 | 0.30 | 텍스처를 emissive 텍스처로 사용 | 상단 간판, 하단 장식 패널, 라벨, STAR CHANCE 렌즈 |
| 내부 부품 `pcb_green`, `motor_black`, `copper` | 0 / 0.6 / 1.0 | 0.50 / 0.45 / 0.35 | | 기판, 모터·전원부, 코일 |

유리는 확산광을 거의 내지 않고 반사만 얹도록 설정했다. 유리 뒤 심볼의 가독성은 자동 검사가 없고
스크린샷으로 확인한다 ([TEST_PLAN.md](TEST_PLAN.md)의 TC-003).

## 2. 조명 (`lighting.rs`)

| 요소 | 내용 |
|---|---|
| 환경 반사 | 코드로 만든 HDR 큐브맵(면당 128×128, 반정밀도 부동소수). `parlor_radiance()`가 어두운 실내, 천장 조명 띠, 좌우 네온 패널, 뒤쪽의 점광 무늬를 그린다. 카메라에 `GeneratedEnvironmentMapLight`(intensity 900)로 붙인다 |
| 전역 주변광 | `GlobalAmbientLight`, 색 (0.75, 0.8, 1.0), brightness 18 |
| 주광 `KeyLight` | `SpotLight`, intensity 420,000, 위치 (0.55, 2.55, 1.75) → 주시점 (0, 0.85, 0.05), 내각 0.35 / 외각 0.62 rad. 그림자는 `render.shadows`를 따른다 |
| 보조광 `FillLight` | `PointLight`, intensity 38,000, 위치 (−1.3, 1.25, 1.5), 그림자 없음 |
| 윤곽광 `RimLight` × 2 | `PointLight`, intensity 26,000, 위치 (±1.1, 1.5, 0.15), 분홍·하늘색, 그림자 없음 |
| 배경색 | `ClearColor` (0.004, 0.004, 0.008) |
| 주변 공간 | 14 m × 14 m 바닥, 뒷벽, 옆자리 게임기 4대(어두운 상자와 희미한 발광 화면) |

릴 내부 조명은 인쇄 띠의 발광 재질로 표현한다. 표시창에는 드럼 앞쪽 곡률에 맞춘 반투명 메시를 두어 위아래 턱으로 들어가는 필름을 어둡게 한다. 이 명암은 회전 중에도 창에 고정되며, 내부 관찰에서는 외장과 함께 숨긴다.

### 램프의 발광 (`effects.rs`의 `apply_lamps`)

`emissive = 색 × 밝기 × 종류별 배율`이다. 밝기와 색은 `fruit-star-machine`의 `lamp.rs`가 계산한다.

| 램프 | 배율 | LED 밝기 설정의 영향 |
|---|---:|---|
| 외곽 LED | 16.0 | `led.brightness`를 그대로 곱한다 |
| STAR CHANCE | 12.0 | 그대로 곱한다 |
| 정지 버튼, MAX BET | 5.0 | 그대로 곱한다 |
| 상단 간판 | 1.9 | `0.6 + 0.4 × min(brightness, 1)` (가독성을 위해 덜 어두워진다) |
| 하단 장식 패널 | 1.0 | 위와 같음 |
| 릴 조명 | 0.33 | 위와 같음 |

## 3. 카메라와 후처리 (`camera.rs`)

주 카메라는 하나이며 시작 시 한 번 구성한다.

| 구성 | 값 | 조건 |
|---|---|---|
| HDR 렌더 타깃 | `Hdr` | 항상 |
| 톤 매핑 | `Tonemapping::TonyMcMapface` | 항상 |
| 안티에일리어싱 | `Smaa` (기본 설정), `Msaa::Off` | 항상 |
| 투영 | 원근, 세로 시야각 `camera.fov_degrees`(32°), near 0.05, far 60 | 항상 |
| Bloom | `Bloom::NATURAL`에서 intensity 0.12 | `render.bloom = true` |
| Ambient Occlusion | `ScreenSpaceAmbientOcclusion` 기본값 | `render.ambient_occlusion = true` |
| 모션 블러 | `MotionBlur { shutter_angle: 0.5, samples: 3 }` | `render.motion_blur = true` |
| 피킹 | `MeshPickingCamera`, `require_markers = true` (표시가 붙은 부품만 광선 검사) | 항상 |

회전 중인 릴의 흐림은 이 화면 공간 모션 블러에 맡긴다. 릴 전용 블러 텍스처나 셰이더는 없다.

### 카메라 프리셋

자세는 주시점 중심의 궤도 값(`OrbitPose`: yaw, pitch, 거리)이다. `V` 키로 보기 방식별 목록을 순환하고,
전환은 `camera.transition_ms`(500 ms) 동안 smoothstep으로 보간한다.

| 보기 방식 | 번호 | 주시점 | yaw / pitch (rad) | 거리 (m) | 용도 |
|---|---:|---|---|---:|---|
| Play | 0 | (0, 0.960, 0.14) | 0 / 0.10 | 1.14 | 기본 플레이 시점: 조작부 선반부터 간판까지가 화면을 채운다 |
| | 1 | (0, 0.835, 0.10) | 0 / 0.085 | 1.74 | 게임기 전체 |
| | 2, 3 | 같음 | ∓0.55 / 0.13 | 1.90 | 왼쪽, 오른쪽에서 |
| | 4 | 릴 앞면 | 0 / 0.02 | 0.62 | 릴 확대 |
| Mechanical | 0, 1 | (0, 0.88, 0) | ±0.62 / 0.30 | 1.50 | 비스듬히 |
| | 2 | 릴 축 | 0.95 / 0.18 | 0.62 | 릴 유닛 확대 |
| | 3 | (0, 0.80, 0) | 0 / 1.05 | 1.45 | 위에서 |
| Gallery | 0 | (0, 0.86, 0.05) | 0 / 0.10 | 2.05 | 자동 회전 (메뉴 화면과 같은 자세) |
| | 1, 2 | Play와 같음 | ∓0.9 / 0.2 | 1.70 | 왼쪽, 오른쪽에서 |

- 메뉴 화면과 Gallery 0번에서는 yaw가 ±0.55 rad 범위로 천천히 흔들린다.
- 드래그·스크롤·핀치로 직접 움직이면 전환과 자동 회전을 멈추고 그 자리에서 조작한다.
  한계는 yaw ±1.9 rad, pitch −0.12 ~ 1.25 rad, 거리 0.42 ~ 3.2 m다.
- Mechanical에서는 `ShellPart` 표시가 붙은 외장 부품을 숨긴다 (`effects.rs`의 `apply_view_mode`).

## 4. 릴 띠 텍스처와 밉맵 (`assets.rs`, `image_ops.rs`)

로딩 단계에서 릴마다 띠 텍스처 한 장을 CPU로 합성한다.

1. `symbols/*.png`(1024×512 RGBA) 9장과 `textures/reel_band.png`를 읽는다.
2. 칸 크기를 정한다. 너비는 품질 프리셋의 `symbol_texture_width()`, 높이는 릴 표면의 실제 가로세로 비
   (`cell_aspect` = 릴 폭 ÷ 한 칸의 호 길이 ≈ 2.476)로 나눈 값이다.
3. 배경 무늬를 칸 너비에 맞춰 늘리고 세로로 반복해 깐다.
4. 각 심볼을 비율을 유지한 채 칸 높이의 94%(너비의 96%) 안에 맞춰 가운데에 알파 합성한다.
   맨 위 칸이 마지막 인덱스, 맨 아래 칸이 0번이다. 릴 메시의 UV가 이 배치를 전제로 한다.
5. 1×1까지의 밉 체인을 직접 만든다 (감마 보정, 알파 가중 평균).

| 품질 | 칸 너비 | 띠 텍스처 크기 (계산값, 21칸) |
|---|---:|---|
| Low | 256 | 256 × 2163 |
| Medium, High | 512 | 512 × 4347 |
| Ultra | 1024 | 1024 × 8694 |

텍스처 형식은 `Rgba8UnormSrgb`, 필터는 확대·축소·밉 모두 선형, 비등방성 16이다. 띠는 세로 방향만 반복하고
나머지 텍스처(간판, 장식 패널, 라벨, 램프 렌즈, 메달)는 가장자리를 고정한다. 이 텍스처들도 같은 밉 체인 생성을 거친다.

릴 원통(`reel_mesh.rs`)은 인쇄 띠와 슬리브, 타공된 두 끝판으로 구성된다. 텍스처 이음매는 심볼 사이 경계에 오고,
릴 위치 `p`(스텝)는 X축 회전 `p × 2π/21`로 표현한다. 둘레 분할 수는 품질 프리셋을 따른다.

## 5. 품질 프리셋과 설정의 실제 효과

`config/graphics.toml`의 `render.quality`가 바꾸는 것은 다음 세 가지다 (`config/graphics.rs`).

| 항목 | Low | Medium | High | Ultra |
|---|---:|---:|---:|---:|
| 심볼 칸 텍스처 너비 | 256 | 512 | 512 | 1024 |
| 릴 원통 둘레 분할 수 | 42 | 84 | 126 | 168 |
| 그림자 맵 크기 (`DirectionalLightShadowMap`, `PointLightShadowMap`) | 1024 | 2048 | 2048 | 4096 |

세 값 모두 시작·로딩 때 한 번만 읽는다. 일시정지 화면에서 품질을 바꾸면 값은 저장되지만 다음 실행부터 적용된다.

| 설정 키 | 허용 범위 | 실제 효과 |
|---|---|---|
| `render.backend` | `"metal"`만 | 검증만 한다. 렌더러 설정에 전달하지 않는다 |
| `render.target_fps` | 30 ~ 240 | 프레임 끝에서 대기해 상한을 건다. vsync가 켜져 있고 모니터 주사율이 목표 이하이면 대기하지 않는다 |
| `render.bloom`, `ambient_occlusion`, `motion_blur` | bool | 시작 시 카메라에 해당 컴포넌트를 붙일지 결정 |
| `render.shadows` | bool | 주광의 그림자 맵 사용 여부 (시작 시) |
| `render.render_scale` | 0.25 ~ 2.0 | **반영되지 않는다** (아래 6절) |
| `render.max_medals` | 0 ~ 500 | 화면에 남는 물리 메달 수 상한. 넘으면 가장 오래된 메달을 재사용한다 |
| `led.brightness` | 0 ~ 1.5 | 램프 발광 배율. 실행 중 바로 반영 |
| `led.reduce_flashing` | bool | 점멸을 느리고 얕은 변화로 바꾼다. 실행 중 바로 반영 ([UI_DESIGN.md](UI_DESIGN.md) 참고) |
| `camera.transition_ms` | 0 ~ 5000 | 카메라 전환 시간 |
| `camera.fov_degrees` | 15 ~ 90 | 세로 시야각 (시작 시) |

창 크기, 전체 화면, vsync는 `config/application.toml`의 `[window]`에 있다. 전체 화면은 테두리 없는
전체 화면(`BorderlessFullscreen`)이며 `F` 키로 전환한다.

## 6. 미구현 / 계획과의 차이

| 계획서 | 실제 |
|---|---|
| Blender에서 만든 GLB 모델 | 캐비닛 전체를 코드로 생성한다 (`cabinet.rs`). GLB 로딩 코드는 없다 |
| HDR 환경광 (파일) | 코드로 만든 HDR 큐브맵. `assets/environments/`는 비어 있다 |
| 렌더 배율 | `render_scale`은 읽고 범위만 검증한다. 렌더 타깃 크기에 쓰는 코드가 없다 |
| 품질 프리셋 전환 | 재시작 후 적용. Bloom·그림자·AO·모션 블러는 설정 화면에 없고 TOML로만 바꾼다 |
| 그림자 품질 | 프리셋 값을 `DirectionalLightShadowMap`(스포트라이트 그림자가 따르는 설정)과 `PointLightShadowMap` 양쪽에 넣는다. 프리셋별 그림자 품질 차이를 화면으로 비교해 보지는 않았다 |
| WGSL 셰이더, `assets/shaders/`, `assets/materials/` | 자체 셰이더와 재질 파일이 없다. 두 디렉터리는 비어 있다 |
| 1K ~ 4K 텍스처 | 띠 텍스처는 위 표의 크기다. 원본 심볼은 1024×512 한 종류다 |
| GPU instancing | 명시적인 인스턴싱 코드는 없다. LED 알은 메시 하나를 공유하지만 재질은 알마다 따로 만든다 |
| 텍스처 아틀라스 | 릴마다 21칸을 한 장으로 합성한 띠 텍스처가 그 역할을 한다. 그 밖의 아틀라스는 없다 |
| Retina 지원 | 전용 코드는 없다. 창의 배율 처리는 Bevy 기본 동작에 맡긴다. 가독성은 수동 확인 대상이다 |
| 유리 반사와 가독성 검증 | 자동 검사가 없다. 스크린샷 확인 결과는 `review/`에 기록한다 |

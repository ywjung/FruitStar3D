# ASSET_SOURCES — 에셋 출처 목록

> 이 문서는 `fruit-star-assetgen` 이 자동으로 생성합니다. 직접 수정하지 말고 워크스페이스 루트에서 `cargo run -p fruit-star-assetgen --release` 를 다시 실행하세요.

## 요약

- 아래 표의 모든 파일은 `crates/fruit-star-assetgen` 의 Rust 코드가 **절차적으로 생성한 독자적(original) 저작물**이며, FRUIT STAR 3D 프로젝트를 위해 새로 만들었습니다.
- 제3자의 그림, 사진, 음원, 샘플, 폰트, 로고, 상표, 캐릭터, 기존 슬롯머신 아트는 **전혀 사용하지 않았습니다.** 인터넷에서 내려받은 자료도 없습니다.
- 글자는 시스템 폰트가 아니라 도구에 내장된 자체 제작 stroke font (`src/font.rs`, "Fruit Star Stroke")로 그립니다. `resvg` 의 text / system-fonts 기능은 빌드에서 꺼 두었습니다.
- 그림은 코드가 만든 SVG(`design/concept/`)를 `resvg` 로 rasterize 한 PNG 이고, 효과음은 oscillator · filtered noise · envelope 만으로 합성한 WAV 입니다. 멜로디(`bonus_start`, `bonus_end` 등)는 이 프로젝트를 위해 새로 지은 것입니다.
- 난수는 고정 seed 의 xorshift(`src/rng.rs`)만 사용하므로 같은 toolchain 에서 다시 실행하면 동일한 결과가 나옵니다.
- 라이선스: 프로젝트 본체와 동일한 라이선스(워크스페이스 `Cargo.toml` 의 `license` 항목)를 따릅니다.

## 릴 심볼 (`assets/symbols/`)

| 파일 | 형식 | 생성 모듈 | 내용 | 출처 |
| --- | --- | --- | --- | --- |
| `assets/symbols/grape.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | 보라색 포도송이 + 잎 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/cherry.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | 쌍둥이 체리 + 줄기와 잎 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/red_seven.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | 빨간 7 (흰 inner outline, gold drop edge) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/blue_seven.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | 파란 7 (silver edge, 별 cut-out 과 stripe) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/bar.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | 검은 plate + gold 테두리 + stroke font "BAR" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/bell.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | 황금 종 + 빨간 리본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/melon.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | 줄무늬 수박 + 빨간 조각 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/star.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | bevel 음영의 gold 별 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/symbols/replay.png` | 1024×512 RGBA PNG (투명 배경) | `symbols.rs` | teal 리본 + stroke font "REPLAY" + 원형 화살표 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |

## 캐비닛 텍스처 (`assets/textures/`)

| 파일 | 형식 | 생성 모듈 | 내용 | 출처 |
| --- | --- | --- | --- | --- |
| `assets/textures/menu_marquee.png` | 960×336 PNG (불투명, sRGB) | `panels.rs` | 메뉴·로딩 간판에 쓰는 FRUIT STAR 3D 레터링과 체리 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/top_panel.png` | 2048×640 PNG (불투명, sRGB) | `panels.rs` | 상단 backlit marquee: stroke font "FRUIT STAR" 타이틀, 별, 과일 (emissive 용) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/belly_panel.png` | 2048×900 PNG (불투명, sRGB) | `panels.rs` | 하단 장식 패널: sunburst, 큰 과일 그림, "FRUIT CLASSIC" 리본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/reel_band.png` | 256×256 PNG (불투명, sRGB, tileable) | `panels.rs` | 릴 띠 배경: cream 색 + 미세한 noise + 좌우 가장자리 선 (세로 반복) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/credit.png` | 362×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "CREDIT" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/payout.png` | 405×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "PAYOUT" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/count.png` | 357×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "COUNT" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/star_chance.png` | 720×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "STAR CHANCE" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/max_bet.png` | 477×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "MAX BET" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/start.png` | 344×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "START" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/stop.png` | 283×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "STOP" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/insert.png` | 738×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "INSERT MEDAL" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/one_bet.png` | 322×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "1 BET" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/replay.png` | 396×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "REPLAY" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/games.png` | 362×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "GAMES" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/big.png` | 174×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "BIG" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/reg.png` | 215×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "REG" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/labels/total.png` | 342×128 PNG (RGBA, 투명 배경) | `panels.rs` | 흰색 stroke font 라벨 "TOTAL" | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/paytable.png` | 2048×256 PNG (RGBA, 투명 배경) | `panels.rs` | 릴 패널에 인쇄되는 배당표: 심볼 9종과 세 개가 늘어섰을 때의 배당 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/star_lamp.png` | 512×512 PNG (RGBA, 투명 배경) | `panels.rs` | STAR CHANCE 램프 렌즈용 흰색 별 (부드러운 gradient) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/textures/medal_face.png` | 512×512 PNG (RGBA, 투명 배경) | `panels.rs` | 게임 메달 앞면: brass 원판, 돋은 테두리, 점 장식, 양각 별 (실제 주화 모방 없음) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/icons/ios/AppIcon-1024.png` | 1024×1024 PNG (불투명) | `icon.rs` | iPhone 앱 아이콘: 모서리를 깎지 않은 정사각형 (iOS가 둥근 모서리를 씌운다) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/icons/app_icon.png` | 1024×1024 PNG (투명 배경) | `icon.rs` | 앱 아이콘: 빨간 타일 위의 금색 별과 빨간 7 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/icons/AppIcon.iconset/` | PNG 10개 (16~1024 px) | `icon.rs` | macOS iconset. 패키징 때 `iconutil`로 `AppIcon.icns`를 만든다 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |

## 효과음 (`assets/audio/`)

| 파일 | 형식 | 생성 모듈 | 내용 | 출처 |
| --- | --- | --- | --- | --- |
| `assets/audio/power_on.wav` | 1.50 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | relay clicks, hum swell and rising chime | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/ambient.wav` | 8.00 s, 44.1 kHz 16-bit mono WAV, peak -18 dBFS, seamless loop | `audio.rs` | quiet machine-fan / arcade-room hum (seamless loop) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/lever_pull.wav` | 0.25 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | lever latch, travel and sprung end stop | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/lever_return.wav` | 0.18 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | lighter spring snap-back | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/reel_start.wav` | 0.40 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | stepper engaging and accelerating, with paper drum chatter | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/reel_loop.wav` | 2.25 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS, seamless loop | `audio.rs` | stepper and bearings under 28 indexing ticks/s, seamless over three turns | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/reel_stop.wav` | 0.18 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | brake shoe closes, pawl rebounds and the chassis damps | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/button_press.wav` | 0.08 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | crisp plastic click with a short 1.8 kHz tick tone | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/button_release.wav` | 0.06 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | softer, lower click up | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/medal_insert.wav` | 0.40 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | slot lip strike, rolling guide and muffled acceptor gate | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/medal_drop.wav` | 0.30 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | hopper pawl indexing and a medal sliding out through the chute | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/medal_collision.wav` | 0.10 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | short, damped edge contact between loose medals | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/medal_tray.wav` | 0.26 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | coin edge ringing over the hollow modes of the metal payout tray | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/bonus_start.wav` | 2.80 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | original fanfare: rising arpeggios over C-F-G, snare, held tremolo chord | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/bonus_end.wav` | 1.80 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | original descending chip-tone phrase resolving on the tonic | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/menu_select.wav` | 0.10 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | soft two-step UI blip | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/win_small.wav` | 0.60 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | rising chip-tone arpeggio C6-E6-G6 with sparkle on the last note | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/chance_lamp.wav` | 0.70 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | sharp rising FM/ring-mod shimmer sweep with echo tail | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/bet.wav` | 0.09 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | short soft-square beep at A5 (880 Hz), made to be pitch-shifted | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/replay.wav` | 0.45 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | two-tone signal E6-B5, repeated once | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/win_big.wav` | 1.40 s, 44.1 kHz 16-bit mono WAV, peak -3 dBFS | `audio.rs` | original big-win jingle: arpeggio runs over C and F, sparkling C7 finish | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `assets/audio/bonus_loop.wav` | 12.80 s, 44.1 kHz 16-bit mono WAV, peak -9 dBFS, seamless loop | `audio.rs` | original chiptune bonus music, 8 bars at 150 BPM in C major (seamless loop) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |

## 디자인 원본 · 참고 자료 (`design/concept/`)

| 파일 | 형식 | 생성 모듈 | 내용 | 출처 |
| --- | --- | --- | --- | --- |
| `design/concept/symbols/grape.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `grape` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/cherry.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `cherry` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/red_seven.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `red_seven` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/blue_seven.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `blue_seven` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/bar.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `bar` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/bell.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `bell` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/melon.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `melon` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/star.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `star` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols/replay.svg` | SVG, viewBox 0 0 1024 512 | `symbols.rs` | `replay` 심볼의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/symbols_sheet.png` | 1536×880 RGB PNG | `main.rs` | 릴 배경색 위에 9개 심볼을 모아 본 contact sheet (가독성 확인용) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/menu_marquee.svg` | SVG, viewBox 0 0 960 336 | `panels.rs` | `menu_marquee.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/top_panel.svg` | SVG, viewBox 0 0 2048 640 | `panels.rs` | `top_panel.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/belly_panel.svg` | SVG, viewBox 0 0 2048 900 | `panels.rs` | `belly_panel.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/reel_band.svg` | SVG, viewBox 0 0 256 256 | `panels.rs` | `reel_band.png` 의 vector 원본 (종이 질감 noise 는 raster 단계에서 추가) | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/credit.svg` | SVG, viewBox 0 0 362 128 | `panels.rs` | `labels/credit.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/payout.svg` | SVG, viewBox 0 0 405 128 | `panels.rs` | `labels/payout.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/count.svg` | SVG, viewBox 0 0 357 128 | `panels.rs` | `labels/count.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/star_chance.svg` | SVG, viewBox 0 0 720 128 | `panels.rs` | `labels/star_chance.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/max_bet.svg` | SVG, viewBox 0 0 477 128 | `panels.rs` | `labels/max_bet.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/start.svg` | SVG, viewBox 0 0 344 128 | `panels.rs` | `labels/start.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/stop.svg` | SVG, viewBox 0 0 283 128 | `panels.rs` | `labels/stop.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/insert.svg` | SVG, viewBox 0 0 738 128 | `panels.rs` | `labels/insert.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/one_bet.svg` | SVG, viewBox 0 0 322 128 | `panels.rs` | `labels/one_bet.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/replay.svg` | SVG, viewBox 0 0 396 128 | `panels.rs` | `labels/replay.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/games.svg` | SVG, viewBox 0 0 362 128 | `panels.rs` | `labels/games.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/big.svg` | SVG, viewBox 0 0 174 128 | `panels.rs` | `labels/big.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/reg.svg` | SVG, viewBox 0 0 215 128 | `panels.rs` | `labels/reg.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/labels/total.svg` | SVG, viewBox 0 0 342 128 | `panels.rs` | `labels/total.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/paytable.svg` | SVG, viewBox 0 0 2048 256 | `panels.rs` | `paytable.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/star_lamp.svg` | SVG, viewBox 0 0 512 512 | `panels.rs` | `star_lamp.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/panels/medal_face.svg` | SVG, viewBox 0 0 512 512 | `panels.rs` | `medal_face.png` 의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/icon/app_icon.svg` | SVG, viewBox 0 0 1024 1024 | `icon.rs` | 앱 아이콘의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |
| `design/concept/icon/app_icon_ios.svg` | SVG, viewBox 0 0 1024 1024 | `icon.rs` | iPhone 앱 아이콘의 vector 원본 | 이 프로젝트를 위해 만든 독자적 절차 생성물 (제3자 자료 없음) |

## 재생성 방법

```sh
cargo run -p fruit-star-assetgen --release            # 워크스페이스 루트에서 실행
cargo run -p fruit-star-assetgen --release -- --root <path>
```

생성 직후 도구가 모든 WAV 의 sample rate / channel / 길이 / peak / loop 연속성과 심볼 PNG 의 여백(잘림 여부)을 자동으로 검사하며, 같은 검사가 `cargo test -p fruit-star-assetgen` 에도 들어 있습니다.

# 릴리스 체크리스트

기준일: 2026-11-26 · 버전 0.1.0 · 개인용(ad-hoc 서명) 빌드 기준

## 빌드와 검사

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (경고 0건)
- [x] `cargo nextest run --workspace` (전부 통과, `TEST_RESULT.md`)
- [x] 릴 배열 보장 조건 검증 (`strip_search`)
- [x] `cargo build --release --target aarch64-apple-darwin`
- [x] `cargo audit` (취약점 0건, 경고 10건)
- [x] 릴리스 바이너리 실행 확인 (자동 플레이)
- [ ] 코드 커버리지 측정 (90% 목표)
- [ ] 24시간 안정성 시험
- [ ] Instruments / Metal 프로파일링

## 기능 확인 (수동)

- [x] 로딩 → 메뉴 → 플레이 진입 (스크린샷으로 확인)
- [x] 자동 플레이로 베팅·레버·정지·지급·표시기 동작 확인
- [x] 내부 관찰 모드 화면 확인
- [x] 키보드 조작 전 항목을 손으로 확인 (B·Space·1·2·3·V·Esc·M·F, 11월 27일)
- [x] 마우스·트랙패드 조작 (부품 클릭·드래그·스크롤, 트랙패드 핀치는 사용자 확인)
- [ ] 전체 화면 전환과 복귀
- [ ] 다른 앱으로 전환했다가 돌아왔을 때의 입력 상태
- [ ] 효과음 청취, 음량·음소거
- [x] 11월 27일 합성 음원 22개 검증, 메달 충돌 재질 구분 테스트, 새 번들의 자동 지급 및 Core Audio 출력 확인
- [x] 일본어·영어 메뉴·일시정지 화면 (Metal 캡처)
- [ ] 종료 후 재실행 시 설정과 보유 메달 복원
- [ ] 보너스 시작부터 종료까지 실제 플레이
- [x] 기록표 열기·닫기, 초기화 확인 문구, HUD 조작표, 시점 전환, 일시정지 버튼의 포인터 입력

## 패키징

- [x] `scripts/package.sh`로 `.app`, ZIP, DMG, `SHA256SUMS`, `LICENSES.md` 생성
- [x] ad-hoc 서명과 `codesign --verify`
- [x] 11월 27일 음향 반영 후 재빌드한 앱에서 arm64 실행 파일의 도움말 출력, 원본과 번들의 에셋·설정 일치, ZIP 무결성과 ZIP·DMG 체크섬 확인
- [x] 11월 26일 UI를 포함한 앱·ZIP·DMG 갱신 및 ZIP·DMG 체크섬 검증
- [x] 11월 27일 릴 드럼이 포함된 arm64 앱·ZIP·DMG 갱신, 번들 서명 및 배포물 체크섬 검증
- [x] 새 드럼의 정면·회전·옆면을 릴리스 번들에서 확인하고 macOS 창으로 내부 관찰 확대 실행
- [x] 번들(.app)에서 실행 확인 (만든 Mac에서, 다른 작업 디렉터리 기준)
- [ ] Developer ID 서명
- [ ] 공증(notarization)과 stapling
- [ ] DMG 서명·공증 필요 여부 확인
- [ ] 개발 도구가 없는 Mac에서 설치·실행

## 문서와 리소스

- [x] `PLAN.md`, `CLAUDE.md`, `AGENTS.md`, `README.md`
- [x] `docs/` 설계·가이드 문서 9종
- [x] `assets/ASSET_SOURCES.md` (리소스 출처와 라이선스)
- [ ] 프로젝트 라이선스 확정과 LICENSE 파일
- [x] `review/` 보고서 5종과 릴리스 노트
- [ ] Blender 원본과 GLB 모델

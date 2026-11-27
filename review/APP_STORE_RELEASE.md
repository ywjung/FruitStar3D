# App Store 출시 준비 (iPhone · macOS)

작성: 2026-11-27. 번들 ID `io.github.ywjung.fruitstar3d`, 무료, 앱 내 구입 없음.
iOS와 macOS를 한 앱 레코드에 넣어 같은 번들 ID로 낸다(유니버설 구입).

## 1. 이 저장소에서 끝낸 것

| 항목 | 상태 | 위치 |
|---|---|---|
| 번들 ID 변경 | 완료 | `scripts/package.sh`, `scripts/package-ios.sh` 기본값 |
| 개인정보 매니페스트 | 완료 | `packaging/PrivacyInfo.xcprivacy` (추적·수집 없음, 신고 대상 API 3종) |
| 수출 규정(암호화) | 완료 | 두 Info.plist에 `ITSAppUsesNonExemptEncryption = false` |
| macOS 샌드박스 | 완료·확인 | `packaging/macos/store.entitlements`. ad-hoc 서명 샌드박스 실행에서 기록 DB가 컨테이너(`~/Library/Containers/io.github.ywjung.fruitstar3d/…`)에 생기고 플레이가 정상 |
| macOS 카테고리 | 완료 | `public.app-category.casino-games` |
| iOS 빌드 정보 키 | 완료 | `DTSDKName`, `DTXcode` 등을 실제 SDK에서 채움, `UIRequiredDeviceCapabilities` = arm64·metal |
| 빌드 번호 | 완료 | `BUILD_NUMBER` 환경 변수 (업로드마다 1씩 올린다) |
| 스토어 패키징 | 완료(서명 단계 미실행) | `STORE=1 scripts/package.sh` → `.pkg`, `scripts/package-ios.sh store` → `.ipa` |
| 스크린샷 | 완료 | `store/screenshots/iphone/` 1320×2868 3장, `store/screenshots/macos/` 2880×1800 3장 |
| 설명문·키워드·심사 메모 | 초안 | `store/metadata.md` (한·영·일) |
| 개인정보 처리방침 | 초안 | `docs/PRIVACY.md` (저장소를 공개 push하면 URL로 쓸 수 있다) |

`packaging/PrivacyInfo.xcprivacy`의 API 신고는 실행 파일의 외부 심볼을 `nm -u`로 확인해 정했다:
`stat/fstat/lstat`(C617.1), `statfs/fstatfs`(E174.1), `mach_absolute_time`(35F9.1). 의존성을 바꾸면 다시 확인한다.

## 2. 막혀 있는 것

**iOS: UIScene.** 이 Mac에는 Xcode 27.2(iOS 27 SDK)만 있다. iOS 27 SDK로 빌드한 앱은 UIScene 생명주기가
필수인데, Bevy 0.20이 쓰는 winit 0.30.13은 지원하지 않아 실행 즉시 멈춘다. 개발용 빌드에서 쓰는 SDK 버전 변경(vtool)은
심사용으로 쓸 수 없으므로 `package-ios.sh store`는 iOS 27 이상 SDK면 빌드를 거부한다.
해결: Xcode 26을 추가 설치해 iOS 26 SDK로 빌드한다 (Apple이 iOS 27 SDK를 의무화하기 전까지 유효).
그 뒤로는 winit의 UIScene 지원(0.31 계열, 현재 베타)과 Bevy 업그레이드가 필요하다.

## 2-1. 계정 작업 진행 상황 (2026-11-27)

| 항목 | 상태 |
|---|---|
| App ID `io.github.ywjung.fruitstar3d` (팀 6C6BHG2XX3) | 등록함 |
| 인증서 Apple Distribution, 3rd Party Mac Developer Installer | Xcode에서 만들어 키체인에 있음 (2027-11-27 만료) |
| 프로파일 "FRUIT STAR 3D iOS App Store", "FRUIT STAR 3D Mac App Store" | 만들어 Xcode로 내려받음 (`~/Library/Developer/Xcode/UserData/Provisioning Profiles/`) |
| App Store Connect 앱 레코드 (Apple ID 6839970897, SKU fruitstar3d, iOS·macOS, 한국어) | 만듦. 두 플랫폼 1.0 "제출 준비 중" |
| Mac App Store 패키지 `dist/store/FruitStar3D-macOS.pkg` (1.0, 빌드 1) | 배포 인증서로 서명함. 앱 아이콘을 icns와 에셋 카탈로그(`Assets.car`, `CFBundleIconName`) 둘 다로 넣음. 2026-11-27 20:01 Xcode 계정으로 업로드 성공(`xcodebuild -exportArchive`, destination upload), 처리 완료(약 10분, "제출 준비 완료"). macOS 1.0 버전에 빌드 1을 연결해 저장했고, 앱 목록·버전 화면에 앱 아이콘이 표시됨. 심사 제출은 하지 않음 |
| iOS `.ipa` `dist/ios-store/FruitStar3D.ipa` (1.0, 빌드 1) | Xcode 26.6(iOS 26.5 SDK)로 빌드, 배포 인증서로 서명. 에셋 카탈로그는 Xcode 26 actool이 시뮬레이터 런타임을 요구해 `ACTOOL_DEVELOPER_DIR`로 Xcode 27의 actool을 씀. 2026-11-27 20:56 `xcodebuild -exportArchive`로 업로드 성공, 처리 완료("제출 준비 완료") |
| 앱 정보 (2026-11-27 입력) | 부제 "3D 기계식 과일 릴 게임기", 카테고리 게임 › 카지노·시뮬레이션, 콘텐츠 권한 "타사 콘텐츠 없음" |
| 연령 등급 | 시뮬레이션 도박 "빈번", 나머지 없음/아니요 → 계산 결과 18+ (일부 지역 R 18+). 대한민국은 조건에 따라 GRAC 등급분류번호 필요 |
| 가격·판매 지역 | 무료 ($0.00). 판매 지역 131개 국가·지역 — 대한민국(GRAC 등급분류번호 필요 가능), 중국 본토(게임 판호 필요), 유럽 42개국(사용자 요청) 제외 |
| 개인정보 | 처리방침 URL 입력, "데이터를 수집하지 않음" 게시함 (사용자 승인) |
| iOS·macOS 1.0 버전 문구 | 한국어 홍보 문구·설명·키워드·지원 URL·저작권·심사 메모 입력. macOS 심사 연락처 저장, "로그인 필요" 해제, 스크린샷 3장(2880×1800) Chrome으로 업로드 |
| **macOS 1.0 심사 제출** | 2026-11-27 20:45 제출. 상태 "심사 대기 중" (최대 48시간, 결과는 계정 이메일로 옴) |
| iOS 1.0 첫 제출 (빌드 1) | 스크린샷 3장(6.3형 1206×2622, `store/screenshots/iphone-6.3/`), 심사 연락처, "로그인 필요" 해제, 빌드 1.0 (1) 연결 후 2026-11-27 21:04 제출. 3분 뒤 자동 검사에서 "잘못된 바이너리"(제출 상태 "해결되지 않은 문제"). App Store Connect에는 이유가 표시되지 않음(계정 이메일로 옴, 미확인) |
| 빌드 2 (도구 통일, 원인 아님) | 빌드 1은 `Assets.car`만 Xcode 27.2 actool로 만들어 도구가 섞여 있었다. iOS 26.5 시뮬레이터 런타임(23F73)을 받고 `simctl runtime match set iphoneos26.5 23F73`으로 Xcode 26 actool이 쓰게 해 모두 Xcode 26.6으로 만든 빌드 2를 21:36에 다시 제출했으나 같은 이유로 거절됨 |
| **실제 원인: ITMS-90111** | Apple 메일: "Unsupported SDK or Xcode version - App submissions must use the latest Xcode and SDK Release Candidates". 두 Xcode 모두 정식(GM) 라이선스였다. Xcode가 만든 다른 iOS 앱(OnEum, S2B)과 비교하니 손으로 쓴 Info.plist에 `DTPlatformBuild`만 빠져 있었고, 나머지 DT 값은 Xcode 26.6 빌드 설정(`xcodebuild -showBuildSettings`)과 같았다. `package-ios.sh`가 플랫폼 `version.plist`의 `ProductBuildVersion`(23F81a)을 `DTPlatformBuild`로 쓰게 고쳐 빌드 3을 21:44에 업로드 |
| **iOS 1.0 재제출 (빌드 3)** | 2026-11-27 21:49 제출. 빌드 1·2는 제출 3분 안에 거절됐는데, 빌드 3은 6분 뒤(21:55)에도 "심사 대기 중". 최종 결과는 Apple 심사 뒤에 나온다 |
| **iOS 빌드 4 업로드** | 2026-11-28 07:25, 사용자 요청으로 커밋 12f7993(휴대폰 화면 배치, 발열 대응, 자동화 종료 수정)을 담아 빌드 4를 올림(`BUILD_NUMBER=4`, Xcode 26.6·iOS 26.5 SDK, `DTPlatformBuild` 23F81a 포함, `codesign --verify` 통과, `xcodebuild -exportArchive` 업로드 성공). 그때 iOS 1.0 (빌드 3)과 macOS 1.0은 각각 다른 제출로 "심사 대기 중"이었다 |
| **iOS 1.0 재제출 (빌드 4)** | 2026-11-28 07:31 빌드 4 처리 완료(VALID) 확인 뒤, 사용자 결정으로 iOS 제출을 심사에서 제거("개발자가 취소함")하고 빌드 3을 빼고 빌드 4를 넣어 저장, 07:33 다시 제출("1개의 항목 제출됨"). 대기 순서는 처음부터 다시 시작한다. macOS 1.0 제출은 따로 있어 그대로 "심사 대기 중". 제출 4분 뒤(07:37)에도 iOS는 "심사 대기 중", 버전에 연결된 빌드는 4. 빌드 4의 화면 배치는 실기기에서 보지 않았다 |

## 3. 직접 해야 하는 일 (계정·서명·제출)

1. **Xcode 26 설치**: developer.apple.com › Downloads에서 받아 `/Applications/Xcode-26.app`처럼 이름을 바꿔 함께 둔다.
2. **인증서** (Xcode › Settings › Accounts › Manage Certificates):
   `Apple Distribution`, `Mac Installer Distribution`(이름이 `3rd Party Mac Developer Installer`로 표시될 수 있음).
3. **App ID 등록** (Certificates, Identifiers & Profiles): `io.github.ywjung.fruitstar3d`, 추가 기능 없음.
4. **프로비저닝 프로파일**: iOS "App Store Connect" 배포용(.mobileprovision), macOS "Mac App Store Connect" 배포용(.provisionprofile).
5. **App Store Connect 앱 레코드**: 플랫폼 iOS + macOS, 번들 ID 위와 동일, SKU 예: `fruitstar3d`.
6. **앱 정보 입력**: `store/metadata.md`의 문구, 개인정보 처리방침 URL, 지원 URL, 카테고리.
7. **앱 개인정보**: "데이터를 수집하지 않음".
8. **연령 등급 설문**: 아래 4절.
9. **빌드 만들기·업로드**:

```bash
DEVELOPER_DIR=/Applications/Xcode-26.app/Contents/Developer MARKETING_VERSION=1.0 BUILD_NUMBER=1 SIGN_IDENTITY="Apple Distribution: …" PROFILE_PATH=<iOS 배포 프로파일> scripts/package-ios.sh store
```

```bash
STORE=1 MARKETING_VERSION=1.0 BUILD_NUMBER=1 SIGN_IDENTITY="Apple Distribution: …" INSTALLER_IDENTITY="3rd Party Mac Developer Installer: …" PROFILE_PATH=<Mac 배포 프로파일> scripts/package.sh
```

   만든 `dist/ios-store/FruitStar3D.ipa`와 `dist/store/FruitStar3D-macOS.pkg`를 Transporter 앱으로 올린다.
   Transporter 없이 Xcode에 로그인된 계정으로 올릴 수도 있다: 서명된 앱을 `.xcarchive`(Products/Applications + Info.plist)로
   묶고 `xcodebuild -exportArchive -exportOptionsPlist <method app-store-connect, destination upload> -allowProvisioningUpdates`.
   2026-11-27 macOS 1.0 (1)은 이 방법으로 올렸다 (`dist/store/ExportOptions.plist`).
10. **TestFlight**로 실기기에서 한 번 더 확인한 뒤 심사 제출. 심사 메모는 `store/metadata.md` 마지막 절.

## 4. 연령 등급 설문 답변 (예상)

| 질문 | 답 |
|---|---|
| 시뮬레이션 도박 (Simulated Gambling) | **잦음/강함 (Frequent/Intense)**: 게임 전체가 슬롯 시뮬레이션 |
| 도박 (실제 돈 또는 환전 가능한 재화) | 아니요 |
| 콘테스트, 사용자 생성 콘텐츠, 채팅, 웹 접근 | 없음 |
| 폭력·성적 내용·욕설·약물·공포 | 없음 |
| 앱 내 구입·광고 | 없음 |

시뮬레이션 도박이 잦은 앱은 높은 연령 등급(현재 체계에서 18+ 예상)을 받는다. 최종 등급은 Apple이 정한다.
일부 지역은 도박 관련 앱을 제한할 수 있으므로 판매 지역을 확인한다.

## 5. 남은 확인

- 배포 인증서로 서명·업로드는 두 플랫폼 모두 했다. TestFlight 실기기 설치는 하지 않았다.
- 스토어 빌드 3과 같은 도구·구성(Xcode 26.6, iOS 26.5 SDK, SDK 버전 변경 없음)으로 만든 개발 서명 앱을 iPhone 18 Pro에 설치해 실행과 40초·180초 자동 플레이(평균 59.9 FPS, 180초는 30.3초 멈춤 1회 제외)를 확인했다. App Store 서명 빌드 자체는 TestFlight로 설치해 보지 않았다.
- macOS 샌드박스는 ad-hoc 서명으로만 확인했다. 배포 프로파일로 서명한 `.pkg` 설치 확인은 하지 않았다.
- 대한민국은 GRAC 등급분류 문제로 판매 지역에서 빠져 있다.
- 실제 빈 셰이더 캐시: 새 번들 ID의 샌드박스 앱 첫 실행에서 렌더 예열이 6,634 ms 걸렸고 이어진 플레이 최대 프레임은
  10.5 ms였다(두 번째 실행 예열 244 ms). 첫 실행의 로딩 화면이 몇 초 길어질 수 있다.

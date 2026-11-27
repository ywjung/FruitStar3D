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
| iOS `.ipa` | Xcode 26 설치 전이라 만들지 않음 |

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

- 실제 배포 인증서로 서명·업로드·TestFlight 설치는 아직 하지 않았다(인증서 없음).
- iOS 26 SDK 정식 빌드는 Xcode 26이 없어 만들지 못했다. 그 빌드의 실기기 실행 확인이 필요하다.
- macOS 샌드박스는 ad-hoc 서명으로만 확인했다. 배포 프로파일로 서명한 `.pkg` 설치 확인이 필요하다.
- 실제 빈 셰이더 캐시: 새 번들 ID의 샌드박스 앱 첫 실행에서 렌더 예열이 6,634 ms 걸렸고 이어진 플레이 최대 프레임은
  10.5 ms였다(두 번째 실행 예열 244 ms). 첫 실행의 로딩 화면이 몇 초 길어질 수 있다.

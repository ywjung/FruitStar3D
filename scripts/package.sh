#!/usr/bin/env bash
# macOS 배포물 생성: FruitStar3D.app, ZIP, DMG, SHA256SUMS, LICENSES.md.
#
# 기본은 개인용 빌드(ad-hoc 서명)다. 외부 배포용으로 서명하려면
#   SIGN_IDENTITY="Developer ID Application: 이름 (TEAMID)" scripts/package.sh
# 처럼 서명 ID를 지정한다. 공증(notarization)과 stapling은 이 스크립트가 하지 않는다.
#
# Mac App Store용 (STORE=1): 샌드박스·개인정보 매니페스트·프로비저닝 프로파일을 넣고
# 서명한 설치 패키지 dist/store/FruitStar3D-macOS.pkg를 만든다. DMG·ZIP은 만들지 않는다.
#   STORE=1 BUILD_NUMBER=1 \
#   SIGN_IDENTITY="Apple Distribution: 이름 (TEAMID)" \
#   INSTALLER_IDENTITY="3rd Party Mac Developer Installer: 이름 (TEAMID)" \
#   PROFILE_PATH=<Mac App Store용 .provisionprofile> scripts/package.sh
# STORE=1에서 서명 ID를 비우면 ad-hoc 서명으로 샌드박스 동작만 확인하는 번들을 만든다.
set -euo pipefail
cd "$(dirname "$0")/.."

STORE="${STORE:-0}"
BUNDLE_ID="${BUNDLE_ID:-io.github.ywjung.fruitstar3d}"

VERSION="$(cargo metadata --format-version 1 --no-deps |
    python3 -I -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="fruit-star-app"))')"
TARGET="aarch64-apple-darwin"
BUILD_NUMBER="${BUILD_NUMBER:-$VERSION}"
DIST="dist"
[ "$STORE" = 1 ] && DIST="dist/store"
APP="$DIST/FruitStar3D.app"
SIGN_IDENTITY="${SIGN_IDENTITY:--}"
if [ "$STORE" = 1 ] && [ "$SIGN_IDENTITY" != "-" ]; then
    : "${PROFILE_PATH:?Mac App Store 서명에는 PROFILE_PATH가 필요합니다}"
    : "${INSTALLER_IDENTITY:?Mac App Store 패키지에는 INSTALLER_IDENTITY가 필요합니다}"
fi
mkdir -p "$DIST"

cargo build --release --target "$TARGET" -p fruit-star-app

rm -rf "$APP" "$DIST/FruitStar3D-macOS-arm64.dmg" "$DIST/FruitStar3D-macOS-arm64.zip"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "target/$TARGET/release/fruit-star-app" "$APP/Contents/MacOS/FruitStar3D"
cp -R assets config "$APP/Contents/Resources/"
# 앱 아이콘: 에셋 생성기가 만든 iconset을 icns로 묶는다.
iconutil -c icns assets/icons/AppIcon.iconset -o "$APP/Contents/Resources/AppIcon.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>FruitStar3D</string>
    <key>CFBundleDisplayName</key><string>FRUIT STAR 3D</string>
    <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
    <key>CFBundleExecutable</key><string>FruitStar3D</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>$BUILD_NUMBER</string>
    <key>CFBundleSupportedPlatforms</key><array><string>MacOSX</string></array>
    <key>LSMinimumSystemVersion</key><string>13.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.casino-games</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>ITSAppUsesNonExemptEncryption</key><false/>
    <key>NSHumanReadableCopyright</key><string>Virtual medals only. No real-money gambling.</string>
</dict>
</plist>
PLIST

if [ "$STORE" = 1 ]; then
    cp packaging/PrivacyInfo.xcprivacy "$APP/Contents/Resources/PrivacyInfo.xcprivacy"
    WORK="$(mktemp -d)"
    trap 'rm -rf "$WORK"' EXIT
    cp packaging/macos/store.entitlements "$WORK/entitlements.plist"
    if [ "$SIGN_IDENTITY" != "-" ]; then
        cp "$PROFILE_PATH" "$APP/Contents/embedded.provisionprofile"
        security cms -D -i "$PROFILE_PATH" > "$WORK/profile.plist"
        TEAM_ID="$(/usr/libexec/PlistBuddy -c "Print :TeamIdentifier:0" "$WORK/profile.plist")"
        /usr/libexec/PlistBuddy -c "Add :com.apple.application-identifier string $TEAM_ID.$BUNDLE_ID" "$WORK/entitlements.plist"
        /usr/libexec/PlistBuddy -c "Add :com.apple.developer.team-identifier string $TEAM_ID" "$WORK/entitlements.plist"
    fi
    codesign --force --sign "$SIGN_IDENTITY" --entitlements "$WORK/entitlements.plist" \
        --options runtime "$APP"
    codesign --verify --deep --strict "$APP"
    if [ "$SIGN_IDENTITY" != "-" ]; then
        productbuild --component "$APP" /Applications --sign "$INSTALLER_IDENTITY" \
            "$DIST/FruitStar3D-macOS.pkg"
        echo "완료: $DIST/FruitStar3D-macOS.pkg (업로드: Transporter 앱 또는 xcrun altool)"
    else
        echo "완료: $APP (ad-hoc 서명 샌드박스 확인용, 제출 불가)"
    fi
    exit 0
fi

codesign --force --deep --sign "$SIGN_IDENTITY" "$APP"
codesign --verify --deep --strict "$APP"

# 의존성 라이선스 목록
cargo metadata --format-version 1 | python3 -I -c '
import json, sys
meta = json.load(sys.stdin)
workspace = set(meta["workspace_members"])
rows = sorted(
    (p["name"], p["version"], p.get("license") or "(명시 없음)")
    for p in meta["packages"] if p["id"] not in workspace
)
print("# 서드파티 라이선스\n")
print("FruitStar3D에 포함된 Rust 크레이트와 각 크레이트가 선언한 라이선스 목록이다.")
print("그림과 효과음은 모두 이 프로젝트의 절차 생성 원본이다 (assets/ASSET_SOURCES.md).\n")
print("| 크레이트 | 버전 | 라이선스 |\n|---|---|---|")
for name, version, license in rows:
    print(f"| {name} | {version} | {license} |")
' > "$DIST/LICENSES.md"

if [ -f review/RELEASE_NOTES.md ]; then
    cp review/RELEASE_NOTES.md "$DIST/RELEASE_NOTES.md"
fi

ditto -c -k --keepParent "$APP" "$DIST/FruitStar3D-macOS-arm64.zip"
hdiutil create -quiet -volname "FRUIT STAR 3D" -srcfolder "$APP" -ov -format UDZO \
    "$DIST/FruitStar3D-macOS-arm64.dmg"
(cd "$DIST" && shasum -a 256 FruitStar3D-macOS-arm64.zip FruitStar3D-macOS-arm64.dmg > SHA256SUMS)

echo "완료:"
ls -lh "$DIST"

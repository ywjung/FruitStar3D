#!/usr/bin/env bash
# iPhone 앱 번들을 만든다.
#
#   scripts/package-ios.sh            시뮬레이터용  → dist/ios-sim/FruitStar3D.app
#   scripts/package-ios.sh device     실기기용      → dist/ios-device/FruitStar3D.app
#
# 실기기용은 Apple 개발자 서명이 필요하다. 다음 환경 변수를 지정한다.
#   SIGN_IDENTITY   codesign 서명 ID (예: "Apple Development: 이름 (XXXXXXXXXX)")
#   PROFILE_PATH    이 번들 ID를 허용하는 프로비저닝 프로파일(.mobileprovision) 경로
#   BUNDLE_ID       프로파일과 일치하는 번들 ID (기본값 local.fruitstar3d)
# 팀 ID와 권한(entitlements)은 프로파일에서 읽는다.
set -euo pipefail
cd "$(dirname "$0")/.."

KIND="${1:-sim}"
case "$KIND" in
    sim) TARGET=aarch64-apple-ios-sim; SDK=iphonesimulator; PLATFORM=iPhoneSimulator ;;
    device) TARGET=aarch64-apple-ios; SDK=iphoneos; PLATFORM=iPhoneOS ;;
    *) echo "사용법: $0 [sim|device]" >&2; exit 2 ;;
esac

MIN_IOS=17.0
BUNDLE_ID="${BUNDLE_ID:-local.fruitstar3d}"
OUT="dist/ios-$KIND"
APP="$OUT/FruitStar3D.app"
VERSION="$(cargo metadata --format-version 1 --no-deps |
    python3 -I -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="fruit-star-app"))')"

if [ "$KIND" = device ]; then
    : "${SIGN_IDENTITY:?실기기용은 SIGN_IDENTITY가 필요합니다}"
    : "${PROFILE_PATH:?실기기용은 PROFILE_PATH가 필요합니다}"
fi

export IPHONEOS_DEPLOYMENT_TARGET="$MIN_IOS"
cargo build --release --target "$TARGET" -p fruit-star-app

rm -rf "$APP"
mkdir -p "$APP"
cp "target/$TARGET/release/fruit-star-app" "$APP/FruitStar3D"
# iOS 27 SDK로 빌드한 앱은 UIScene 생명주기를 쓰지 않으면 실행 직후 UIKit이 강제로 멈춘다.
# Bevy 0.20이 쓰는 winit 0.30은 아직 UIScene을 지원하지 않으므로, 실행 파일에 기록된 SDK
# 버전을 26.0으로 낮춰 이전 방식(UIApplication 단독)으로 실행되게 한다. 앱 스토어 제출에는
# 쓸 수 없는 임시 조치이며, winit이 UIScene을 지원하면 제거한다.
LINKED_SDK="$(xcrun vtool -show-build "$APP/FruitStar3D" | awk '/ sdk /{print $2; exit}')"
if [ "${LINKED_SDK%%.*}" -ge 27 ]; then
    BUILD_PLATFORM=$([ "$KIND" = sim ] && echo iossim || echo ios)
    xcrun vtool -set-build-version "$BUILD_PLATFORM" "$MIN_IOS" 26.0 -replace \
        -output "$APP/FruitStar3D.tmp" "$APP/FruitStar3D" 2>/dev/null
    mv "$APP/FruitStar3D.tmp" "$APP/FruitStar3D"
    chmod +x "$APP/FruitStar3D"
fi
# 리소스: 앱 실행 파일 옆의 assets/와 config/를 찾는다 (crates/fruit-star-app/src/paths.rs).
cp -R assets "$APP/assets"
rm -rf "$APP/assets/icons"
cp -R config "$APP/config"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# 아이콘: 1024px 원본 하나에서 actool이 필요한 크기와 Assets.car를 만든다.
ICONSET="$WORK/Assets.xcassets/AppIcon.appiconset"
mkdir -p "$ICONSET"
cp assets/icons/ios/AppIcon-1024.png "$ICONSET/AppIcon-1024.png"
cat > "$WORK/Assets.xcassets/Contents.json" <<'JSON'
{ "info": { "author": "xcode", "version": 1 } }
JSON
cat > "$ICONSET/Contents.json" <<'JSON'
{
  "images": [
    { "filename": "AppIcon-1024.png", "idiom": "universal", "platform": "ios", "size": "1024x1024" }
  ],
  "info": { "author": "xcode", "version": 1 }
}
JSON
xcrun actool "$WORK/Assets.xcassets" --compile "$APP" --platform "$SDK" \
    --minimum-deployment-target "$MIN_IOS" --target-device iphone \
    --app-icon AppIcon --output-partial-info-plist "$WORK/icon.plist" >/dev/null

cat > "$APP/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>ko</string>
    <key>CFBundleDisplayName</key><string>FRUIT STAR</string>
    <key>CFBundleExecutable</key><string>FruitStar3D</string>
    <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleName</key><string>FruitStar3D</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>$VERSION</string>
    <key>CFBundleSupportedPlatforms</key><array><string>$PLATFORM</string></array>
    <key>LSRequiresIPhoneOS</key><true/>
    <key>MinimumOSVersion</key><string>$MIN_IOS</string>
    <key>UIDeviceFamily</key><array><integer>1</integer></array>
    <key>UILaunchScreen</key><dict><key>UIColorName</key><string></string></dict>
    <key>UIRequiresFullScreen</key><true/>
    <key>UIStatusBarHidden</key><true/>
    <key>UIViewControllerBasedStatusBarAppearance</key><false/>
    <key>UISupportedInterfaceOrientations</key>
    <array><string>UIInterfaceOrientationPortrait</string></array>
    <key>UIApplicationSupportsIndirectInputEvents</key><true/>
</dict>
</plist>
PLIST
/usr/libexec/PlistBuddy -c "Merge $WORK/icon.plist" "$APP/Info.plist" >/dev/null
plutil -lint "$APP/Info.plist" >/dev/null

if [ "$KIND" = sim ]; then
    codesign --force --sign - --timestamp=none "$APP"
else
    cp "$PROFILE_PATH" "$APP/embedded.mobileprovision"
    security cms -D -i "$PROFILE_PATH" > "$WORK/profile.plist"
    /usr/libexec/PlistBuddy -x -c "Print :Entitlements" "$WORK/profile.plist" > "$WORK/entitlements.plist"
    # 와일드카드 프로파일(팀ID.*)이면 앱 ID를 실제 번들 ID로 바꿔 서명한다.
    TEAM_ID="$(/usr/libexec/PlistBuddy -c "Print :TeamIdentifier:0" "$WORK/profile.plist")"
    /usr/libexec/PlistBuddy -c "Set :application-identifier $TEAM_ID.$BUNDLE_ID" "$WORK/entitlements.plist"
    # 이 앱이 쓰지 않는 권한(다른 앱의 앱 그룹 등)은 빼고 서명한다.
    /usr/libexec/PlistBuddy -c "Delete :com.apple.security.application-groups" "$WORK/entitlements.plist" 2>/dev/null || true
    codesign --force --sign "$SIGN_IDENTITY" --entitlements "$WORK/entitlements.plist" \
        --timestamp=none "$APP"
fi
codesign --verify --deep --strict "$APP"

echo "완료: $APP ($(du -sh "$APP" | cut -f1))"
if [ "$KIND" = sim ]; then
    echo "시뮬레이터에 설치: xcrun simctl install booted \"$APP\" && xcrun simctl launch booted $BUNDLE_ID"
else
    echo "기기에 설치: xcrun devicectl device install app --device <기기 이름 또는 UDID> \"$APP\""
fi

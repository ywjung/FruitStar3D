#!/usr/bin/env bash
# macOS 배포물 생성: FruitStar3D.app, ZIP, DMG, SHA256SUMS, LICENSES.md.
#
# 기본은 개인용 빌드(ad-hoc 서명)다. 외부 배포용으로 서명하려면
#   SIGN_IDENTITY="Developer ID Application: 이름 (TEAMID)" scripts/package.sh
# 처럼 서명 ID를 지정한다. 공증(notarization)과 stapling은 이 스크립트가 하지 않는다.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="$(cargo metadata --format-version 1 --no-deps |
    python3 -I -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="fruit-star-app"))')"
TARGET="aarch64-apple-darwin"
DIST="dist"
APP="$DIST/FruitStar3D.app"
SIGN_IDENTITY="${SIGN_IDENTITY:--}"

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
    <key>CFBundleIdentifier</key><string>local.fruitstar3d</string>
    <key>CFBundleExecutable</key><string>FruitStar3D</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>$VERSION</string>
    <key>LSMinimumSystemVersion</key><string>13.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.simulation-games</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

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

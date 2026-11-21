#!/bin/sh
# Builds "Hex Meetings.app" for Apple silicon and packs it into dist/ as
# Hex-Meetings-macOS-arm64.zip (for the installer) and .dmg (to drag into
# Applications).
#
# Signing is optional. With SIGN_IDENTITY set to a "Developer ID Application"
# identity in the keychain, the app and disk image are signed with it; with
# NOTARY_APPLE_ID, NOTARY_TEAM_ID and NOTARY_PASSWORD (an app-specific
# password) as well, they're notarized and stapled. Without them the app is
# ad-hoc signed, which is enough to run on Apple silicon.
set -eu
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
name="Hex-Meetings-macOS-arm64"
dist="$PWD/dist"
app="$dist/Hex Meetings.app"

cargo build --locked --release --bin hex-meetings --target aarch64-apple-darwin

rm -rf "$dist"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/aarch64-apple-darwin/release/hex-meetings "$app/Contents/MacOS/Hex Meetings"
sed "s/@VERSION@/$version/g" assets/Info.plist > "$app/Contents/Info.plist"

# The Finder and Dock icon, from the 1024 px render.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
iconset="$work/AppIcon.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" assets/icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" assets/icon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"

if [ -n "${SIGN_IDENTITY:-}" ]; then
    codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$app"
else
    codesign --force --sign - "$app"
fi
codesign --verify --strict --verbose=2 "$app"

# The disk image: the app beside a link to Applications.
stage="$work/dmg"
mkdir -p "$stage"
ditto "$app" "$stage/Hex Meetings.app"
ln -s /Applications "$stage/Applications"
hdiutil create -quiet -volname "Hex Meetings" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$dist/$name.dmg"

if [ -n "${SIGN_IDENTITY:-}" ]; then
    codesign --force --timestamp --sign "$SIGN_IDENTITY" "$dist/$name.dmg"
    if [ -n "${NOTARY_APPLE_ID:-}" ] && [ -n "${NOTARY_TEAM_ID:-}" ] && [ -n "${NOTARY_PASSWORD:-}" ]; then
        # Notarizing the disk image covers the app inside it too.
        xcrun notarytool submit "$dist/$name.dmg" --wait \
            --apple-id "$NOTARY_APPLE_ID" --team-id "$NOTARY_TEAM_ID" --password "$NOTARY_PASSWORD"
        xcrun stapler staple "$dist/$name.dmg"
        xcrun stapler staple "$app"
    fi
fi

ditto -c -k --keepParent "$app" "$dist/$name.zip"
(cd "$dist" && shasum -a 256 "$name.zip" "$name.dmg" > SHA256SUMS)
echo "Hex Meetings $version: $dist/$name.zip, $dist/$name.dmg"

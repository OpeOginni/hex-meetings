#!/bin/sh
# Installs the latest Hex Meetings release into /Applications (or
# ~/Applications when /Applications isn't writable). Run it again to update.
#
#   curl -fsSL https://raw.githubusercontent.com/OpeOginni/hex-meetings/main/scripts/install.sh | sh
set -eu

url="${HEX_MEETINGS_ZIP:-https://github.com/OpeOginni/hex-meetings/releases/latest/download/Hex-Meetings-macOS-arm64.zip}"

if [ "$(uname -s)" != Darwin ]; then
    echo "Hex Meetings is a macOS app." >&2
    exit 1
fi
if [ "$(uname -m)" != arm64 ]; then
    echo "Hex Meetings needs an Apple silicon Mac, like Hex itself." >&2
    exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

echo "Downloading Hex Meetings..."
curl -fL --progress-bar -o "$work/app.zip" "$url"
ditto -x -k "$work/app.zip" "$work"
if [ ! -d "$work/Hex Meetings.app" ]; then
    echo "The download didn't contain Hex Meetings.app." >&2
    exit 1
fi

dest=/Applications
if [ ! -w "$dest" ]; then
    dest="$HOME/Applications"
    mkdir -p "$dest"
fi

if pgrep -xq "Hex Meetings"; then
    osascript -e 'quit app "Hex Meetings"' >/dev/null 2>&1 || true
    sleep 1
fi
rm -rf "$dest/Hex Meetings.app"
ditto "$work/Hex Meetings.app" "$dest/Hex Meetings.app"

version=$(defaults read "$dest/Hex Meetings.app/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo "")
echo "Installed Hex Meetings $version in $dest."
echo "Open it from Spotlight or Launchpad, or run: open -a \"Hex Meetings\""

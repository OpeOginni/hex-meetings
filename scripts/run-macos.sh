#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

cargo build --locked --bin hex-meetings --target-dir target
app="$PWD/target/Hex Meetings.app"
mkdir -p "$app/Contents/MacOS"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
sed "s/@VERSION@/$version/g" assets/Info.plist > "$app/Contents/Info.plist"
cp target/debug/hex-meetings "$app/Contents/MacOS/Hex Meetings"
exec "$app/Contents/MacOS/Hex Meetings" "$@"

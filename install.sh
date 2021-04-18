#!/bin/sh

set -eu

DESTDIR="${DESTDIR:-/}"
PREFIX="${PREFIX:-"$DESTDIR/usr/local"}"
RELEASES_URL="https://github.com/soywod/himalaya/releases"

system=$(uname -s | tr [:upper:] [:lower:])

case $system in
  msys*|mingw*|cygwin*|win*) system=windows;;
  linux|freebsd) system=linux;;
  darwin) system=macos;;
  *) echo "Error: Unsupported system: $system"; exit 1;;
esac

if ! tmpdir=$(mktemp -d); then
  echo "Error: Failed to create tmpdir"
  exit 1
else
  trap "rm -rf $tmpdir" EXIT
fi

echo "Downloading latest $system release…"
curl -sLo "$tmpdir/himalaya.tar.gz" "$RELEASES_URL/latest/download/himalaya-$system.tar.gz"

echo "Installing binary…"
tar -xzf "$tmpdir/himalaya.tar.gz" -C "$tmpdir"

if [ -w "$PREFIX" ]; then
  mkdir -p "$PREFIX/bin"
  cp -f -- "$tmpdir/himalaya.exe" "$PREFIX/bin/himalaya"
else
  sudo mkdir -p "$PREFIX/bin"
  sudo cp -f -- "$tmpdir/himalaya.exe" "$PREFIX/bin/himalaya"
fi

printf '%s installed!\n' "$("$PREFIX/bin/himalaya" --version)"

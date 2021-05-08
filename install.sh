#!/bin/sh

set -eu

die() {
    printf '%s\n' "$1" >&2
    exit "${2-1}"
}

DESTDIR="${DESTDIR:-}"
PREFIX="${PREFIX:-"$DESTDIR/usr/local"}"
RELEASES_URL="https://github.com/soywod/himalaya/releases"

system=$(uname -s | tr [:upper:] [:lower:])
case $system in
  msys*|mingw*|cygwin*|win*) system=windows; binary=himalaya.exe;;
  linux|freebsd) system=linux; binary=himalaya;;
  darwin) system=macos; binary=himalaya;;
  *) die "Unsupported system: $system" ;;
esac

tmpdir=$(mktemp -d) || die "Failed to create tmpdir"
trap "rm -rf $tmpdir" EXIT

echo "Downloading latest $system release…"
curl -sLo "$tmpdir/himalaya.tar.gz" \
     "$RELEASES_URL/latest/download/himalaya-$system.tar.gz"

echo "Installing binary…"
tar -xzf "$tmpdir/himalaya.tar.gz" -C "$tmpdir"

mkdir -p "$PREFIX/bin"
cp -f -- "$tmpdir/$binary" "$PREFIX/bin/$binary"

die "$("$PREFIX/bin/$binary" --version) installed!" 0

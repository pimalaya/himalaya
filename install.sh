#!/bin/sh

set -eu

die() {
    printf '%s\n' "$1" >&2
    exit "${2-1}"
}

DESTDIR="${DESTDIR:-}"
PREFIX="${PREFIX:-"$DESTDIR/usr/local"}"
RELEASES_URL="https://github.com/soywod/himalaya/releases"

binary=himalaya
system=$(uname -s | tr [:upper:] [:lower:])
machine=$(uname -m | tr [:upper:] [:lower:])

case $system in
    msys*|mingw*|cygwin*|win*)
	target=x86_64-windows
	binary=himalaya.exe;;

    linux|freebsd)
	case $machine in
	    x86_64) target=x86_64-linux;;
	    arm64|aarch64) target=aarch64-linux;;
	    *) die "Unsupported machine $machine for system $system";;
	esac;;

    darwin)
	case $machine in
	    x86_64) target=x86_64-macos;;
	    arm64|aarch64) target=aarch64-macos;;
	    *) die "Unsupported machine $machine for system $system";;
	esac;;

    *)
	die "Unsupported system $system";;
esac

tmpdir=$(mktemp -d) || die "Cannot create temporary directory"
trap "rm -rf $tmpdir" EXIT

echo "Downloading latest $system release…"
curl -sLo "$tmpdir/himalaya.tgz" \
     "$RELEASES_URL/latest/download/himalaya.$target.tgz"

echo "Installing binary…"
tar -xzf "$tmpdir/himalaya.tgz" -C "$tmpdir"

mkdir -p "$PREFIX/bin"
cp -f -- "$tmpdir/$binary" "$PREFIX/bin/$binary"

die "$("$PREFIX/bin/$binary" --version) installed!" 0

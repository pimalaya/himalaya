#!/bin/sh

REPO=https://github.com/soywod/himalaya

die() {
    printf '%s: %s\n' "${0##*/}" "$1" >&2
    exit "${2-1}"
}

set -e

case $(uname -s | tr [:upper:] [:lower:]) in
    *bsd*|linux*) os=linux ;;
    darwin*) os=macos ;;
    cygwin*|mingw*|win*) os=windows ;;
    *) die 'Unable to detect host operating system.' ;;
esac

printf 'Downloading latest %s release…\n' "$os" >&2
trap "rm -f \"$himalaya.tar.gz\" himalaya.exe" EXIT
curl -sSLo "$himalaya.tar.gz" \
    "$REPO/releases/latest/download/himalaya-$os.tar.gz"

printf 'Installing binary…\n' >&2
tar -zx himalaya.exe -f "$himalaya.tar.gz"
mkdir -p /usr/local/bin
cp -f himalaya.exe /usr/local/bin/himalaya
chmod a+rx /usr/local/bin/himalaya

die "$(himalaya --version) installed!" 0

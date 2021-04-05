#!/bin/sh

set -eu

DESTDIR="${DESTDIR:-/}"
PREFIX="${PREFIX:-"$DESTDIR/usr/local"}"

releases_url="https://github.com/soywod/himalaya/releases"

uname_os() {
  os=$(uname -s | tr '[:upper:]' '[:lower:]')

  case $os in
    msys*) os="windows" ;;
    mingw*) os="windows" ;;
    cygwin*) os="windows" ;;
    win*) os="windows" ;;
  esac

  echo "$os"
}

get_system() {
  case $(uname_os) in
    linux | freebsd) system="linux" ;;
    darwin) system="macos" ;;
    windows) system="windows" ;;
  esac

  echo "$system"
}

start() {
  system=$(get_system)
  if [ -z "$system" ]; then
    echo "Error: Unsupported system: $system"
    exit 1
  fi

  if ! tmpdir=$(mktemp -d); then
    echo "Error: Failed to create tmpdir"
    exit 1
  else
    trap 'rm -rf $tmpdir' EXIT
  fi

  echo "Downloading latest $system release…"
  curl -sLo "$tmpdir/himalaya.tar.gz" "$releases_url/latest/download/himalaya-$system.tar.gz"

  echo "Installing binary…"
  tar -xzf "$tmpdir/himalaya.tar.gz" -C "$tmpdir"

  if [ -w "$PREFIX" ]; then
    mkdir -p "$PREFIX/bin"
    install "$tmpdir/himalaya.exe" "$PREFIX/bin/himalaya"
  else
    sudo mkdir -p "$PREFIX/bin"
    sudo install "$tmpdir/himalaya.exe" "$PREFIX/bin/himalaya"
  fi

  echo "$("$PREFIX/bin/himalaya" --version) installed!"
}

start

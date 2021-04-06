#!/bin/bash

case $OSTYPE in
    linux-gnu|freebsd*) OS=linux ;;
    darwin*) OS=macos ;;
    cygwin|msys|win32) OS=windows ;;
esac

cd /tmp
echo "Downloading latest ${OS} release…"
curl -sLo himalaya.tar.gz "https://github.com/soywod/himalaya/releases/latest/download/himalaya-${OS}.tar.gz"
echo "Installing binary…"
tar -xzf himalaya.tar.gz
rm himalaya.tar.gz
chmod u+x himalaya.exe
sudo mv himalaya.exe /usr/local/bin/himalaya

echo "$(himalaya --version) installed!"

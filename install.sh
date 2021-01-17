#!/bin/bash

get_os () {
  if [[ "$OSTYPE" == "linux-gnu" ]]; then
    echo "linux"
  elif [[ "$OSTYPE" == "freebsd"* ]]; then
    echo "linux"
  elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "macos"
  elif [[ "$OSTYPE" == "cygwin" ]]; then
    echo "windows"
  elif [[ "$OSTYPE" == "msys" ]]; then
    echo "windows"
  elif [[ "$OSTYPE" == "win32" ]]; then
    echo "windows"
  else
    return -1
  fi
}

OS=`get_os`

cd /tmp
echo "Downloading latest ${OS} release…"
curl -sLo himalaya.tar.gz "https://github.com/soywod/himalaya/releases/latest/download/himalaya-${OS}.tar.gz"
echo "Installing binary…"
tar -xzf himalaya.tar.gz
rm himalaya.tar.gz
chmod u+x himalaya.exe
sudo mv himalaya.exe /usr/local/bin/himalaya

echo "$(himalaya --version) installed!"

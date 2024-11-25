{
  aarch64-apple-darwin = {
    rustTarget = "aarch64-apple-darwin";
  };

  aarch64-unknown-linux-musl = {
    rustTarget = "aarch64-unknown-linux-musl";
  };

  armv6l-unknown-linux-musleabihf = {
    rustTarget = "arm-unknown-linux-musleabihf";
  };

  armv7l-unknown-linux-musleabihf = {
    rustTarget = "armv7-unknown-linux-musleabihf";
  };

  i686-unknown-linux-musl = {
    rustTarget = "i686-unknown-linux-musl";
  };

  i686-w64-mingw32 = {
    rustTarget = "i686-pc-windows-gnu";
    emulator = pkgs: "${pkgs.wine}/bin/wine";
  };

  x86_64-apple-darwin = {
    rustTarget = "x86_64-apple-darwin";
  };

  x86_64-unknown-linux-musl = {
    rustTarget = "x86_64-unknown-linux-musl";
  };

  x86_64-w64-mingw32 = {
    rustTarget = "x86_64-pc-windows-gnu";
  };
}  

{
  aarch64-apple-darwin = {
    rustTarget = "aarch64-apple-darwin";
    isStatic = true;
  };

  aarch64-unknown-linux-musl = {
    rustTarget = "aarch64-unknown-linux-musl";
    isStatic = true;
  };

  armv6l-unknown-linux-musleabihf = {
    rustTarget = "arm-unknown-linux-musleabihf";
    isStatic = true;
  };

  armv7l-unknown-linux-musleabihf = {
    rustTarget = "armv7-unknown-linux-musleabihf";
    isStatic = true;
  };

  i686-unknown-linux-musl = {
    rustTarget = "i686-unknown-linux-musl";
    isStatic = true;
  };

  i686-w64-mingw32 = {
    rustTarget = "i686-pc-windows-gnu";
    isStatic = false;
  };

  x86_64-apple-darwin = {
    rustTarget = "x86_64-apple-darwin";
    isStatic = false;
  };

  x86_64-unknown-linux-musl = {
    rustTarget = "x86_64-unknown-linux-musl";
    isStatic = true;
  };

  x86_64-w64-mingw32 = {
    rustTarget = "x86_64-pc-windows-gnu";
    isStatic = false;
  };
}  

# The first level represents the build platform system, and the second
# level represents the build platform triple config.

{
  x86_64-linux = {
    x86_64-unknown-linux-musl = { };

    aarch64-unknown-linux-musl = {
      runner = { qemu, ... }: "${qemu}/bin/qemu-aarch64 ./himalaya";
    };

    x86_64-pc-windows-gnu = {
      runner = { wine, ... }:
        let wine64 = wine.override { wineBuild = "wine64"; };
        in "${wine64}/bin/wine64 ./himalaya.exe";
    };
  };

  aarch64-linux = {
    x86_64-unknown-linux-musl = { };
  };

  x86_64-darwin = {
    x86_64-apple-darwin = { };
  };

  aarch64-darwin = {
    aarch64-apple-darwin = { };
  };
}  

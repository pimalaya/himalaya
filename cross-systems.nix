{
  aarch64-unknown-linux-musl = {
    runner = { qemu, ... }: "${qemu}/bin/qemu-aarch64 ./himalaya";
  };

  x86_64-pc-windows-gnu = {
    runner = { wine, ... }:
      let wine64 = wine.override { wineBuild = "wine64"; };
      in "${wine64}/bin/wine64 ./himalaya.exe";
  };
}  

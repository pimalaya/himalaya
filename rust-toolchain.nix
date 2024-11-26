fenix:

let
  file = ./rust-toolchain.toml;
  sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
in

rec {
  fromFile = { buildSystem }: fenix.packages.${buildSystem}.fromToolchainFile {
    inherit file sha256;
  };

  toRustTarget = target: {
    x86_64-w64-mingw32 = "x86_64-pc-windows-gnu";
    i686-w64-mingw32 = "i686-pc-windows-gnu";
    armv6l-unknown-linux-musleabihf = "arm-unknown-linux-musleabihf";
    armv7l-unknown-linux-musleabihf = "armv7-unknown-linux-musleabihf";
  }.${target} or target;

  fromTarget = { lib, targetSystem ? null }:
    let
      name = (lib.importTOML file).toolchain.channel;
      toolchain = fenix.fromToolchainName { inherit name sha256; };
      target = if isNull targetSystem then null else toRustTarget targetSystem;
      components = [ toolchain.rustc toolchain.cargo ]
        ++ lib.optional (!isNull target) (fenix.targets.${target}.fromToolchainName { inherit name sha256; }.rust-std);
    in

    fenix.combine components;
}

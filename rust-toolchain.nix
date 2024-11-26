{ lib, fenix }:

let
  file = ./rust-toolchain.toml;
  sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
in

{
  fromTarget = target:
    let
      name = (lib.importTOML file).toolchain.channel;
      specs = { inherit name sha256; };
      toolchain = fenix.fromToolchainName specs;
      crossToolchain = fenix.targets.${target}.fromToolchainName specs;
      components = [ toolchain.rustc toolchain.cargo ]
        ++ lib.optional (!isNull target) crossToolchain.rust-std;
    in

    fenix.combine components;
}

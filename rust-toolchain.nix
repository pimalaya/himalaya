fenix:

let
  file = ./rust-toolchain.toml;
  sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
in

{
  fromFile = { buildSystem }: fenix.packages.${buildSystem}.fromToolchainFile {
    inherit file sha256;
  };

  fromTarget = { pkgs, targetSystem }:
    let
      name = (pkgs.lib.importTOML file).toolchain.channel;
      toolchain = fenix.fromToolchainName { inherit name sha256; };
      targetToolchain = fenix.targets.${targetSystem}.fromToolchainName { inherit name sha256; };
    in
    fenix.combine [
      toolchain.rustc
      toolchain.cargo
      targetToolchain.rust-std
    ];
}

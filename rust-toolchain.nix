fenix:

let
  file = ./rust-toolchain.toml;
  sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
in
{
  fromFile = { buildSystem }: fenix.packages.${buildSystem}.fromToolchainFile {
    inherit file sha256;
  };

  fromTarget = { pkgs, buildSystem, targetSystem }:
    let
      name = (pkgs.lib.importTOML file).toolchain.channel;
      fenixPackage = fenix.packages.${buildSystem};
      toolchain = fenixPackage.fromToolchainName { inherit name sha256; };
      targetToolchain = fenixPackage.targets.${targetSystem}.fromToolchainName { inherit name sha256; };
    in
    fenixPackage.combine [
      toolchain.rustc
      toolchain.cargo
      targetToolchain.rust-std
    ];
}

fenix:

let
  file = ./rust-toolchain.toml;
  sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
in
{
  fromFile = { system }: fenix.packages.${system}.fromToolchainFile {
    inherit file sha256;
  };

  fromTarget = { pkgs, buildPlatform, targetPlatform }:
    let
      name = (pkgs.lib.importTOML file).toolchain.channel;
      fenixPackage = fenix.packages.${buildPlatform};
      toolchain = fenixPackage.fromToolchainName { inherit name sha256; };
      targetToolchain = fenixPackage.targets.${targetPlatform}.fromToolchainName { inherit name sha256; };
    in
    fenixPackage.combine [
      toolchain.rustc
      toolchain.cargo
      targetToolchain.rust-std
    ];
}

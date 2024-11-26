fenix:

let
  file = ./rust-toolchain.toml;
  sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
in

{
  # fromFile = { buildSystem }: fenix.packages.${buildSystem}.fromToolchainFile {
  #   inherit file sha256;
  # };

  fromTarget = { lib, target ? null }:
    let
      name = (lib.importTOML file).toolchain.channel;
      specs = { inherit name sha256; };
      toolchain = fenix.fromToolchainName specs;
      crossToolchain = fenix.targets.${target}.fromToolchainName specs;
      components = [ toolchain.rustc toolchain.cargo ]
        ++ lib.optional (!isNull target) crossToolchain;
    in

    fenix.combine components;
}

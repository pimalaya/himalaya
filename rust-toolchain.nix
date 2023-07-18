fenix:

let
  file = ./rust-toolchain.toml;
  sha256 = "ks0nMEGGXKrHnfv4Fku+vhQ7gx76ruv6Ij4fKZR3l78=";
in
{
  fromFile = { system }: fenix.packages.${system}.fromToolchainFile {
    inherit file sha256;
  };

  fromTarget = { pkgs, buildPlatform, targetPlatform ? null }:
    let
      inherit ((pkgs.lib.importTOML file).toolchain) channel;
      toolchain = fenix.packages.${buildPlatform};
    in
    if
      isNull targetPlatform
    then
      fenix.packages.${buildPlatform}.${channel}.toolchain
    else
      toolchain.combine [
        toolchain.${channel}.rustc
        toolchain.${channel}.cargo
        toolchain.targets.${targetPlatform}.${channel}.rust-std
      ];
}

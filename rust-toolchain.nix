{ lib, fenix }:

let
  file = ./rust-toolchain.toml;
  sha256 = "yMuSb5eQPO/bHv+Bcf/US8LVMbf/G/0MSfiPwBhiPpk=";
in

{
  fromFile = fenix.fromToolchainFile { inherit file sha256; };

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

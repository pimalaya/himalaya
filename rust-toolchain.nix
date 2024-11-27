{ lib, fenix }:

let
  file = ./rust-toolchain.toml;
  sha256 = "yMuSb5eQPO/bHv+Bcf/US8LVMbf/G/0MSfiPwBhiPpk=";
in

{
  fromFile =
    let spec = { inherit file sha256; };
    in fenix.fromToolchainFile spec;

  fromTarget = target:
    let
      name = (lib.importTOML file).toolchain.channel;
      spec = { inherit name sha256; };
      toolchain = fenix.fromToolchainName spec;
      crossToolchain = fenix.targets.${target}.fromToolchainName spec;
      components = [ toolchain.rustc toolchain.cargo ]
        ++ lib.optional (!isNull target) crossToolchain.rust-std;
    in

    fenix.combine components;
}

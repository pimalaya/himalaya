{
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  nixpkgs ? <nixpkgs>,
  ...
}@args:

pimalaya.mkDefault (
  {
    src = ./.;
    version = "1.1.0";
    mkPackage = (
      {
        lib,
        pkgs,
        rustPlatform,
        defaultFeatures,
        features,
      }:
      pkgs.callPackage "${nixpkgs}/pkgs/by-name/hi/himalaya/package.nix" {
        inherit lib rustPlatform;
        installShellCompletions = false;
        installManPages = false;
        buildNoDefaultFeatures = !defaultFeatures;
        buildFeatures = lib.splitString "," features;
      }
    );
  }
  // removeAttrs args [ "pimalaya" ]
)

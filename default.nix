{
  nixpkgs ? <nixpkgs>,
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  ...
}@args:

let
  himalaya = import ./default.nix (
    removeAttrs args [
      "crossPkgs"
      "isStatic"
      "target"
    ]
  );

in
pimalaya.mkDefault (
  {
    src = ./.;
    version = "2.1.0";
    mkPackage = (
      {
        lib,
        pkgs,
        rustPlatform,
        defaultFeatures,
        features,
        buildPackages,
      }:

      (pkgs.callPackage "${nixpkgs}/pkgs/by-name/hi/himalaya/package.nix" {
        inherit lib rustPlatform;
        # the nixpkgs derivation runs the binary it just built, which needs
        # a native one when cross compiling
        buildPackages = buildPackages // {
          inherit himalaya;
        };
        installShellCompletions = false;
        installManPages = false;
        buildNoDefaultFeatures = !defaultFeatures;
        buildFeatures = lib.splitString "," features;
      })
      # HACK: needed until the v2.1.0 derivation lands on nixpkgs's master
      .overrideAttrs
        {
          postInstall =
            let
              inherit (pkgs) stdenv;
              exe =
                if stdenv.buildPlatform.canExecute stdenv.hostPlatform then
                  "$out/bin/himalaya"
                else
                  lib.getExe himalaya;
            in
            ''
              mkdir -p $out/share/{completions,man,schemas}
              ${exe} completion -d "$out"/share/completions bash elvish fish powershell zsh
              ${exe} manual -d "$out"/share/man
              ${exe} json-schema -d "$out"/share/schemas
            '';
        }
    );
  }
  // removeAttrs args [ "pimalaya" ]
)

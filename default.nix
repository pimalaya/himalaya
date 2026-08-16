{
  nixpkgs ? <nixpkgs>,
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  ...
}@args:

pimalaya.mkDefault (
  {
    src = ./.;
    version = "2.0.0";
    mkPackage = (
      {
        lib,
        pkgs,
        buildPackages,
        rustPlatform,
        defaultFeatures,
        features,
      }:
      (pkgs.callPackage "${nixpkgs}/pkgs/by-name/hi/himalaya/package.nix" {
        inherit lib rustPlatform;
        buildNoDefaultFeatures = !defaultFeatures;
        buildFeatures = lib.splitString "," features;
      })
      # HACK: needed until new derivation available on nixpkgs's
      # master branch
      .overrideAttrs
        {
          postInstall =
            let
              inherit (pkgs) stdenv;
              emulator = stdenv.hostPlatform.emulator buildPackages;
              exe = stdenv.hostPlatform.extensions.executable;
            in
            lib.optionalString (lib.hasInfix "wine" emulator) ''
              export WINEPREFIX="''${WINEPREFIX:-$(mktemp -d)}"
              mkdir -p $WINEPREFIX
            ''
            + ''
              mkdir -p $out/share/{completions,man,schemas}
              ${emulator} "$out"/bin/himalaya${exe} completion -d "$out"/share/completions bash elvish fish powershell zsh
              ${emulator} "$out"/bin/himalaya${exe} manual "$out"/share/man
              ${emulator} "$out"/bin/himalaya${exe} json-schema "$out"/share/schemas
            ''
            + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
              installManPage "$out"/share/man/*
            ''
            + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
              installShellCompletion --cmd himalaya \
                --bash "$out"/share/completions/himalaya.bash \
                --fish "$out"/share/completions/himalaya.fish \
                --zsh "$out"/share/completions/_himalaya
            '';
        }
    );
  }
  // removeAttrs args [ "pimalaya" ]
)

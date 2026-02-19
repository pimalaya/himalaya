{
  nixpkgs ? <nixpkgs>,
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  ...
}@args:

pimalaya.mkDefault (
  {
    src = ./.;
    version = "1.2.0";
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
              mkdir -p $out/share/{applications,completions,man}
              cp assets/himalaya.desktop "$out"/share/applications/
              ${emulator} "$out"/bin/himalaya${exe} man "$out"/share/man
              ${emulator} "$out"/bin/himalaya${exe} completion bash > "$out"/share/completions/himalaya.bash
              ${emulator} "$out"/bin/himalaya${exe} completion elvish > "$out"/share/completions/himalaya.elvish
              ${emulator} "$out"/bin/himalaya${exe} completion fish > "$out"/share/completions/himalaya.fish
              ${emulator} "$out"/bin/himalaya${exe} completion powershell > "$out"/share/completions/himalaya.powershell
              ${emulator} "$out"/bin/himalaya${exe} completion zsh > "$out"/share/completions/himalaya.zsh
            ''
            + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
              installManPage "$out"/share/man/*
            ''
            + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
              installShellCompletion "$out"/share/completions/himalaya.{bash,fish,zsh}
            '';
        }
    );
  }
  // removeAttrs args [ "pimalaya" ]
)

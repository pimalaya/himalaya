{
  nixpkgs ? <nixpkgs>,
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
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
      (pkgs.callPackage "${nixpkgs}/pkgs/by-name/hi/himalaya/package.nix" {
        inherit lib rustPlatform;
        buildNoDefaultFeatures = !defaultFeatures;
        buildFeatures = lib.splitString "," features;
      })
      # HACK: needed until new derivation available on nixpkgs
      .overrideAttrs
        {
          postInstall =
            let
              inherit (pkgs) stdenv buildPackages;
              emulator = stdenv.hostPlatform.emulator buildPackages;
            in
            ''
              mkdir -p $out/share/{applications,completions,man}
              cp assets/himalaya.desktop "$out"/share/applications/
              ${emulator} "$out"/bin/himalaya man "$out"/share/man
              ${emulator} "$out"/bin/himalaya completion bash > "$out"/share/completions/himalaya.bash
              ${emulator} "$out"/bin/himalaya completion elvish > "$out"/share/completions/himalaya.elvish
              ${emulator} "$out"/bin/himalaya completion fish > "$out"/share/completions/himalaya.fish
              ${emulator} "$out"/bin/himalaya completion powershell > "$out"/share/completions/himalaya.powershell
              ${emulator} "$out"/bin/himalaya completion zsh > "$out"/share/completions/himalaya.zsh
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

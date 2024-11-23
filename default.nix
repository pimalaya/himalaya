{ target ? null }:

let
  pkgs = import <nixpkgs> (
    if isNull target then { }
    else { crossSystem.config = target; }
  );

  inherit (pkgs) lib hostPlatform buildPlatform;

  fenix = import (fetchTarball "https://github.com/soywod/fenix/archive/main.tar.gz") { };

  mkToolchain = import ./rust-toolchain.nix fenix;

  rustToolchain = mkToolchain.fromTarget {
    inherit lib;
    targetSystem = hostPlatform.config;
  };

  rustPlatform = pkgs.makeRustPlatform {
    rustc = rustToolchain;
    cargo = rustToolchain;
  };

  himalaya = import ./package.nix {
    inherit lib hostPlatform rustPlatform;
    fetchFromGitHub = pkgs.fetchFromGitHub;
    stdenv = pkgs.stdenv;
    darwin = pkgs.darwin;
    installShellFiles = false;
    installShellCompletions = false;
    installManPages = false;
    notmuch = pkgs.notmuch;
    gpgme = pkgs.gpgme;
  };

  himalayaBin =
    if hostPlatform == buildPlatform
    then "./himalaya"
    else "${hostPlatform.emulator pkgs.buildPackages} himalaya";

in

himalaya.overrideAttrs (drv: {
  version = "1.0.0";

  postInstall = lib.optionalString hostPlatform.isWindows ''
    export WINEPREFIX="$(${lib.getExe pkgs.buildPackages.mktemp} -d)"
  '' + ''
    mkdir -p $out/bin/share/{applications,completions,man,services}
    cp assets/himalaya.desktop $out/bin/share/applications/
    cp assets/himalaya-watch@.service $out/bin/share/services/

    cd $out/bin
    ${himalayaBin} man ./share/man
    ${himalayaBin} completion bash > ./share/completions/himalaya.bash
    ${himalayaBin} completion elvish > ./share/completions/himalaya.elvish
    ${himalayaBin} completion fish > ./share/completions/himalaya.fish
    ${himalayaBin} completion powershell > ./share/completions/himalaya.powershell
    ${himalayaBin} completion zsh > ./share/completions/himalaya.zsh

    ${lib.getExe pkgs.buildPackages.gnutar} -czf himalaya.tgz himalaya* share
    mv himalaya.tgz ../

    ${lib.getExe pkgs.buildPackages.zip} -r himalaya.zip himalaya* share
    mv himalaya.zip ../
  '';

  src = pkgs.nix-gitignore.gitignoreSource [ ] ./.;

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };
})

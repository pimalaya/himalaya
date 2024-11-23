{ target ? null }:

let
  pkgs = import <nixpkgs> (
    if isNull target then { }
    else { crossSystem.config = target; }
  );

  fenix = import (fetchTarball "https://github.com/soywod/fenix/archive/main.tar.gz") { };

  mkToolchain = import ./rust-toolchain.nix fenix;

  rustToolchain = mkToolchain.fromTarget {
    lib = pkgs.lib;
    targetSystem = pkgs.hostPlatform.config;
  };

  rustPlatform = pkgs.makeRustPlatform {
    rustc = rustToolchain;
    cargo = rustToolchain;
  };

  himalaya = import ./package.nix {
    inherit rustPlatform;
    darwin = pkgs.darwin;
    windows = pkgs.windows;
    lib = pkgs.lib;
    hostPlatform = pkgs.hostPlatform;
    fetchFromGitHub = pkgs.fetchFromGitHub;
    pkg-config = pkgs.pkg-config;
    installShellFiles = false;
    installShellCompletions = false;
    installManPages = false;
    notmuch = pkgs.notmuch;
    gpgme = pkgs.gpgme;
    stdenv = pkgs.stdenv;
    pkgsCross = pkgs.pkgsCross;
  };
in

himalaya.overrideAttrs (drv: {
  version = "1.0.0";
  postInstall = ''
    export WINEPREFIX="$(mktemp -d)"

    mkdir -p $out/bin/share/{applications,completions,man,services}
    cp assets/himalaya.desktop $out/bin/share/applications/
    cp assets/himalaya-watch@.service $out/bin/share/services/

    cd $out/bin
    ${pkgs.hostPlatform.emulator pkgs.buildPackages} himalaya man ./share/man
    ${pkgs.hostPlatform.emulator pkgs.buildPackages} himalaya completion bash > ./share/completions/himalaya.bash
    ${pkgs.hostPlatform.emulator pkgs.buildPackages} himalaya completion elvish > ./share/completions/himalaya.elvish
    ${pkgs.hostPlatform.emulator pkgs.buildPackages} himalaya completion fish > ./share/completions/himalaya.fish
    ${pkgs.hostPlatform.emulator pkgs.buildPackages} himalaya completion powershell > ./share/completions/himalaya.powershell
    ${pkgs.hostPlatform.emulator pkgs.buildPackages} himalaya completion zsh > ./share/completions/himalaya.zsh

    tar -czf himalaya.tgz himalaya* share
    mv himalaya.tgz ../

    ${pkgs.buildPackages.zip}/bin/zip -r himalaya.zip himalaya* share
    mv himalaya.zip ../
  '';
  src = pkgs.nix-gitignore.gitignoreSource [ ] ./.;
  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };
})

# https://nixos.org/manual/nixpkgs/stable/#sec-cross-usage
{ crossSystem ? null }:

let
  crossBuildEnabled = !isNull crossSystem;
  crossSystems = import ./cross-systems.nix;

  buildPkgs = import <nixpkgs> { };
  hostPkgs = if crossBuildEnabled then import <nixpkgs> { inherit crossSystem; } else buildPkgs;

  inherit (hostPkgs.stdenv) buildPlatform hostPlatform;

  runner = if crossBuildEnabled then crossSystems.${hostPlatform.config}.runner buildPkgs else "./himalaya";

  fenix = import (fetchTarball "https://github.com/soywod/fenix/archive/main.tar.gz") { };

  mkToolchain = import ./rust-toolchain.nix fenix;

  rustToolchain = mkToolchain.fromTarget {
    pkgs = hostPkgs;
    targetSystem = buildPlatform.config;
  };

  rustPlatform = hostPkgs.makeRustPlatform {
    rustc = rustToolchain;
    cargo = rustToolchain;
  };

  himalaya = import ./package.nix {
    inherit rustPlatform;
    lib = hostPkgs.lib;
    fetchFromGitHub = hostPkgs.fetchFromGitHub;
    pkg-config = hostPkgs.pkg-config;
    darwin = hostPkgs.darwin;
    installShellFiles = false;
    installShellCompletions = false;
    installManPages = false;
    notmuch = hostPkgs.notmuch;
    gpgme = hostPkgs.gpgme;
    stdenv = hostPkgs.stdenv;
  };
in

himalaya.overrideAttrs (drv: {
  version = "1.0.0";
  postInstall = ''
    mkdir -p $out/bin/share/{applications,completions,man,services}
    cp assets/himalaya.desktop $out/bin/share/applications/
    cp assets/himalaya-watch@.service $out/bin/share/services/

    cd $out/bin
    ${runner} man ./share/man
    ${runner} completion bash > ./share/completions/himalaya.bash
    ${runner} completion elvish > ./share/completions/himalaya.elvish
    ${runner} completion fish > ./share/completions/himalaya.fish
    ${runner} completion powershell > ./share/completions/himalaya.powershell
    ${runner} completion zsh > ./share/completions/himalaya.zsh

    tar -czf himalaya.tgz himalaya* share
    mv himalaya.tgz ../

    ${hostPkgs.zip}/bin/zip -r himalaya.zip himalaya* share
    mv himalaya.zip ../
  '';
  src = hostPkgs.nix-gitignore.gitignoreSource [ ] ./.;
  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };
})

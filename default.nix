{ target ? null }:

let
  pkgs = import <nixpkgs> { };
  hostPkgs =
    if isNull target || pkgs.stdenv.buildPlatform.config == target
    then pkgs else import <nixpkgs> { crossSystem.config = target; };

  inherit (hostPkgs) lib;
  inherit (hostPkgs.stdenv) buildPlatform hostPlatform;

  crossSystems = import ./cross-systems.nix;
  crossSystem =
    if lib.attrsets.hasAttrByPath [ buildPlatform.system hostPlatform.config ] crossSystems then
      crossSystems.${buildPlatform.system}.${hostPlatform.config}
    else
      throw "Platform not supported: " + hostPlatform.config;

  runner = crossSystem.runner or (_: "./himalaya") pkgs;

  fenix = import (fetchTarball "https://github.com/soywod/fenix/archive/main.tar.gz") { };

  mkToolchain = import ./rust-toolchain.nix fenix;

  rustToolchain = mkToolchain.fromTarget {
    pkgs = hostPkgs;
    targetSystem = hostPlatform.config;
  };

  rustPlatform = hostPkgs.makeRustPlatform {
    rustc = rustToolchain;
    cargo = rustToolchain;
  };

  himalaya = import ./package.nix {
    inherit lib rustPlatform;
    fetchFromGitHub = hostPkgs.fetchFromGitHub;
    pkg-config = hostPkgs.pkg-config;
    darwin = hostPkgs.darwin;
    windows = hostPkgs.pkgsCross.mingwW64.windows;
    installShellFiles = false;
    installShellCompletions = false;
    installManPages = false;
    notmuch = hostPkgs.notmuch;
    gpgme = hostPkgs.gpgme;
    stdenv =
      if hostPlatform.isWindows
      then hostPkgs.pkgsCross.mingwW64.stdenv
      else hostPkgs.stdenv;
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

{ pkgs ? import <nixpkgs> { }
, crossPkgs ? pkgs
, fenix ? import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { }
, defaultFeatures ? true
, features ? ""
}:

let
  inherit (pkgs) binutils gnutar lib mktemp stdenv zip;
  inherit (crossPkgs) buildPlatform hostPlatform;

  mkToolchain = import ./rust-toolchain.nix { inherit lib fenix; };
  rustTarget = if buildPlatform == hostPlatform then null else hostPlatform.rust.rustcTarget;
  rustToolchain = mkToolchain.fromTarget rustTarget;
  rustPlatform = crossPkgs.makeRustPlatform {
    rustc = rustToolchain;
    cargo = rustToolchain;
  };

  # HACK: https://github.com/NixOS/nixpkgs/issues/177129
  empty-libgcc_eh = stdenv.mkDerivation {
    pname = "empty-libgcc_eh";
    version = "0";
    dontUnpack = true;
    installPhase = ''
      mkdir -p "$out"/lib
      "${lib.getExe' binutils "ar"}" r "$out"/lib/libgcc_eh.a
    '';
  };

  himalayaExe =
    let ext = lib.optionalString hostPlatform.isWindows ".exe";
    in "${hostPlatform.emulator pkgs} ./himalaya${ext}";

  himalaya = import ./package.nix {
    inherit lib rustPlatform;
    fetchFromGitHub = crossPkgs.fetchFromGitHub;
    stdenv = crossPkgs.stdenv;
    apple-sdk = if hostPlatform.isx86_64 then crossPkgs.apple-sdk_13 else crossPkgs.apple-sdk_14;
    installShellFiles = false;
    installShellCompletions = false;
    installManPages = false;
    notmuch = crossPkgs.notmuch;
    gpgme = crossPkgs.gpgme;
    pkg-config = crossPkgs.pkg-config;
    buildNoDefaultFeatures = !defaultFeatures;
    buildFeatures = lib.strings.splitString "," features;
  };
in

himalaya.overrideAttrs (drv: {
  version = "1.0.0";

  propagatedBuildInputs = (drv.propagatedBuildInputs or [ ])
    ++ lib.optional hostPlatform.isWindows empty-libgcc_eh;

  postInstall = (drv.postInstall or "") + lib.optionalString hostPlatform.isWindows ''
    export WINEPREFIX="$(${lib.getExe' mktemp "mktemp"} -d)"
  '' + ''
    mkdir -p $out/bin/share/{applications,completions,man,services}
    cp assets/himalaya.desktop $out/bin/share/applications/
    cp assets/himalaya-watch@.service $out/bin/share/services/

    cd $out/bin
    ${himalayaExe} man ./share/man
    ${himalayaExe} completion bash > ./share/completions/himalaya.bash
    ${himalayaExe} completion elvish > ./share/completions/himalaya.elvish
    ${himalayaExe} completion fish > ./share/completions/himalaya.fish
    ${himalayaExe} completion powershell > ./share/completions/himalaya.powershell
    ${himalayaExe} completion zsh > ./share/completions/himalaya.zsh

    ${lib.getExe gnutar} -czf himalaya.tgz himalaya* share
    mv himalaya.tgz ../

    ${lib.getExe zip} -r himalaya.zip himalaya* share
    mv himalaya.zip ../
  '';

  src = crossPkgs.nix-gitignore.gitignoreSource [ ] ./.;

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };
})

{ pkgs ? import <nixpkgs> { }
, crossPkgs ? pkgs
, fenix ? import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { }
, defaultFeatures ? true
, features ? ""
}:

let
  inherit (pkgs) binutils gnutar lib mktemp stdenv wine zip;
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

  himalaya = import ./package.nix {
    inherit lib rustPlatform;
    fetchFromGitHub = crossPkgs.fetchFromGitHub;
    stdenv = crossPkgs.stdenv;
    apple-sdk = if hostPlatform.isx86_64 then crossPkgs.apple-sdk_13 else crossPkgs.apple-sdk_14;
    installShellFiles = crossPkgs.installShellFiles;
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

  postInstall = null;

  src = crossPkgs.nix-gitignore.gitignoreSource [ ] ./.;

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };
})

{ target ? null
, isStatic ? false
, defaultFeatures ? true
, features ? ""
}:

let
  buildPackages = import (fetchTarball "https://github.com/soywod/nixpkgs/archive/master.tar.gz") { };
  inherit (buildPackages) stdenv binutils mktemp gnutar zip;

  pkgs = import (fetchTarball "https://github.com/soywod/nixpkgs/archive/master.tar.gz") (
    if isNull target then { } else {
      crossSystem = {
        inherit isStatic;
        config = target;
      };
    }
  );

  inherit (pkgs) lib hostPlatform;
  fenix = import (fetchTarball "https://github.com/soywod/fenix/archive/main.tar.gz") { };
  mkToolchain = import ./rust-toolchain.nix fenix;
  rustTarget = if isNull target then null else hostPlatform.rust.rustcTarget;
  rustToolchain = mkToolchain.fromTarget { inherit lib; target = rustTarget; };
  rustPlatform = pkgs.makeRustPlatform {
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
    in "${hostPlatform.emulator buildPackages} ./himalaya${ext}";

  himalaya = import ./package.nix {
    inherit lib rustPlatform;
    fetchFromGitHub = pkgs.fetchFromGitHub;
    stdenv = pkgs.stdenv;
    apple-sdk = if pkgs.stdenv.hostPlatform.isx86_64 then pkgs.apple-sdk_13 else pkgs.apple-sdk_14;
    installShellFiles = false;
    windows = pkgs.windows;
    installShellCompletions = false;
    installManPages = false;
    notmuch = pkgs.notmuch;
    gpgme = pkgs.gpgme;
    pkg-config = pkgs.pkg-config;
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

  src = pkgs.nix-gitignore.gitignoreSource [ ] ./.;

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };
})

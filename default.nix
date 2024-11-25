{ target
, defaultFeatures ? true
, features ? ""
}:

let
  systems = import ./systems.nix;
  system = systems.${target};

  buildPackages = import (fetchTarball "https://github.com/nixos/nixpkgs/archive/nixos-unstable.tar.gz") { };
  inherit (buildPackages) stdenv binutils mktemp gnutar zip;

  pkgs = import (fetchTarball "https://github.com/nixos/nixpkgs/archive/nixos-unstable.tar.gz") {
    crossSystem = {
      isStatic = true;
      config = target;
    };
  };

  inherit (pkgs) lib hostPlatform;

  fenix = import (fetchTarball "https://github.com/soywod/fenix/archive/main.tar.gz") { };

  mkToolchain = import ./rust-toolchain.nix fenix;

  rustToolchain = mkToolchain.fromTarget {
    inherit lib;
    targetSystem = system.rustTarget;
  };

  rustPlatform = pkgs.makeRustPlatform {
    rustc = rustToolchain;
    cargo = rustToolchain;
  };

  himalayaExe =
    let ext = lib.optionalString hostPlatform.isWindows ".exe";
    in "${(system.emulator or hostPlatform.emulator) buildPackages} ./himalaya${ext}";

  himalaya = import ./package.nix {
    inherit lib rustPlatform;
    fetchFromGitHub = pkgs.fetchFromGitHub;
    stdenv = pkgs.stdenv;
    darwin = pkgs.darwin;
    installShellFiles = false;
    installShellCompletions = false;
    installManPages = false;
    notmuch = pkgs.notmuch;
    gpgme = pkgs.gpgme;
    pkg-config = pkgs.pkg-config;
    buildNoDefaultFeatures = !defaultFeatures;
    buildFeatures = lib.strings.splitString "," features;
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

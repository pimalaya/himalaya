{ pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz")
, ...
} @args:

pimalaya.mkDefault ({
  src = ./.;
  version = "1.0.0";
  mkPackage = ({ lib, pkgs, rustPlatform, defaultFeatures, features }: import ./package.nix {
    inherit lib rustPlatform;
    fetchFromGitHub = pkgs.fetchFromGitHub;
    stdenv = pkgs.stdenv;
    apple-sdk = if pkgs.hostPlatform.isx86_64 then pkgs.apple-sdk_13 else pkgs.apple-sdk_14;
    installShellFiles = pkgs.installShellFiles;
    installShellCompletions = false;
    installManPages = false;
    notmuch = pkgs.notmuch;
    gpgme = pkgs.gpgme;
    pkg-config = pkgs.pkg-config;
    buildNoDefaultFeatures = !defaultFeatures;
    buildFeatures = lib.splitString "," features;
  });
} // removeAttrs args [ "pimalaya" ])

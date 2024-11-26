{ pkgs ? import <nixpkgs> { } }:

let
  inherit (pkgs) lib;

  fenix = import (fetchTarball "https://github.com/soywod/fenix/archive/main.tar.gz") { };
  mkToolchain = import ./rust-toolchain.nix { inherit lib fenix; };
  rust = mkToolchain.fromFile;
in

pkgs.mkShell {
  buildInputs = with pkgs; [
    nixd
    nixpkgs-fmt
    rust
  ];
}

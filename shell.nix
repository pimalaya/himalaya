{ pkgs ? import <nixpkgs> { }
, fenix ? import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { }
, withNotmuch ? false
, withGpg ? false
, withOpenSsl ? false
}:

let
  inherit (pkgs) lib;
  mkRustToolchain = import ./rust-toolchain.nix { inherit lib fenix; };
  rust = mkRustToolchain.fromFile;
in

pkgs.mkShell {
  buildInputs = [ ]
    # Nix language
    ++ [ pkgs.nixd pkgs.nixpkgs-fmt ]

    # Rust
    ++ [ rust ]

    # Cargo features
    ++ lib.optional withNotmuch pkgs.notmuch
    ++ lib.optional withGpg pkgs.gpgme
    ++ lib.optional withOpenSsl pkgs.openssl;
}

{
  description = "Minimalist CLI email client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = { 
      url = "github:hercules-ci/gitignore"; 
      flake=false; 
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, gitignore, rust-overlay, ... }: 
    flake-utils.lib.eachDefaultSystem (system: 
      let 
        pkgs = import nixpkgs { 
          inherit system;
          overlays = [ rust-overlay.overlay ];
        };
        inherit (import gitignore { inherit (pkgs) lib; }) gitignoreSource;
        himalaya = 
          pkgs.rustPlatform.buildRustPackage rec {
            pname = "himalaya";
            version = "0.2.2";
            src = gitignoreSource ./.;
            nativeBuildInputs = with pkgs; [
              pkg-config 
              openssl.dev
              (pkgs.rust-bin.stable.latest.default.override {
                extensions = [
                  "rust-src"
                  "cargo"
                  "rustc"
                  "rls"
                  "rust-analysis"
                  "rustfmt"
                  "clippy"
                ];
              })
            ];
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            # When Cargo dependencies change, the sha here will have to be updated.
            # `nix-build` will give you the new sha.
            cargoSha256 = "sha256-G0f96pZe/KkuxQiFK45namDrSnDtYYcI/Ml00rt7G5M=";
            meta = with pkgs.stdenv.lib; {
              description = "Minimalist CLI email client";
              homepage = "https://github.com/soywod/himalaya";
            };
          };
      in {
        defaultPackage = himalaya;
      });
}

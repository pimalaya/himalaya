{
  description = "Minimalist CLI email client";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
    gitignore = { 
      url = "github:hercules-ci/gitignore"; 
      flake=false; 
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, naersk, gitignore, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let 
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ 
              rust-overlay.overlay
              (self: super: {
                # Because rust-overlay bundles multiple rust packages into one
                # derivation, specify that mega-bundle here, so that naersk
                # will use them automatically.
                rustc = self.rust-bin.stable.latest.default;
                cargo = self.rust-bin.stable.latest.default;
              })
            ];
          };
          inherit (import gitignore { inherit (pkgs) lib; }) gitignoreSource;
          naersk-lib = naersk.lib."${system}";
          nativeBuildInputs = with pkgs; [
            # List your C dependencies here
            pkg-config
            openssl.dev
          ];
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        in rec {
          # `nix build`
          packages.himalaya = naersk-lib.buildPackage {
            pname = "himalaya";
            root = gitignoreSource ./.;
            inherit nativeBuildInputs PKG_CONFIG_PATH;
          };
          defaultPackage = packages.himalaya;

          # `nix run`
          apps.himalaya = utils.lib.mkApp {
            drv = packages.himalaya;
          };
          defaultApp = apps.himalaya;

          # `nix develop`
          devShell = pkgs.mkShell {
            inherit PKG_CONFIG_PATH;
            nativeBuildInputs = 
              nativeBuildInputs ++ (with pkgs; [ rustc cargo ]) ;
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          };
        }
      );
}

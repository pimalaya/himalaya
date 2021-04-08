{
  description = "Minimalist CLI email client";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crate2nix = {
      url = "github:balsoft/crate2nix/tools-nix-version-comparison";
      flake = false;
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, crate2nix, ... }:
    utils.lib.eachDefaultSystem
      (system:
       let 
          name = "himalaya";
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
          inherit (import "${crate2nix}/tools.nix" { inherit pkgs; })
            generatedCargoNix;
          project = pkgs.callPackage (generatedCargoNix {
            inherit name;
            src = ./.;
          }) {
            # Individual crate overrides go here
            # Example: https://github.com/balsoft/simple-osd-daemons/blob/6f85144934c0c1382c7a4d3a2bbb80106776e270/flake.nix#L28-L50
          };
          nativeBuildInputs = with pkgs; [ rustc cargo pkgconfig openssl.dev ];
          buildEnvVars = {
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };
          rootCrateBuild = pkgs.lib.overrideDerivation project.rootCrate.build (oldAttrs: {
            inherit nativeBuildInputs;
          } // buildEnvVars);
        in rec {
          packages.${name} = rootCrateBuild;

          # `nix build`
          defaultPackage = packages.${name};

          # `nix run`
          apps.${name} = utils.lib.mkApp {
            inherit name;
            drv = packages.${name};
          };
          defaultApp = apps.${name};

          # `nix develop`
          devShell = pkgs.mkShell {
            inherit nativeBuildInputs;
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          } // buildEnvVars;
        }
      );
}

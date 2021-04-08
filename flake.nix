{
  description = "Minimalist CLI email client";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    crate2nix = {
      url = "github:balsoft/crate2nix/tools-nix-version-comparison";
      flake = false;
    };
    gitignore = { 
      url = "github:hercules-ci/gitignore"; 
      flake=false; 
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, gitignore, crate2nix, ... }:
    utils.lib.eachDefaultSystem
      (system:
       let 
          name = "himalaya";
          pkgs = import nixpkgs { inherit system; };
          inherit (import "${crate2nix}/tools.nix" { inherit pkgs; })
            generatedCargoNix;
          inherit (import gitignore { inherit (pkgs) lib; }) gitignoreSource;
          project = pkgs.callPackage (generatedCargoNix {
            inherit name;
            src = gitignoreSource ./.;
          }) {
            # Individual crate overrides go here
            # Example: https://github.com/balsoft/simple-osd-daemons/blob/6f85144934c0c1382c7a4d3a2bbb80106776e270/flake.nix#L28-L50
          };
        in rec {
          packages.${name} = project.rootCrate.build;

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
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            nativeBuildInputs = 
              with pkgs; [ rustc cargo pkgconfig openssl.dev ] ;
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          };
        }
      );
}

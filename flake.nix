{
  description = "📫 CLI email client";

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

          version = "0.4.0";

          # Imports
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              rust-overlay.overlay
              (self: super: {
                # Because rust-overlay bundles multiple rust packages into one
                # derivation, specify that mega-bundle here, so that crate2nix
                # will use them automatically.
                rustc = self.rust-bin.stable.latest.default;
                cargo = self.rust-bin.stable.latest.default;
              })
            ];
          };
          inherit (import "${crate2nix}/tools.nix" { inherit pkgs; })
            generatedCargoNix;

          # Create the cargo2nix project
          project = pkgs.callPackage (generatedCargoNix {
            inherit name;
            src = ./.;
          }) {
            # Individual crate overrides go here
            # Example: https://github.com/balsoft/simple-osd-daemons/blob/6f85144934c0c1382c7a4d3a2bbb80106776e270/flake.nix#L28-L50
            defaultCrateOverrides = pkgs.defaultCrateOverrides // {
              # The himalaya crate itself is overriden here. Typically we
              # configure non-Rust dependencies (see below) here.
              ${name} = oldAttrs: {
                inherit buildInputs nativeBuildInputs;
                postInstall = ''
                  mkdir -p $out/share/applications/
                  cp assets/himalaya.desktop $out/share/applications/
                '';
              };
            };
          };

          # Configuration for the non-Rust dependencies
          buildInputs = with pkgs; [ openssl.dev ];
          nativeBuildInputs = with pkgs; [ rustc cargo pkgconfig ];
        in
        rec {
          packages = {
            ${name} = project.rootCrate.build;

            "${name}-vim" = pkgs.vimUtils.buildVimPluginFrom2Nix {
              inherit (packages.${name}) version;
              name = "${name}-vim";
              src = self;
              buildInputs = [ packages.${name} ];
              dontConfigure = false;
              configurePhase = "cd vim/";
              postInstall = ''
                mkdir -p $out/bin
                ln -s ${packages.${name}}/bin/himalaya $out/bin/himalaya
              '';
            };
          };

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
            inputsFrom = builtins.attrValues self.packages.${system};
            buildInputs = with pkgs; [ cargo cargo-watch trunk ];
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          };
        }
      );
}

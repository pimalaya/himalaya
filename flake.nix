{
  description = "Command-line interface for email management";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, naersk, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          name = "himalaya";
          pkgs = import nixpkgs { inherit system; };
          naersk-lib = naersk.lib.${system};
        in
        rec {
          # `nix build`
          defaultPackage = packages.${name};
          packages = {
            ${name} = naersk-lib.buildPackage {
              pname = name;
              root = ./.;
              nativeBuildInputs = with pkgs; [ openssl.dev pkgconfig ];
              overrideMain = _: {
                postInstall = ''
                  mkdir -p $out/share/applications/
                  cp assets/himalaya.desktop $out/share/applications/
                '';
              };
            };
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

          # `nix run`
          defaultApp = apps.${name};
          apps.${name} = utils.lib.mkApp {
            inherit name;
            drv = packages.${name};
          };

          # `nix develop`
          devShell = pkgs.mkShell {
            inputsFrom = builtins.attrValues self.packages.${system};
            buildInputs = with pkgs; [ cargo cargo-watch trunk ];
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          };
        }
      );
}

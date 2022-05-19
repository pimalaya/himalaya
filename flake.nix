{
  description = "Command-line interface for email management";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    naersk.url = "github:nix-community/naersk";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, naersk, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          name = "himalaya";
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              rust-overlay.overlay
              (self: super: {
                # Because rust-overlay bundles multiple rust packages
                # into one derivation, specify that mega-bundle here,
                # so that crate2nix will use them automatically.
                rustc = self.rust-bin.stable.latest.default;
                cargo = self.rust-bin.stable.latest.default;
              })
            ];
          };
        in
        rec {
          # nix build
          defaultPackage = packages.${name};
          packages = {
            ${name} = naersk.lib.${system}.buildPackage {
              pname = name;
              root = ./.;
              nativeBuildInputs = with pkgs; [ openssl.dev pkgconfig ];
              cargoBuildOptions = _: [
                "$cargo_release"
                ''-j "$NIX_BUILD_CORES"''
                "--message-format=$cargo_message_format"
                "--package=himalaya"
              ];
              overrideMain = _: {
                postInstall = ''
                  mkdir -p $out/share/applications/
                  cp assets/himalaya.desktop $out/share/applications/
                '';
              };
            };
            "${name}-gui" = naersk.lib.${system}.buildPackage {
              pname = "${name}-gui";
              root = ./.;
              nativeBuildInputs = with pkgs; [
                llvmPackages_rocm.llvm
                cmake
                git
                openssl.dev
                pkgconfig
                llvm
                clang
                gtk3
              ];
              cargoBuildOptions = _: [
                "$cargo_release"
                ''-j "$NIX_BUILD_CORES"''
                "--message-format=$cargo_message_format"
                "--package=himalaya-gui"
              ];
              overrideMain = _: {
                LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
                postInstall = ''
                  mkdir -p $out/share/applications/
                  cp assets/himalaya-gui.desktop $out/share/applications/
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

          # nix run
          defaultApp = apps.${name};
          apps.${name} = utils.lib.mkApp {
            inherit name;
            drv = packages.${name};
          };

          # nix develop
          devShell = pkgs.mkShell {
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            inputsFrom = builtins.attrValues self.packages.${system};
            buildInputs = with pkgs; [
              cargo
              cargo-watch
              trunk
              ripgrep
              rust-analyzer
              rustfmt
              rnix-lsp
              nixpkgs-fmt
              notmuch

              # for gui
              llvmPackages_rocm.llvm
              cmake
              git
              llvm
              clang
              gtk3
              pkgconfig
            ];
          };
        }
      );
}

{
  description = "CLI to manage emails";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:soywod/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, gitignore, fenix, naersk, ... }:
    let
      inherit (nixpkgs) lib;
      inherit (gitignore.lib) gitignoreSource;

      staticRustFlags = [ "-Ctarget-feature=+crt-static" ];

      # Map of map matching supported Nix build systems with Rust
      # cross target systems.
      crossSystems = {
        x86_64-linux = {
          x86_64-linux = {
            rustTarget = "x86_64-unknown-linux-musl";
          };

          aarch64-linux = rec {
            rustTarget = "aarch64-unknown-linux-musl";
            runner = { pkgs, himalaya }: "${pkgs.qemu}/bin/qemu-aarch64 ${himalaya}";
            mkPackage = { system, pkgs }: package:
              let
                inherit (mkPkgsCross system rustTarget) stdenv;
                cc = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
              in
              package // {
                TARGET_CC = cc;
                CARGO_BUILD_RUSTFLAGS = package.CARGO_BUILD_RUSTFLAGS ++ [ "-Clinker=${cc}" ];
              };
          };

          x86_64-windows = {
            rustTarget = "x86_64-pc-windows-gnu";
            runner = { pkgs, himalaya }:
              let wine = pkgs.wine.override { wineBuild = "wine64"; };
              in "${wine}/bin/wine64 ${himalaya}.exe";
            mkPackage = { system, pkgs }: package:
              let
                inherit (pkgs.pkgsCross.mingwW64) stdenv windows;
                cc = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
              in
              package // {
                depsBuildBuild = [ stdenv.cc windows.pthreads ];
                TARGET_CC = cc;
                CARGO_BUILD_RUSTFLAGS = package.CARGO_BUILD_RUSTFLAGS ++ [ "-Clinker=${cc}" ];
              };
          };
        };

        aarch64-linux = {
          aarch64-linux = {
            rustTarget = "aarch64-unknown-linux-musl";
          };
        };

        x86_64-darwin = {
          x86_64-darwin = {
            rustTarget = "x86_64-apple-darwin";
            mkPackage = { pkgs, ... }: package:
              let inherit (pkgs.darwin.apple_sdk.frameworks) AppKit Cocoa;
              in package // {
                buildInputs = [ Cocoa ];
                NIX_LDFLAGS = "-F${AppKit}/Library/Frameworks -framework AppKit";
              };
          };

          # FIXME: infinite recursion in stdenv?!
          aarch64-darwin = {
            rustTarget = "aarch64-apple-darwin";
            runner = { pkgs, himalaya }: "${pkgs.qemu}/bin/qemu-aarch64 ${himalaya}";
            mkPackage = { system, pkgs }: package:
              let
                inherit ((mkPkgsCross system "aarch64-darwin").pkgsStatic) stdenv darwin;
                inherit (darwin.apple_sdk.frameworks) AppKit Cocoa;
                cc = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
              in
              package // {
                buildInputs = [ Cocoa ];
                NIX_LDFLAGS = "-F${AppKit}/Library/Frameworks -framework AppKit";
                TARGET_CC = cc;
                CARGO_BUILD_RUSTFLAGS = package.CARGO_BUILD_RUSTFLAGS ++ [ "-Clinker=${cc}" ];
              };
          };
        };

        aarch64-darwin = {
          aarch64-darwin = {
            rustTarget = "aarch64-apple-darwin";
            mkPackage = { pkgs, ... }: package:
              let inherit (pkgs.darwin.apple_sdk.frameworks) AppKit Cocoa;
              in package // {
                buildInputs = [ Cocoa ];
                NIX_LDFLAGS = "-F${AppKit}/Library/Frameworks -framework AppKit";
              };
          };
        };
      };

      mkToolchain = import ./rust-toolchain.nix fenix;

      mkPkgsCross = buildSystem: crossSystem: import nixpkgs {
        system = buildSystem;
        crossSystem.config = crossSystem;
      };

      mkDevShells = buildSystem:
        let
          pkgs = import nixpkgs { system = buildSystem; };
          rust-toolchain = mkToolchain.fromFile { inherit buildSystem; };
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = with pkgs; [
              # Nix
              nixd
              nixpkgs-fmt

              # Rust
              rust-toolchain
              cargo-watch

              # Email env
              gnupg
              gpgme
              msmtp
              notmuch
            ];
          };
        };

      mkPackages = buildSystem:
        let
          pkgs = import nixpkgs { system = buildSystem; };

          mkPackage = targetSystem: targetConfig:
            let
              mkPackage' = targetConfig.mkPackage or (_: p: p);
              himalaya = "./himalaya";
              runner = targetConfig.runner or (_: himalaya) { inherit pkgs himalaya; };
            in
            mkPackage' { inherit pkgs; system = buildSystem; } {
              name = "himalaya";
              src = gitignoreSource ./.;
              strictDeps = true;
              doCheck = false;
              auditable = false;
              nativeBuildInputs = with pkgs; [ pkg-config ];
              CARGO_BUILD_TARGET = targetConfig.rustTarget;
              CARGO_BUILD_RUSTFLAGS = staticRustFlags;
              postInstall = ''
                export WINEPREFIX="$(mktemp -d)"

                mkdir -p $out/bin/share/{applications,completions,man,services}
                cp assets/himalaya.desktop $out/bin/share/applications/
                cp assets/himalaya-watch@.service $out/bin/share/services/

                cd $out/bin
                ${runner} man ./share/man
                ${runner} completion bash > ./share/completions/himalaya.bash
                ${runner} completion elvish > ./share/completions/himalaya.elvish
                ${runner} completion fish > ./share/completions/himalaya.fish
                ${runner} completion powershell > ./share/completions/himalaya.powershell
                ${runner} completion zsh > ./share/completions/himalaya.zsh
                tar -czf himalaya.tgz himalaya* share
                ${pkgs.zip}/bin/zip -r himalaya.zip himalaya* share

                mv share ../
                mv himalaya.tgz himalaya.zip ../
              '';
            };

          buildPackage = targetSystem: targetConfig:
            let
              toolchain = mkToolchain.fromTarget {
                inherit pkgs buildSystem;
                targetSystem = targetConfig.rustTarget;
              };
              rust = naersk.lib.${buildSystem}.override {
                cargo = toolchain;
                rustc = toolchain;
              };
              package = mkPackage targetSystem targetConfig;
            in
            rust.buildPackage package;

          defaultPackage = buildPackage buildSystem crossSystems.${buildSystem}.${buildSystem};
          packages = builtins.mapAttrs buildPackage crossSystems.${buildSystem};

        in
        { default = defaultPackage; } // packages;

      mkApps = buildSystem:
        let
          pkgs = import nixpkgs { system = buildSystem; };
          mkAppWrapper = { targetSystem }:
            let
              targetConfig = crossSystems.${buildSystem}.${targetSystem};
              drv = self.packages.${buildSystem}.${targetSystem};
              exePath = drv.passthru.exePath or "/bin/himalaya";
              himalaya = "${drv}${exePath}";
              himalayaWrapper = targetConfig.runner or (_: himalaya) { inherit pkgs himalaya; };
              wrapper = pkgs.writeShellScriptBin "himalaya" "${himalayaWrapper} $@";
            in
            {
              type = "app";
              program = "${wrapper}/bin/himalaya";
            };
          mkApp = targetSystem: _: mkAppWrapper { inherit targetSystem; };

          defaultApp = mkApp buildSystem null;
          apps = builtins.mapAttrs mkApp crossSystems.${buildSystem};
        in
        { default = defaultApp; } // apps;

      eachSystem = lib.genAttrs (builtins.attrNames crossSystems);

    in
    {
      apps = eachSystem mkApps;
      packages = eachSystem mkPackages;
      devShells = eachSystem mkDevShells;
    };
}

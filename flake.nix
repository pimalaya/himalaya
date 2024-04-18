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
      inherit (gitignore.lib) gitignoreSource;

      staticRustFlags = [ "-Ctarget-feature=+crt-static" ];

      # Map of map matching supported Nix build systems with Rust
      # cross target systems.
      crossBuildTargets = {
        x86_64-linux = {
          x86_64-linux = {
            rustTarget = "x86_64-unknown-linux-musl";
          };

          aarch64-linux = rec {
            rustTarget = "aarch64-unknown-linux-musl";
            runner = pkgs: "${pkgs.qemu}/bin/qemu-aarch64 ./himalaya";
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
            runner = pkgs:
              let wine = pkgs.wine.override { wineBuild = "wine64"; };
              in "${wine}/bin/wine64 ./himalaya.exe";
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

        # FIXME: attribute 'sharedLibrary' missing?
        # x86_64-windows = {
        #   x86_64-windows = {
        #     rustTarget = "x86_64-pc-windows-gnu";
        #     runner = _: "./himalaya.exe";
        #     mkPackage = { system, pkgs }: package: package;
        #   };
        # };

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
          # aarch64-darwin = {
          #   rustTarget = "aarch64-apple-darwin";
          #   override = { system, pkgs }:
          #     let
          #       # inherit (mkPkgsCross system "aarch64-darwin") stdenv;
          #       inherit ((mkPkgsCross system "aarch64-darwin").pkgsStatic) stdenv darwin;
          #       inherit (darwin.apple_sdk.frameworks) AppKit Cocoa;
          #       cc = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
          #     in
          #     {
          #       buildInputs = [ Cocoa ];
          #       NIX_LDFLAGS = "-F${AppKit}/Library/Frameworks -framework AppKit -F${Cocoa}/Library/Frameworks -framework Cocoa";
          #       NIX_CFLAGS_COMPILE = "-F${AppKit}/Library/Frameworks -framework AppKit -F${Cocoa}/Library/Frameworks -framework Cocoa";
          #       TARGET_CC = cc;
          #       CARGO_BUILD_RUSTFLAGS = staticRustFlags ++ [ "-Clinker=${cc}" "-lframework=${Cocoa}/Library/Frameworks" ];
          #       postInstall = mkPostInstall {
          #         inherit pkgs;
          #         bin = "${pkgs.qemu}/bin/qemu-aarch64 ./himalaya";
          #       };
          #     };
          # };
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

      mkPackageArchives = { pkgs, runner ? "./himalaya" }: ''
        export WINEPREFIX="$(mktemp -d)"
        cd $out/bin
        mkdir -p {man,completions}
        ${runner} man ./man
        ${runner} completion bash > ./completions/himalaya.bash
        ${runner} completion elvish > ./completions/himalaya.elvish
        ${runner} completion fish > ./completions/himalaya.fish
        ${runner} completion powershell > ./completions/himalaya.powershell
        ${runner} completion zsh > ./completions/himalaya.zsh
        tar -czf himalaya.tgz himalaya* man completions
        ${pkgs.zip}/bin/zip -r himalaya.zip himalaya* man completions
      '';

      mkDevShells = buildPlatform:
        let
          pkgs = import nixpkgs { system = buildPlatform; };
          rust-toolchain = mkToolchain.fromFile { system = buildPlatform; };
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

      mkPackages = buildPlatform:
        let
          pkgs = import nixpkgs { system = buildPlatform; };

          mkPackage = targetPlatform: crossBuild:
            let mkPackage' = crossBuild.mkPackage or (_: p: p);
            in mkPackage' { inherit pkgs; system = buildPlatform; } {
              name = "himalaya";
              src = gitignoreSource ./.;
              # overrideMain = _: {
              #   postInstall = ''
              #     mkdir -p $out/share/applications/
              #     cp assets/himalaya.desktop $out/share/applications/
              #   '';
              # };
              doCheck = false;
              auditable = false;
              strictDeps = true;
              CARGO_BUILD_TARGET = targetPlatform;
              CARGO_BUILD_RUSTFLAGS = staticRustFlags;
            };

          buildPackage = doPostInstall: targetPlatform: crossBuild:
            let
              toolchain = mkToolchain.fromTarget {
                inherit pkgs buildPlatform;
                targetPlatform = crossBuild.rustTarget;
              };
              rust = naersk.lib.${buildPlatform}.override {
                cargo = toolchain;
                rustc = toolchain;
              };
              package = mkPackage targetPlatform crossBuild;
              postInstall = pkgs.lib.optionalAttrs doPostInstall {
                postInstall = mkPackageArchives {
                  inherit pkgs;
                  runner = (crossBuild.runner or (_: null)) pkgs;
                };
              };
            in
            rust.buildPackage package // postInstall;

          defaultPackage = buildPackage false buildPlatform crossBuildTargets.${buildPlatform}.${buildPlatform};
          packages = builtins.mapAttrs (buildPackage false) crossBuildTargets.${buildPlatform};
          archives = pkgs.lib.foldlAttrs (p: k: v: p // { "${k}-archives" = buildPackage true k v; }) { } crossBuildTargets.${buildPlatform};

        in
        { default = defaultPackage; } // packages // archives;

      mkApp = drv:
        let exePath = drv.passthru.exePath or "/bin/himalaya";
        in
        {
          type = "app";
          program = "${drv}${exePath}";
        };

      mkApps = buildPlatform:
        let
          pkgs = import nixpkgs { system = buildPlatform; };
          mkApp' = target: package: mkApp self.packages.${buildPlatform}.${target};
        in
        builtins.mapAttrs mkApp' crossBuildTargets.${buildPlatform};

      supportedSystems = builtins.attrNames crossBuildTargets;
      mapSupportedSystem = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      apps = mapSupportedSystem mkApps;
      packages = mapSupportedSystem mkPackages;
      devShells = mapSupportedSystem mkDevShells;
    };
}

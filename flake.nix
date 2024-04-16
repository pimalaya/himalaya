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
            override = { ... }: { };
          };

          arm64-linux = rec {
            rustTarget = "aarch64-unknown-linux-musl";
            override = { system, pkgs }:
              let
                inherit (mkPkgsCross system rustTarget) stdenv;
                cc = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc"; in
              rec {
                TARGET_CC = cc;
                CARGO_BUILD_RUSTFLAGS = staticRustFlags ++ [ "-Clinker=${cc}" ];
                postInstall = mkPostInstall {
                  inherit pkgs;
                  bin = "${pkgs.qemu}/bin/qemu-aarch64 ./himalaya";
                };
              };
          };

          x86_64-windows = {
            rustTarget = "x86_64-pc-windows-gnu";
            override = { system, pkgs }:
              let
                inherit (pkgs) pkgsCross zip;
                inherit (pkgsCross.mingwW64) stdenv windows;
                cc = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
                wine = pkgs.wine.override { wineBuild = "wine64"; };
                postInstall = mkPostInstall {
                  inherit pkgs;
                  bin = "${wine}/bin/wine64 ./himalaya.exe";
                };
              in
              {
                depsBuildBuild = [ stdenv.cc windows.pthreads ];
                TARGET_CC = cc;
                CARGO_BUILD_RUSTFLAGS = staticRustFlags ++ [ "-Clinker=${cc}" ];
                postInstall = ''
                  export WINEPREFIX="$(mktemp -d)"
                  ${postInstall}
                '';
              };
          };
        };

        x86_64-darwin = {
          x86_64-macos = {
            rustTarget = "x86_64-apple-darwin";
            override = { pkgs, ... }:
              let inherit (pkgs.darwin.apple_sdk.frameworks) AppKit Cocoa; in
              {
                buildInputs = [ Cocoa ];
                NIX_LDFLAGS = "-F${AppKit}/Library/Frameworks -framework AppKit";
              };
          };

          # FIXME: infinite recursion in stdenv?!
          arm64-macos = {
            rustTarget = "aarch64-apple-darwin";
            override = { system, pkgs }:
              let
                # inherit (mkPkgsCross system "aarch64-darwin") stdenv;
                inherit ((mkPkgsCross system "aarch64-darwin").pkgsStatic) stdenv darwin;
                inherit (darwin.apple_sdk.frameworks) AppKit Cocoa;
                cc = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
              in
              rec {
                buildInputs = [ Cocoa ];
                NIX_LDFLAGS = "-F${AppKit}/Library/Frameworks -framework AppKit -F${Cocoa}/Library/Frameworks -framework Cocoa";
                NIX_CFLAGS_COMPILE = "-F${AppKit}/Library/Frameworks -framework AppKit -F${Cocoa}/Library/Frameworks -framework Cocoa";
                TARGET_CC = cc;
                CARGO_BUILD_RUSTFLAGS = staticRustFlags ++ [ "-Clinker=${cc}" "-lframework=${Cocoa}/Library/Frameworks" ];
                postInstall = mkPostInstall {
                  inherit pkgs;
                  bin = "${pkgs.qemu}/bin/qemu-aarch64 ./himalaya";
                };
              };
          };
        };
      };

      mkToolchain = import ./rust-toolchain.nix fenix;

      mkPkgsCross = buildSystem: crossSystem: import nixpkgs {
        system = buildSystem;
        crossSystem.config = crossSystem;
      };

      mkPostInstall = { pkgs, bin ? "./himalaya" }: with pkgs; ''
        cd $out/bin
        mkdir -p {man,completions}
        ${bin} man ./man
        ${bin} completion bash > ./completions/himalaya.bash
        ${bin} completion elvish > ./completions/himalaya.elvish
        ${bin} completion fish > ./completions/himalaya.fish
        ${bin} completion powershell > ./completions/himalaya.powershell
        ${bin} completion zsh > ./completions/himalaya.zsh
        tar -czf himalaya.tgz himalaya* man completions
        ${zip}/bin/zip -r himalaya.zip himalaya* man completions
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
              # rnix-lsp
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

      mkPackage = pkgs: buildPlatform: targetPlatform: package:
        let
          toolchain = mkToolchain.fromTarget {
            inherit pkgs buildPlatform targetPlatform;
          };
          naersk' = naersk.lib.${buildPlatform}.override {
            cargo = toolchain;
            rustc = toolchain;
          };
          package' = {
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
            postInstall = mkPostInstall { inherit pkgs; };
          } // package;
        in
        naersk'.buildPackage package';

      mkPackages = system:
        let
          pkgs = import nixpkgs { inherit system; };
          mkPackage' = target: package: mkPackage pkgs system package.rustTarget (package.override { inherit system pkgs; });
        in
        builtins.mapAttrs mkPackage' crossBuildTargets.${system};

      mkApp = drv:
        let exePath = drv.passthru.exePath or "/bin/himalaya"; in
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

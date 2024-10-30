{
  description = "CLI to manage emails";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      # https://github.com/nix-community/fenix/pull/145
      # url = "github:nix-community/fenix";
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

      crossSystems = {
        x86_64-linux = {
          x86_64-linux = {
            rustTarget = "x86_64-unknown-linux-musl";
          };

          aarch64-linux = rec {
            rustTarget = "aarch64-unknown-linux-musl";
            runner = { pkgs, himalaya }: "${pkgs.qemu}/bin/qemu-aarch64 ${himalaya}";
            mkPackage = { system, ... }: package:
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
            mkPackage = { pkgs, ... }: package:
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

        aarch64-linux.aarch64-linux = {
          rustTarget = "aarch64-unknown-linux-musl";
        };

        x86_64-darwin.x86_64-darwin = {
          rustTarget = "x86_64-apple-darwin";
          mkPackage = { pkgs, ... }: package:
            let inherit (pkgs.darwin.apple_sdk_11_0.frameworks) Security;
            in package // {
              NIX_LDFLAGS = "-F${Security}/Library/Frameworks -framework Security";
            };
        };

        aarch64-darwin.aarch64-darwin = {
          rustTarget = "aarch64-apple-darwin";
        };
      };

      eachBuildSystem = lib.genAttrs (builtins.attrNames crossSystems);

      mkPkgsCross = buildSystem: crossSystem: import nixpkgs {
        system = buildSystem;
        crossSystem.config = crossSystem;
      };

      mkToolchain = import ./rust-toolchain.nix fenix;

      mkApp = { pkgs, buildSystem, targetSystem ? buildSystem }:
        let
          himalaya = lib.getExe self.packages.${buildSystem}.${targetSystem};
          wrapper = crossSystems.${buildSystem}.${targetSystem}.runner or (_: himalaya) { inherit pkgs himalaya; };
          program = lib.getExe (pkgs.writeShellScriptBin "himalaya" "${wrapper} $@");
          app = { inherit program; type = "app"; };
        in
        app;

      mkApps = buildSystem:
        let
          pkgs = import nixpkgs { system = buildSystem; };
          mkApp' = targetSystem: _: mkApp { inherit pkgs buildSystem targetSystem; };
          defaultApp = mkApp { inherit pkgs buildSystem; };
          apps = builtins.mapAttrs mkApp' crossSystems.${buildSystem};
        in
        apps // { default = defaultApp; };

      mkPackage = { pkgs, buildSystem, targetSystem ? buildSystem }:
        let
          targetConfig = crossSystems.${buildSystem}.${targetSystem};
          toolchain = mkToolchain.fromTarget {
            inherit pkgs buildSystem;
            targetSystem = targetConfig.rustTarget;
          };
          rust = naersk.lib.${buildSystem}.override {
            cargo = toolchain;
            rustc = toolchain;
          };
          mkPackage' = targetConfig.mkPackage or (_: p: p);
          himalaya = "./himalaya";
          runner = targetConfig.runner or (_: himalaya) { inherit pkgs himalaya; };
          package = mkPackage' { inherit pkgs; system = buildSystem; } {
            name = "himalaya";
            src = gitignoreSource ./.;
            strictDeps = true;
            doCheck = false;
            auditable = false;
            nativeBuildInputs = with pkgs; [ pkg-config ];
            CARGO_BUILD_TARGET = targetConfig.rustTarget;
            CARGO_BUILD_RUSTFLAGS = [ "-Ctarget-feature=+crt-static" ];
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
              mv himalaya.tgz ../

              ${pkgs.zip}/bin/zip -r himalaya.zip himalaya* share
              mv himalaya.zip ../
            '';
          };
        in
        rust.buildPackage package;

      mkPackages = buildSystem:
        let
          pkgs = import nixpkgs { system = buildSystem; };
          mkPackage' = targetSystem: _: mkPackage { inherit pkgs buildSystem targetSystem; };
          defaultPackage = mkPackage { inherit pkgs buildSystem; };
          packages = builtins.mapAttrs mkPackage' crossSystems.${buildSystem};
        in
        packages // { default = defaultPackage; };

      mkDevShells = buildSystem:
        let
          pkgs = import nixpkgs { system = buildSystem; };
          rust-toolchain = mkToolchain.fromFile { inherit buildSystem; };
          defaultShell = pkgs.mkShell {
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
        in
        { default = defaultShell; };

    in
    {
      apps = eachBuildSystem mkApps;
      packages = eachBuildSystem mkPackages;
      devShells = eachBuildSystem mkDevShells;
    };
}

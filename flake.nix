{
  description = "CLI to manage emails";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      # https://github.com/nix-community/fenix/pull/145
      url = "github:soywod/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix }:
    let
      inherit (nixpkgs) lib;

      crossSystems = {
        aarch64-apple = [ ];
        aarch64-linux = [
          "aarch64-unknown-linux-musl"
        ];
        x86_64-apple = [ ];
        x86_64-linux = [
          "aarch64-unknown-linux-musl"
          "armv6l-unknown-linux-musleabihf"
          "armv7l-unknown-linux-musleabihf"
          "i686-unknown-linux-musl"
          "x86_64-unknown-linux-musl"
          "x86_64-w64-mingw32"
        ];
      };

      # Dev shells

      mkDevShell = system: {
        default = import ./shell.nix {
          pkgs = import nixpkgs { inherit system; };
          fenix = fenix.packages.${system};
        };
      };

      # Packages

      mkPackages = system: mkCrossPackages system // {
        default = withGitEnvs (import ./default.nix ({
          pkgs = import nixpkgs { inherit system; };
          fenix = fenix.packages.${system};
        }));
      };

      withGitEnvs = package: package.overrideAttrs (drv: {
        GIT_REV = drv.GIT_REV or self.rev or self.dirtyRev or "dirty";
        GIT_DESCRIBE = drv.GIT_DESCRIBE or "flake-" + self.shortRev or self.dirtyShortRev or "dirty";
      });

      mkCrossPackages = system:
        lib.attrsets.mergeAttrsList (map (mkCrossPackage system) crossSystems.${system});

      mkCrossPackage = system: crossConfig:
        let
          pkgs = import nixpkgs { inherit system; };
          crossSystem = { config = crossConfig; isStatic = true; };
          crossPkgs = import nixpkgs { inherit system crossSystem; };
          crossPkg = import ./default.nix { inherit pkgs crossPkgs; fenix = fenix.packages.${system}; };
        in
        { "cross-${crossPkgs.hostPlatform.system}" = withGitEnvs crossPkg; };

      # Apps

      mkApps = system: mkCrossApps system // {
        default = { type = "app"; program = lib.getExe self.packages.${system}.default; };
      };

      mkCrossApps = system:
        lib.attrsets.mergeAttrsList (map (mkCrossApp system) crossSystems.${system});

      mkCrossApp = system: crossConfig:
        let
          pkgs = import nixpkgs { inherit system; };
          emulator = crossPkgs.hostPlatform.emulator pkgs;
          crossSystem = { config = crossConfig; isStatic = true; };
          crossPkgs = import nixpkgs { inherit system crossSystem; };
          crossPkgName = "cross-${crossPkgs.hostPlatform.system}";
          crossPkgExe = lib.getExe self.packages.${system}.${crossPkgName};
          program = lib.getExe (pkgs.writeShellScriptBin "himalaya" "${emulator} ${crossPkgExe} $@");
        in
        { "${crossPkgName}" = { inherit program; type = "app"; }; };
    in

    {
      devShells = lib.genAttrs (lib.attrNames crossSystems) mkDevShell;
      packages = lib.genAttrs (lib.attrNames crossSystems) mkPackages;
      apps = lib.genAttrs (lib.attrNames crossSystems) mkApps;
    };
}

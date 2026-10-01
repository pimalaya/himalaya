{
  nixpkgs ? <nixpkgs>,
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  ...
}@args:

let
  himalaya = import ./default.nix (
    removeAttrs args [
      "crossPkgs"
      "isStatic"
      "target"
    ]
  );

in
pimalaya.mkDefault (
  {
    src = ./.;
    version = "2.1.0";
    mkPackage = (
      {
        lib,
        pkgs,
        rustPlatform,
        defaultFeatures,
        features,
        buildPackages,
      }:

      let
        inherit (pkgs) sqlite stdenv windows;

        # NOTE: nixpkgs' mingw sqlite fails its pthread probe and compiles
        # single-threaded, defining no sqlite3_mutex_* rusqlite links against
        sqlite' =
          if stdenv.hostPlatform.isWindows then
            sqlite.overrideAttrs (old: {
              buildInputs = (old.buildInputs or [ ]) ++ [ windows.pthreads ];
            })
          else
            sqlite;

        buildFeatures = lib.splitString "," features;

        systemSqlite =
          (defaultFeatures || builtins.elem "pimdir" buildFeatures)
          && !builtins.elem "vendored" buildFeatures;

      in
      (pkgs.callPackage "${nixpkgs}/pkgs/by-name/hi/himalaya/package.nix" {
        inherit lib rustPlatform buildFeatures;
        buildPackages = buildPackages // {
          inherit himalaya;
        };
        installShellCompletions = false;
        installManPages = false;
        buildNoDefaultFeatures = !defaultFeatures;
      })
      # HACK: needed until the v2.1.0 derivation lands on nixpkgs's master
      .overrideAttrs
        (drv: {
          buildInputs = (drv.buildInputs or [ ]) ++ lib.optional systemSqlite sqlite';

          # pkg-config hands the linker libsqlite3 but no rpath, leaving a
          # binary that cannot find it: not in postInstall, which runs it, nor
          # once installed.
          env = (drv.env or { }) // {
            NIX_LDFLAGS = lib.optionalString systemSqlite ("-rpath " + lib.getLib sqlite' + "/lib");
          };

          postInstall =
            let
              inherit (pkgs) stdenv;
              exe =
                if stdenv.buildPlatform.canExecute stdenv.hostPlatform then
                  "$out/bin/himalaya"
                else
                  lib.getExe himalaya;
            in
            ''
              mkdir -p $out/share/{completions,man,schemas}
              ${exe} completion -d "$out"/share/completions bash elvish fish powershell zsh
              ${exe} manual -d "$out"/share/man
              ${exe} json-schema -d "$out"/share/schemas
            '';
        })
    );
  }
  // removeAttrs args [ "pimalaya" ]
)

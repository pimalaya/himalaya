{
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  ...
}@args:

pimalaya.mkDefault (
  {
    src = ./.;
    version = "1.1.0";
    mkPackage = (
      {
        lib,
        pkgs,
        defaultFeatures,
        features,
        ...
      }:
      pkgs.himalaya.overrideAttrs (_: {
        installShellCompletions = false;
        installManPages = false;
        buildNoDefaultFeatures = !defaultFeatures;
        buildFeatures = lib.splitString "," features;
      })
    );
  }
  // removeAttrs args [ "pimalaya" ]
)

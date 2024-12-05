{ pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz")
, ...
} @args:

pimalaya.mkDefault ({
  src = ./.;
  version = "1.0.0";
  mkPackage = ({ lib, pkgs, rustPlatform, defaultFeatures, features }: pkgs.callPackage ./package.nix {
    inherit lib rustPlatform;
    apple-sdk = if pkgs.hostPlatform.isx86_64 then pkgs.apple-sdk_13 else pkgs.apple-sdk_14;
    installShellCompletions = false;
    installManPages = false;
    buildNoDefaultFeatures = !defaultFeatures;
    buildFeatures = lib.splitString "," features;
  });
} // removeAttrs args [ "pimalaya" ])

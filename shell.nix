# This file exists for legacy nix-shell
# https://nixos.wiki/wiki/Flakes#Using_flakes_project_from_a_legacy_Nix
# You generally do *not* have to modify this ever.
(import (
  let
    lock = builtins.fromJSON (builtins.readFile ./flake.lock);
  in fetchTarball {
    url = "https://github.com/edolstra/flake-compat/archive/${lock.nodes.flake-compat.locked.rev}.tar.gz";
    sha256 = lock.nodes.flake-compat.locked.narHash; }
) {
  src =  ./.;
}).shellNix

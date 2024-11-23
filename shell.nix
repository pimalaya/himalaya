{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
}:

let
  package = import ./default.nix { inherit pkgs; };
in

pkgs.mkShell {
  inputsFrom = [ package ];
}

{ pkgs ? import <nixpkgs> { } }:

let
  package = import ./default.nix {
    isStatic = false;
  };
in

pkgs.mkShell {
  inputsFrom = [
    package
  ];

  buildInputs = with pkgs; [
    nixd
    nixpkgs-fmt
  ];
}

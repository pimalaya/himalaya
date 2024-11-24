{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    nixd
    nixpkgs-fmt
  ];
}

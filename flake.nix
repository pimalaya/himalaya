{
  description = "CLI to manage emails";

  inputs = {
    # TODO: https://github.com/NixOS/nixpkgs/pull/358989
    # nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs.url = "github:soywod/nixpkgs";
    fenix = {
      # TODO: https://github.com/nix-community/fenix/pull/145
      # url = "github:nix-community/fenix";
      url = "github:soywod/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pimalaya = {
      url = "github:pimalaya/nix";
      flake = false;
    };
  };

  outputs = inputs: (import inputs.pimalaya).mkFlakeOutputs inputs {
    shell = ./shell.nix;
    default = ./default.nix;
  };
}

{
  description = "CLI to manage emails";

  inputs = {
    # FIXME: when #358989 lands on nixos-unstable
    # https://nixpk.gs/pr-tracker.html?pr=358989
    nixpkgs.url = "github:nixos/nixpkgs/staging-next";
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

  outputs =
    inputs:
    (import inputs.pimalaya).mkFlakeOutputs inputs {
      shell = ./shell.nix;
      default = ./default.nix;
    };
}

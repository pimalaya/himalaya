{
  description = "Client library and CLI to discover PIM-related services, written in Rust";

  inputs = {
    nixpkgs = {
      # until crates.io fix fully backported
      url = "github:nixos/nixpkgs?tag=25.11&rev=c767db50e209f33ffce3c18165b36101079d367d";
    };
    fenix = {
      url = "github:nix-community/fenix/monthly";
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

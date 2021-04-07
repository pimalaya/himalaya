let 
  pkgs = import <nixpkgs> {};
in 
pkgs.rustPlatform.buildRustPackage rec {
  pname = "himalaya";
  version = "0.2.2";

  # TODO: gitignore
  src = ./.;

  nativeBuildInputs = with pkgs; [
    pkg-config 
    openssl.dev
  ];
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

  # When Cargo dependencies change, the sha here will have to be updated.
  # `nix-build` will give you the new sha.
  cargoSha256 = "sha256-G0f96pZe/KkuxQiFK45namDrSnDtYYcI/Ml00rt7G5M=";

  meta = with pkgs.stdenv.lib; {
    description = "Minimalist CLI email client";
    homepage = "https://github.com/soywod/himalaya";
  };
}

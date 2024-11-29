# TODO: move this to nixpkgs
# This file aims to be a replacement for the nixpkgs derivation.

{ lib
, pkg-config
, rustPlatform
, fetchFromGitHub
, stdenv
, apple-sdk
, installShellFiles
, installShellCompletions ? stdenv.buildPlatform.canExecute stdenv.hostPlatform
, installManPages ? stdenv.buildPlatform.canExecute stdenv.hostPlatform
, notmuch
, gpgme
, buildNoDefaultFeatures ? false
, buildFeatures ? [ ]
}:

let
  version = "1.0.0-beta.4";
  hash = "sha256-NrWBg0sjaz/uLsNs8/T4MkUgHOUvAWRix1O5usKsw6o=";
  cargoHash = "sha256-YS8IamapvmdrOPptQh2Ef9Yold0IK1XIeGs0kDIQ5b8=";
in

rustPlatform.buildRustPackage rec {
  inherit cargoHash version;
  inherit buildNoDefaultFeatures buildFeatures;

  pname = "himalaya";

  src = fetchFromGitHub {
    inherit hash;
    owner = "pimalaya";
    repo = "himalaya";
    rev = "v${version}";
  };

  nativeBuildInputs = [ pkg-config ]
    ++ lib.optional (installManPages || installShellCompletions) installShellFiles;

  buildInputs = [ ]
    ++ lib.optional stdenv.hostPlatform.isDarwin apple-sdk
    ++ lib.optional (builtins.elem "notmuch" buildFeatures) notmuch
    ++ lib.optional (builtins.elem "pgp-gpg" buildFeatures) gpgme;

  doCheck = false;
  auditable = false;

  # unit tests only
  cargoTestFlags = [ "--lib" ];

  postInstall = ''
    mkdir -p $out/share/{applications,completions,man}
    cp assets/himalaya.desktop "$out"/share/applications/
  '' + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    "$out"/bin/himalaya man "$out"/share/man
  '' + lib.optionalString installManPages ''
    installManPage "$out"/share/man/*
  '' + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    "$out"/bin/himalaya completion bash > "$out"/share/completions/himalaya.bash
    "$out"/bin/himalaya completion elvish > "$out"/share/completions/himalaya.elvish
    "$out"/bin/himalaya completion fish > "$out"/share/completions/himalaya.fish
    "$out"/bin/himalaya completion powershell > "$out"/share/completions/himalaya.powershell
    "$out"/bin/himalaya completion zsh > "$out"/share/completions/himalaya.zsh
  '' + lib.optionalString installShellCompletions ''
    installShellCompletion "$out"/share/completions/himalaya.{bash,fish,zsh}
  '';

  meta = rec {
    description = "CLI to manage emails";
    mainProgram = "himalaya";
    homepage = "https://github.com/pimalaya/himalaya";
    changelog = "${homepage}/blob/v${version}/CHANGELOG.md";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ soywod toastal yanganto ];
  };
}

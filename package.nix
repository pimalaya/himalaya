{ lib
, pkg-config
, hostPlatform
, rustPlatform
, fetchFromGitHub
, stdenv
, darwin
, installShellFiles
, installShellCompletions ? stdenv.buildPlatform.canExecute stdenv.hostPlatform
, installManPages ? stdenv.buildPlatform.canExecute stdenv.hostPlatform
, notmuch
, gpgme
, buildNoDefaultFeatures ? false
, buildFeatures ? [ ]
, binutils
}:

let
  # HACK: work around https://github.com/NixOS/nixpkgs/issues/177129
  # Though this is an issue between Clang and GCC,
  # so it may not get fixed anytime soon...
  empty-libgcc_eh = stdenv.mkDerivation {
    pname = "empty-libgcc_eh";
    version = "0";
    dontUnpack = true;
    installPhase = ''
      mkdir -p "$out"/lib
      "${lib.getExe' binutils "ar"}" r "$out"/lib/libgcc_eh.a
    '';
  };
in

rustPlatform.buildRustPackage rec {
  inherit buildNoDefaultFeatures buildFeatures;

  pname = "himalaya";
  version = "1.0.0-beta.4";

  src = fetchFromGitHub {
    owner = "soywod";
    repo = "himalaya";
    rev = "v${version}";
    hash = "sha256-NrWBg0sjaz/uLsNs8/T4MkUgHOUvAWRix1O5usKsw6o=";
  };

  cargoHash = "sha256-YS8IamapvmdrOPptQh2Ef9Yold0IK1XIeGs0kDIQ5b8=";

  # unit tests only
  doCheck = false;
  cargoTestFlags = [ "--lib" ];

  nativeBuildInputs = [ pkg-config ]
    ++ lib.optional hostPlatform.isDarwin libiconv
    ++ lib.optional (installManPages || installShellCompletions) installShellFiles;

  buildInputs = [ ]
    ++ lib.optional hostPlatform.isWindows empty-libgcc_eh
    ++ lib.optional hostPlatform.isDarwin libiconv
    ++ lib.optionals hostPlatform.isDarwin (with darwin.apple_sdk_11_0.frameworks; [ Security ])
    ++ lib.optional (builtins.elem "notmuch" buildFeatures) notmuch
    ++ lib.optional (builtins.elem "pgp-gpg" buildFeatures) gpgme;

  postInstall = lib.optionalString installManPages ''
    mkdir -p $out/man
    $out/bin/himalaya man $out/man
    installManPage $out/man/*
  '' + lib.optionalString installShellCompletions ''
    installShellCompletion --cmd himalaya \
      --bash <($out/bin/himalaya completion bash) \
      --fish <($out/bin/himalaya completion fish) \
      --zsh <($out/bin/himalaya completion zsh)
  '';

  meta = {
    description = "CLI to manage emails";
    mainProgram = "himalaya";
    homepage = "https://github.com/pimalaya/himalaya/";
    changelog = "https://github.com/soywod/himalaya/blob/v${version}/CHANGELOG.md";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ soywod toastal yanganto ];
  };
}

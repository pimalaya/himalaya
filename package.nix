{ lib
, darwin
, windows
, rustPlatform
, fetchFromGitHub
, hostPlatform
, stdenv
, pkg-config
, installShellFiles
, installShellCompletions ? stdenv.buildPlatform.canExecute stdenv.hostPlatform
, installManPages ? stdenv.buildPlatform.canExecute stdenv.hostPlatform
, notmuch
, gpgme
, pkgsCross
, buildNoDefaultFeatures ? false
, buildFeatures ? [ ]
, cargoLock ? null
}:

rustPlatform.buildRustPackage rec {
  inherit buildNoDefaultFeatures buildFeatures cargoLock;

  pname = "himalaya";
  version = "1.0.0-beta.4";

  src = fetchFromGitHub {
    owner = "soywod";
    repo = "himalaya";
    rev = "v${version}";
    hash = "sha256-NrWBg0sjaz/uLsNs8/T4MkUgHOUvAWRix1O5usKsw6o=";
  };

  cargoHash = "sha256-YS8IamapvmdrOPptQh2Ef9Yold0IK1XIeGs0kDIQ5b8=";

  NIX_BUILD_CORES = 4;
  # CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
  TARGET_CC = "${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc";
  CARGO_BUILD_RUSTFLAGS = [ "-Ctarget-feature=+crt-static" "-Clinker=${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc" ];
  # CARGO_BUILD_RUSTFLAGS = [ "-Clinker=${stdenv.cc}/bin/${stdenv.cc.targetPrefix}cc" ];
  CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

  doCheck = false;
  auditable = false;
  strictDeps = true;
  cargoTestFlags = [
    # Only run lib tests (unit tests)
    # All other tests are integration tests which should not be run with Nix build
    "--lib"
  ];

  depsBuildBuild = [ stdenv.cc windows.pthreads ];

  nativeBuildInputs = [ ]
    ++ lib.optional (installManPages || installShellCompletions) installShellFiles;

  buildInputs = [ ]
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

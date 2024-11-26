{ lib
, pkg-config
, rustPlatform
, windows
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

  NIX_LDFLAGS = lib.optionals (stdenv.hostPlatform.isWindows && stdenv.hostPlatform.isi686) [
    # "-L${windows.mcfgthreads}/lib"
    "-L${windows.mingwrt}/lib"
    #"-L${windows.w32api}/lib"
    #"-lmcfgthread"
    #"-lunwind"
  ];

  # unit tests only
  doCheck = false;
  cargoTestFlags = [ "--lib" ];

  nativeBuildInputs = [ pkg-config ]
    ++ lib.optional (installManPages || installShellCompletions) installShellFiles;

  buildInputs = [ ]
    ++ lib.optional stdenv.hostPlatform.isDarwin apple-sdk
    ++ lib.optional (stdenv.hostPlatform.isWindows && stdenv.hostPlatform.isi686) windows.mcfgthreads
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

# TODO: move this to nixpkgs
# This file aims to be a replacement for the nixpkgs derivation.

{
  buildFeatures ? [ ],
  buildNoDefaultFeatures ? false,
  buildPackages,
  fetchFromGitHub,
  installManPages ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellCompletions ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellFiles,
  lib,
  openssl,
  pkg-config,
  rustPlatform,
  sqlite,
  stdenv,
  windows,
}:

let
  nativeTls = builtins.elem "native-tls" buildFeatures;

  # NOTE: nixpkgs' mingw sqlite fails its pthread probe and compiles
  # single-threaded, defining no sqlite3_mutex_* rusqlite links against
  sqlite' =
    if stdenv.hostPlatform.isWindows then
      sqlite.overrideAttrs (old: {
        buildInputs = (old.buildInputs or [ ]) ++ [ windows.pthreads ];
      })
    else
      sqlite;

  # The pimdir store is what links SQLite. The defaults carry both `pimdir`
  # and `vendored`, which builds a SQLite from source, so the
  # system library is linked only with the defaults off, `pimdir` asked for
  # by name and `vendored` left out, as pimalaya/nix builds it.
  systemSqlite =
    buildNoDefaultFeatures
    && builtins.elem "pimdir" buildFeatures
    && !builtins.elem "vendored" buildFeatures;

in
rustPlatform.buildRustPackage (finalAttrs: {
  __structuredAttrs = true;

  inherit buildNoDefaultFeatures buildFeatures;

  pname = "himalaya";
  version = "3.0.0";
  cargoHash = "";

  src = fetchFromGitHub {
    owner = "pimalaya";
    repo = finalAttrs.pname;
    tag = "v${finalAttrs.version}";
    hash = "";
  };

  # openssl should not be provided by vendors, not even on windows
  env.OPENSSL_NO_VENDOR = 1;

  # pkg-config hands the linker libsqlite3 but no rpath, leaving a binary that
  # cannot find it: not in postInstall, which runs it, nor once installed.
  env.NIX_LDFLAGS = lib.optionalString systemSqlite ("-rpath " + lib.getLib sqlite' + "/lib");

  nativeBuildInputs = [
    pkg-config
    installShellFiles
  ];

  buildInputs = lib.optional systemSqlite sqlite' ++ lib.optional nativeTls openssl;

  postInstall =
    let
      exe =
        if stdenv.buildPlatform.canExecute stdenv.hostPlatform then
          "$out/bin/${finalAttrs.pname}"
        else
          lib.getExe buildPackages.${finalAttrs.pname};
    in
    ''
      mkdir -p $out/share/{completions,man,schemas}
      install -Dm644 config.sample.toml "$out"/share/doc/${finalAttrs.pname}/config.sample.toml
      ${exe} completion -d "$out"/share/completions bash elvish fish powershell zsh
      ${exe} manual -d "$out"/share/man
      ${exe} json-schema -d "$out"/share/schemas
    ''
    + lib.optionalString installManPages ''
      installManPage "$out"/share/man/*
    ''
    + lib.optionalString installShellCompletions ''
      installShellCompletion --cmd ${finalAttrs.pname} \
        --bash "$out"/share/completions/${finalAttrs.pname}.bash \
        --fish "$out"/share/completions/${finalAttrs.pname}.fish \
        --zsh "$out"/share/completions/_${finalAttrs.pname}
    '';

  meta = {
    description = "CLI to manage emails";
    mainProgram = finalAttrs.pname;
    homepage = "https://github.com/pimalaya/${finalAttrs.pname}";
    changelog = "https://github.com/pimalaya/${finalAttrs.pname}/releases/tag/${finalAttrs.src.tag}";
    license = with lib.licenses; [
      asl20
      mit
    ];
    maintainers = with lib.maintainers; [
      soywod
      yanganto
    ];
  };
})

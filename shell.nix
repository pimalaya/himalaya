{
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
}:

pimalaya.mkShell {
  extraBuildInputs = "nixd,nixfmt-rfc-style,git-cliff,cargo-deny";
}

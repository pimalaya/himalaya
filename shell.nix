{ pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz")
, ...
} @args:

pimalaya.mkShell ({ rustToolchainFile = ./rust-toolchain.toml; }
  // removeAttrs args [ "pimalaya" ])

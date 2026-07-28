use std::env;

use pimalaya_cli::build::{features_env, git_envs, target_envs};

fn main() {
    features_env(include_str!("./Cargo.toml"));
    target_envs();
    git_envs();

    // `backend` collapses the repeated mailbox-backend feature list: it
    // is set when any backend that exposes a mailbox is enabled (every
    // backend except the send-only SMTP). Code that also needs SMTP uses
    // `any(backend, feature = "smtp")`. Cargo exports `CARGO_FEATURE_<NAME>`
    // for every enabled feature.
    println!("cargo::rustc-check-cfg=cfg(backend)");

    let backend = ["IMAP", "JMAP", "GMAIL", "MSGRAPH", "MAILDIR", "M2DIR"]
        .iter()
        .any(|feature| env::var_os(format!("CARGO_FEATURE_{feature}")).is_some());

    if backend {
        println!("cargo::rustc-cfg=backend");
    }
}

//! # Build script
//!
//! Sets the `backend` cfg when any backend exposing a mailbox is enabled
//! (every one but the send-only SMTP), collapsing the repeated feature list.
//! Code that also needs SMTP uses `any(backend, feature = "smtp")`.

use std::env;

use pimalaya_cli::build::{features_env, git_envs, target_envs};

fn main() {
    features_env(include_str!("./Cargo.toml"));
    target_envs();
    git_envs();

    println!("cargo::rustc-check-cfg=cfg(backend)");

    let backend = [
        "IMAP", "JMAP", "GMAIL", "MSGRAPH", "MAILDIR", "MBOX", "PIMDIR",
    ]
    .iter()
    .any(|feature| env::var_os(format!("CARGO_FEATURE_{feature}")).is_some());

    if backend {
        println!("cargo::rustc-cfg=backend");
    }
}

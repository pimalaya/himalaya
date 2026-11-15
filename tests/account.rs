//! # Account and backend resolution
//!
//! The built binary's failure object for an account it cannot take, and
//! for a backend the build leaves out: its block loads, so one file
//! serves every build, but a command reaching it fails coded
//! `backend-not-built`, naming the cargo feature. The second half runs on
//! a reduced build, such as `cargo test --no-default-features --features
//! rustls-ring,maildir --test account`.

use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs the binary under `--json` with `args` over the configuration
/// `toml`, in a home of its own, its stdin closed.
fn run(home: &Path, toml: &str, args: &[&str]) -> Output {
    let config = home.join("config.toml");
    fs::write(&config, toml).unwrap();

    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["--json", "-c", config.to_str().unwrap()])
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// What `output` printed, once it exited with `status`.
fn printed(output: &Output, status: i32) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(status), "{stdout}{stderr}");
    serde_json::from_str(&stdout).unwrap()
}

#[test]
fn an_unknown_account_name_is_coded_account_missing() {
    let home = TempDir::new().unwrap();
    let toml = "[accounts.a]\ndefault = true\n";

    let output = run(home.path(), toml, &["-a", "nope", "account", "capability"]);
    let json = printed(&output, 1);
    let error = json["error"].as_str().unwrap();
    assert_eq!(json["code"], "account-missing", "{json}");
    assert!(error.contains("Account `nope` not found"), "{error}");
}

#[test]
fn the_name_default_is_a_name_like_any_other() {
    let home = TempDir::new().unwrap();
    let toml = "[accounts.a]\ndefault = true\n";

    let args = ["-a", "default", "account", "capability"];
    let output = run(home.path(), toml, &args);
    let json = printed(&output, 1);
    let error = json["error"].as_str().unwrap();
    assert_eq!(json["code"], "account-missing", "{json}");
    assert!(error.contains("omit `-a`"), "{error}");
}

#[test]
fn no_account_marked_default_is_coded_account_default_missing() {
    let home = TempDir::new().unwrap();
    let toml = "[accounts.a]\n[accounts.b]\n";

    let output = run(home.path(), toml, &["account", "capability"]);
    let json = printed(&output, 1);
    assert_eq!(json["code"], "account-default-missing", "{json}");
}

#[test]
fn several_accounts_marked_default_are_coded_account_default_several() {
    let home = TempDir::new().unwrap();
    let toml = "[accounts.a]\ndefault = true\n[accounts.b]\ndefault = true\n";

    let output = run(home.path(), toml, &["account", "capability"]);
    let json = printed(&output, 1);
    let error = json["error"].as_str().unwrap();
    assert_eq!(json["code"], "account-default-several", "{json}");
    assert!(error.contains("(a, b)"), "{error}");
}

/// An account over an mbox alone, which the build leaves out.
#[cfg(not(feature = "mbox"))]
const MBOX: &str = "[accounts.a]\ndefault = true\nmbox.root = \"/nonexistent\"\n";

/// Asserts `json` is the failure of reaching the mbox backend.
#[cfg(not(feature = "mbox"))]
fn assert_mbox_not_built(json: &Value) {
    let error = json["error"].as_str().unwrap();
    assert_eq!(json["code"], "backend-not-built", "{json}");
    assert!(error.contains("`--features mbox`"), "{error}");
}

#[cfg(not(feature = "mbox"))]
#[test]
fn a_unified_command_over_a_backend_left_out_is_backend_not_built() {
    let home = TempDir::new().unwrap();

    let output = run(home.path(), MBOX, &["mailbox", "list"]);
    assert_mbox_not_built(&printed(&output, 1));

    let output = run(home.path(), MBOX, &["envelope", "list"]);
    assert_mbox_not_built(&printed(&output, 1));
}

#[cfg(not(feature = "mbox"))]
#[test]
fn a_backend_flag_naming_a_backend_left_out_is_backend_not_built() {
    let home = TempDir::new().unwrap();

    let output = run(
        home.path(),
        MBOX,
        &["--backend", "mbox", "envelope", "list"],
    );
    assert_mbox_not_built(&printed(&output, 1));
}

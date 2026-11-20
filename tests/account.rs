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

/// `account list` names the backends this build serves under `backends`
/// and the ones it leaves out under `notBuilt`, marked in text.
#[test]
fn account_list_splits_the_declared_backends_by_build() {
    let home = TempDir::new().unwrap();
    let toml = "[accounts.a]\ndefault = true\n\
                maildir.root = \"/nonexistent\"\n\
                mbox.root = \"/nonexistent\"\n";

    let (mut built, mut not_built) = (Vec::new(), Vec::new());
    for (backend, compiled) in [
        ("maildir", cfg!(feature = "maildir")),
        ("mbox", cfg!(feature = "mbox")),
    ] {
        match compiled {
            true => built.push(backend),
            false => not_built.push(backend),
        }
    }

    let output = run(home.path(), toml, &["account", "list"]);
    let json = printed(&output, 0);
    let account = &json["accounts"][0];
    assert_eq!(account["name"], "a", "{json}");
    assert_eq!(account["backends"], serde_json::json!(built), "{json}");
    assert_eq!(account["notBuilt"], serde_json::json!(not_built), "{json}");

    let config = home.path().join("config.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "account", "list"])
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    let marked: Vec<String> = not_built
        .iter()
        .map(|b| format!("{b} (not built)"))
        .collect();
    let cell = [built.join(","), marked.join(",")]
        .into_iter()
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(text, format!("a  {cell}  default\n"));
}

/// `account check` names the TLS each network backend connected with:
/// the account's `tls` reaching both, the SMTP block's own provider
/// winning over it, a local backend naming none.
#[cfg(all(feature = "imap", feature = "smtp", feature = "maildir"))]
#[test]
fn an_account_check_names_the_trust_of_each_transport() {
    let home = TempDir::new().unwrap();
    let maildir = home.path().join("mail");
    for subdir in ["cur", "new", "tmp"] {
        fs::create_dir_all(maildir.join(subdir)).unwrap();
    }
    let toml = format!(
        "[accounts.a]\ndefault = true\n\
         tls.cert = \"/etc/ca.pem\"\n\
         imap.server = \"imaps://127.0.0.1:1\"\n\
         imap.sasl.plain.username = \"a\"\n\
         imap.sasl.plain.password.raw = \"x\"\n\
         smtp.server = \"smtps://127.0.0.1:1\"\n\
         smtp.tls.provider = \"rustls\"\n\
         maildir.root = {:?}\n",
        maildir.to_str().unwrap()
    );

    let output = run(home.path(), &toml, &["account", "check"]);
    let json = printed(&output, 3);
    let row = |backend: &str| {
        let rows = json["accounts"][0]["backends"].as_array().unwrap();
        rows.iter()
            .find(|row| row["backend"] == backend)
            .unwrap()
            .clone()
    };

    let imap = row("imap");
    assert_eq!(imap["tls"]["cert"], "/etc/ca.pem", "{json}");
    assert_eq!(imap["tls"]["provider"], Value::Null, "{json}");
    let smtp = row("smtp");
    assert_eq!(smtp["tls"]["cert"], "/etc/ca.pem", "{json}");
    assert_eq!(smtp["tls"]["provider"], "rustls", "{json}");
    assert_eq!(smtp["tls"]["clientCert"], Value::Null, "{json}");
    assert_eq!(row("maildir")["tls"], Value::Null, "{json}");
}

/// A cleartext connection names no TLS, `tls` printing `null` as for a
/// local backend, while a STARTTLS upgrade names the build's defaults.
#[cfg(all(feature = "imap", feature = "smtp"))]
#[test]
fn an_account_check_names_no_tls_over_a_cleartext_connection() {
    let home = TempDir::new().unwrap();
    let toml = "[accounts.a]\ndefault = true\n\
                imap.server = \"imap://127.0.0.1:1\"\n\
                smtp.server = \"smtp://127.0.0.1:1\"\n\
                smtp.starttls = true\n";

    let output = run(home.path(), toml, &["account", "check"]);
    let json = printed(&output, 3);
    let account = &json["accounts"][0];
    assert_eq!(account["account"], "a", "{json}");

    let backends = account["backends"].as_array().unwrap();
    let row = |backend: &str| backends.iter().find(|row| row["backend"] == backend);

    assert_eq!(row("imap").unwrap()["tls"], Value::Null, "{json}");
    let defaults = serde_json::json!({
        "provider": null,
        "crypto": null,
        "cert": null,
        "clientCert": null,
    });
    assert_eq!(row("smtp").unwrap()["tls"], defaults, "{json}");
}

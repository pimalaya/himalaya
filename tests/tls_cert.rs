//! # TLS certificate files
//!
//! A certificate the configuration names but that cannot be read is
//! refused before any connection, the failure naming its path.

#![cfg(feature = "imap")]

use std::{
    fs,
    net::TcpListener,
    process::{Command, Stdio},
    thread,
};

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn a_missing_certificate_is_named() {
    let dir = TempDir::new().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let held: Vec<_> = listener.incoming().take(2).collect();
        drop(held);
    });

    let cert = dir.path().join("missing.pem");
    let config = dir.path().join("config.toml");
    let toml = format!(
        "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n\
         tls.cert = \"{}\"\nimap.server = \"imaps://{addr}\"\n",
        cert.display(),
    );
    fs::write(&config, toml).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "--json", "envelope", "list"])
        .env("HOME", dir.path())
        .env("XDG_CONFIG_HOME", dir.path())
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let failure: Value = serde_json::from_str(stdout.lines().next().unwrap()).unwrap();
    assert_eq!(output.status.code(), Some(1), "{failure}");
    let path = cert.display().to_string();
    assert!(failure["error"].to_string().contains(&path), "{failure}");
}

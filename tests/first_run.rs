//! # First run
//!
//! The built binary run with and without a configuration, its stderr a
//! pipe, so it answers as a script sees it: the help when a configuration
//! exists, the missing configuration otherwise, never the welcome, which
//! only a person at a terminal meets.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs the binary with `args` in `home`, which holds no configuration of
/// its own.
fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// A home and the path of a configuration file in it holding `toml`, or
/// none when `toml` is `None`.
fn home(toml: Option<&str>) -> (TempDir, PathBuf) {
    let home = TempDir::new().unwrap();
    let path = home.path().join("config.toml");

    if let Some(toml) = toml {
        fs::write(&path, toml).unwrap();
    }

    (home, path)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_bare_invocation_prints_the_help_when_a_configuration_exists() {
    let (home, path) = home(Some("[accounts.a]\ndefault = true\n"));
    let output = run(home.path(), &["-c", path.to_str().unwrap()]);

    assert!(output.status.success(), "{}", text(&output.stdout));
    assert!(text(&output.stdout).contains("Usage: himalaya"));
}

/// A broken file is still a configuration, its error left to the first
/// command reading it.
#[test]
fn a_bare_invocation_prints_the_help_over_a_broken_configuration() {
    let (home, path) = home(Some("[accounts.a\n"));
    let output = run(home.path(), &["-c", path.to_str().unwrap()]);

    assert!(output.status.success(), "{}", text(&output.stdout));
    assert!(text(&output.stdout).contains("Usage: himalaya"));
}

#[test]
fn a_bare_invocation_without_configuration_names_the_missing_path() {
    let (home, path) = home(None);
    let output = run(home.path(), &["-c", path.to_str().unwrap()]);
    let (stdout, stderr) = (text(&output.stdout), text(&output.stderr));

    assert_eq!(output.status.code(), Some(1));
    let missing = format!("No configuration found at {}", path.display());
    assert!(stdout.contains(&missing), "{stdout}");
    assert!(!stdout.contains("Usage:"), "{stdout}");
    assert!(!stderr.contains("Welcome"), "{stderr}");
}

#[test]
fn a_json_caller_gets_the_missing_configuration_as_the_failure_object() {
    let (home, path) = home(None);
    let args = ["--json", "-c", path.to_str().unwrap(), "account", "list"];
    let output = run(home.path(), &args);

    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let error = json["error"].as_str().unwrap();
    assert!(error.contains(&path.display().to_string()), "{error}");
    assert!(error.contains("config.sample.toml"), "{error}");
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
}

/// stdout carries MCP messages only, so a server failing to start says why
/// on stderr.
#[test]
fn an_mcp_server_missing_its_configuration_leaves_stdout_empty() {
    let (home, path) = home(None);
    let output = run(home.path(), &["-c", path.to_str().unwrap(), "mcp"]);
    let stderr = text(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    let missing = format!("No configuration found at {}", path.display());
    assert!(stderr.contains(&missing), "{stderr}");
}

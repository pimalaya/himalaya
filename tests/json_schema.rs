//! # JSON Schema identifiers
//!
//! The built binary writes every schema to a directory: each carries the
//! `$id` of the contract version, the major version of Himalaya.

use std::{
    fs,
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn every_schema_id_names_the_v3_contract() {
    let home = TempDir::new().unwrap();
    let dir = home.path().join("schemas");

    let output = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["--json", "json-schema", "--dir"])
        .arg(&dir)
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");

    let mut count = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let schema: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let id = format!("https://pimalaya.org/schemas/himalaya/v3/{name}.json");

        assert_eq!(schema["$id"], id, "{}", path.display());
        count += 1;
    }

    assert!(count > 1, "{count} schemas written");
}

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

/// The breaking shapes the review of 2026-11-26 settled before the tag
/// (cairn/changes/the-schema-review-remainder, R1 to R5).
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "pimdir",
    feature = "sieve"
))]
mod settled {
    use super::*;

    /// The schemas the built binary writes, by name.
    fn schemas() -> std::collections::BTreeMap<String, Value> {
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

        fs::read_dir(&dir)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
                (
                    name,
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap(),
                )
            })
            .collect()
    }

    /// The type `name` a schema describes, its root or one of its `$defs`.
    fn def<'a>(schema: &'a Value, name: &str) -> &'a Value {
        match schema["title"] == name {
            true => schema,
            false => &schema["$defs"][name],
        }
    }

    /// The subschema at `path` from `node`, each `$ref` on the way resolved
    /// in `schema`.
    fn at<'a>(schema: &'a Value, mut node: &'a Value, path: &[&str]) -> &'a Value {
        let resolve = |mut node: &'a Value| {
            while let Some(reference) = node["$ref"].as_str() {
                node = &schema["$defs"][reference.trim_start_matches("#/$defs/")];
            }
            node
        };

        node = resolve(node);
        for key in path {
            node = resolve(&node[*key]);
        }
        node
    }

    /// Whether a subschema's `type` admits `kind`.
    fn admits(node: &Value, kind: &str) -> bool {
        match &node["type"] {
            Value::String(name) => name == kind,
            Value::Array(names) => names.iter().any(|name| name == kind),
            _ => false,
        }
    }

    /// The type `ty` of the schema `schema` the built binary writes, at
    /// `path`.
    fn field(schema: &str, ty: &str, path: &[&str]) -> Value {
        let schemas = schemas();
        let schema = &schemas[schema];
        at(schema, def(schema, ty), path).clone()
    }

    /// R1 of the schema review of 2026-11-26: `jmap email read` prints the
    /// emails as sent beside `errors`.
    #[cfg(feature = "jmap")]
    #[test]
    fn jmap_email_read_prints_raw_emails_and_errors() {
        let output = field("himalaya-jmap-email-read", "JmapEmailReadOutput", &[]);
        let errors = at(&output, &output, &["properties", "errors"]);
        assert!(admits(errors, "array"), "{output}");
        let email = &output["properties"]["emails"]["items"];
        assert!(email.get("properties").is_none(), "{email}");
        assert_eq!(email["additionalProperties"], true, "{email}");
    }

    /// R2: a `pimdir tag` row's `flags` is `null` when never read.
    #[cfg(feature = "pimdir")]
    #[test]
    fn pimdir_tag_flags_are_nullable() {
        let flags = field(
            "himalaya-pimdir-tag",
            "PimdirTagged",
            &["properties", "flags"],
        );
        assert!(admits(&flags, "array") && admits(&flags, "null"), "{flags}");
    }

    /// R3: `sieve deactivate` prints the `id` of the script active before.
    #[cfg(feature = "sieve")]
    #[test]
    fn sieve_deactivate_prints_the_script_active_before() {
        let output = field("himalaya-sieve-deactivate", "SieveDeactivateOutput", &[]);
        let id = &output["properties"]["id"];
        assert!(admits(id, "string") && admits(id, "null"), "{output}");
        assert!(output["properties"].get("active").is_none(), "{output}");
    }

    /// R4: `pimdir thread` lists the members it cannot read under `errors`.
    #[cfg(feature = "pimdir")]
    #[test]
    fn pimdir_thread_prints_errors() {
        let output = field("himalaya-pimdir-thread", "PimdirThreadOutput", &[]);
        assert!(admits(&output["properties"]["errors"], "array"), "{output}");
    }

    /// R5: `imap thread` entries print their flags as wire strings.
    #[cfg(feature = "imap")]
    #[test]
    fn imap_thread_flags_are_wire_strings() {
        let flag = field(
            "himalaya-imap-thread",
            "ThreadEntry",
            &["properties", "flags", "items"],
        );
        assert!(admits(&flag, "string"), "{flag}");
    }
}

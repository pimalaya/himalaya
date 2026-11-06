//! # MCP
//!
//! `himalaya mcp` run as the built binary against a Maildir, fed JSON-RPC
//! requests on stdin, asserting the tools each gate lists, that a call
//! answers the command's payload, that an unknown argument or a failing
//! command is a tool error the caller reads, that a path given as a
//! message is never read from disk, and that an attachment is read as an
//! embedded resource, bounded by the server, without writing a file.

#![cfg(all(backend, feature = "maildir"))]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use base64::{Engine, prelude::BASE64_STANDARD};
use serde_json::{Value, json};
use tempfile::TempDir;

/// A Maildir account holding one message in its inbox as `m1`, and the
/// configuration reaching it.
fn account(dir: &Path) -> PathBuf {
    let root = dir.join("root");
    for sub in ["cur", "new", "tmp"] {
        fs::create_dir_all(root.join("inbox").join(sub)).unwrap();
    }
    fs::write(
        root.join("inbox/cur/m1:2,S"),
        "From: a@example.org\r\nSubject: Hello there\r\n\r\nbody\r\n",
    )
    .unwrap();

    let config = dir.join("config.toml");
    fs::write(
        &config,
        format!(
            "[accounts.a]\ndefault = true\nemail = \"a@example.org\"\nmaildir.root = \"{}\"\nsmtp.server = \"127.0.0.1:1\"\n",
            root.display(),
        ),
    )
    .unwrap();
    config
}

/// Runs a server with the given flags over `requests`, and returns its
/// responses in order.
fn serve(config: &Path, flags: &[&str], requests: &[Value]) -> Vec<Value> {
    let dir = config.parent().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .args(["-c", config.to_str().unwrap(), "mcp"])
        .args(flags)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env_remove("HIMALAYA_CONFIG")
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    for request in requests {
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );

    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn call(id: u32, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments },
    })
}

fn list(id: u32) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "tools/list" })
}

fn tool<'a>(tools: &'a Value, name: &str) -> Option<&'a Value> {
    tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == name)
}

#[test]
fn reading_tools_only_without_flags() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let responses = serve(
        &config,
        &[],
        &[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            list(2),
        ],
    );

    assert_eq!(responses.len(), 2, "{responses:#?}");
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-06-18");

    let tools = &responses[1];
    assert!(tool(tools, "envelope_list").is_some());
    assert!(tool(tools, "flag_add").is_none());
    assert!(tool(tools, "message_send").is_none());

    let compose = tool(tools, "message_compose").unwrap();
    assert!(compose["inputSchema"]["properties"].get("send").is_none());
    assert!(compose["inputSchema"]["properties"].get("save").is_none());

    let read = tool(tools, "message_read").unwrap();
    assert!(read["inputSchema"]["properties"].get("seen").is_none());
    assert_eq!(read["annotations"]["readOnlyHint"], true);

    for tool in tools["result"]["tools"].as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false, "{tool}");

        if let Some(schema) = tool.get("outputSchema") {
            assert_eq!(schema["type"], "object", "{tool}");
        }
    }
}

#[test]
fn write_and_send_tools_under_their_flags() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let responses = serve(&config, &["--allow-write"], &[list(1)]);

    assert!(tool(&responses[0], "flag_add").is_some());
    assert!(tool(&responses[0], "message_send").is_none());
    assert!(tool(&responses[0], "mailbox_create").is_some());
    assert!(tool(&responses[0], "mailbox_delete").is_some());

    let responses = serve(&config, &["--allow-send"], &[list(1)]);
    let compose = tool(&responses[0], "message_compose").unwrap();

    assert!(tool(&responses[0], "message_send").is_some());
    assert!(tool(&responses[0], "flag_add").is_none());
    // NOTE: a Maildir keeps no auto-reply, so neither tool is listed.
    assert!(tool(&responses[0], "vacation_set").is_none());
    assert!(tool(&responses[0], "vacation_get").is_none());
    assert!(compose["inputSchema"]["properties"].get("send").is_some());
    assert!(compose["inputSchema"]["properties"].get("save").is_none());
}

#[test]
fn a_call_answers_the_payload_and_refuses_what_it_does_not_take() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let responses = serve(
        &config,
        &[],
        &[
            call(
                1,
                "envelope_list",
                json!({ "mailbox": "inbox", "page-size": 10 }),
            ),
            call(2, "envelope_list", json!({ "query": "hello" })),
            call(3, "flag_add", json!({})),
            call(
                4,
                "envelope_search",
                json!({ "query": ["subject", "Hello"] }),
            ),
        ],
    );

    assert_eq!(responses.len(), 4, "{responses:#?}");

    let envelopes = &responses[0]["result"]["structuredContent"]["envelopes"];
    assert_eq!(envelopes[0]["id"], "m1");

    let refused = &responses[1]["result"];
    let text = refused["content"][0]["text"].as_str().unwrap();
    assert_eq!(refused["isError"], true);
    assert!(text.contains("Unknown argument `query`"), "{text}");
    assert!(text.contains("mailbox"), "{text}");

    assert_eq!(responses[2]["error"]["code"], -32602);

    let found = &responses[3]["result"]["structuredContent"]["envelopes"];
    assert_eq!(found[0]["id"], "m1");
}

#[test]
fn a_failing_command_carries_its_code_and_the_server_goes_on() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let responses = serve(
        &config,
        &["--allow-write"],
        &[
            call(1, "message_add", json!({ "mailbox": "inbox" })),
            call(2, "envelope_list", json!({})),
        ],
    );

    let failed = &responses[0]["result"];
    let report: Value =
        serde_json::from_str(failed["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(failed["isError"], true);
    assert_eq!(report["code"], "stdin-taken");

    assert_eq!(
        responses[1]["result"]["structuredContent"]["envelopes"][0]["id"],
        "m1"
    );
}

#[test]
fn a_path_given_as_a_message_is_not_read_from_disk() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let path = config.to_str().unwrap();
    let responses = serve(
        &config,
        &["--allow-write"],
        &[call(
            1,
            "message_add",
            json!({ "mailbox": "inbox", "message": [path] }),
        )],
    );

    let id = responses[0]["result"]["structuredContent"]["id"]
        .as_str()
        .unwrap();
    let cur = dir.path().join("root/inbox/cur");
    let stored = fs::read_dir(&cur)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|entry| entry.file_name().unwrap().to_string_lossy().starts_with(id))
        .unwrap();
    let stored = fs::read_to_string(stored).unwrap();

    assert_eq!(stored, path, "the configuration file was stored instead");
}

/// A configuration whose one account holds `blocks` alone.
fn account_with(dir: &Path, blocks: &str) -> PathBuf {
    let config = dir.join("config.toml");
    let account = format!("[accounts.a]\ndefault = true\nemail = \"a@example.org\"\n{blocks}");
    fs::write(&config, account).unwrap();
    config
}

#[test]
fn a_tool_the_account_cannot_serve_is_not_listed() {
    let flags = ["--allow-write", "--allow-send"];

    let dir = TempDir::new().unwrap();
    let config = account_with(dir.path(), "smtp.server = \"127.0.0.1:1\"\n");
    let tools = &serve(&config, &flags, &[list(1)])[0];

    assert!(tool(tools, "message_send").is_some());
    assert!(tool(tools, "message_compose").is_some());
    assert!(tool(tools, "envelope_list").is_none());
    assert!(tool(tools, "flag_add").is_none());

    #[cfg(feature = "msgraph")]
    {
        let dir = TempDir::new().unwrap();
        let config = account_with(dir.path(), "msgraph.auth.token.command = [\"false\"]\n");
        let tools = &serve(&config, &flags, &[list(1)])[0];

        assert!(tool(tools, "message_add").is_none());
        assert!(tool(tools, "message_copy").is_some());
        assert!(tool(tools, "flag_add").is_some());
    }
}

/// A message whose part `3` is a binary attachment of 7 bytes, `2` its
/// text body.
const ATTACHED: &str = "From: a@example.org\r\n\
    Subject: With a file\r\n\
    Content-Type: multipart/mixed; boundary=\"b\"\r\n\
    \r\n\
    --b\r\n\
    Content-Type: text/plain\r\n\
    \r\n\
    body\r\n\
    --b\r\n\
    Content-Type: application/octet-stream; name=\"blob.bin\"\r\n\
    Content-Transfer-Encoding: base64\r\n\
    \r\n\
    AAH//gAKDQ==\r\n\
    --b--\r\n";

/// The bytes of part `3` of [`ATTACHED`].
const BLOB: &[u8] = b"\x00\x01\xff\xfe\x00\x0a\x0d";

/// The account of [`account`], its inbox holding [`ATTACHED`] as `m2`, and
/// downloads aimed inside `dir`, so a file written lands where a test
/// looks.
fn account_with_attachment(dir: &Path) -> PathBuf {
    let config = account(dir);
    fs::write(dir.join("root/inbox/cur/m2:2,S"), ATTACHED).unwrap();

    let mut toml = fs::read_to_string(&config).unwrap();
    toml.push_str(&format!(
        "downloads-dir = \"{}\"\n",
        dir.join("downloads").display()
    ));
    fs::write(&config, toml).unwrap();

    config
}

/// Every file under `dir` with its bytes, sorted by path.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push((path.clone(), fs::read(path).unwrap()));
            }
        }
    }

    files.sort();
    files
}

#[test]
fn the_download_tool_is_listed_read_only_without_its_disk_arguments() {
    let dir = TempDir::new().unwrap();
    let config = account_with_attachment(dir.path());
    let tools = &serve(&config, &[], &[list(1)])[0];

    let download = tool(tools, "attachment_download").expect("listed without flags");
    let properties = download["inputSchema"]["properties"].as_object().unwrap();

    assert_eq!(download["annotations"]["readOnlyHint"], true);
    assert!(properties.get("stdout").is_none(), "{properties:?}");
    assert!(properties.get("dir").is_none(), "{properties:?}");

    let id = &properties["attachment-id"];
    assert_eq!(id["type"], "array");
    assert_eq!(id["minItems"], 1);
    assert_eq!(id["maxItems"], 1);

    let required = download["inputSchema"]["required"].as_array().unwrap();
    assert!(required.contains(&json!("message-id")), "{required:?}");
    assert!(required.contains(&json!("attachment-id")), "{required:?}");

    let output = &download["outputSchema"];
    assert_eq!(output["type"], "object");
    assert!(output["properties"].get("data").is_none(), "{output}");
    assert!(output["properties"].get("size").is_some(), "{output}");
    assert!(
        !output["required"]
            .as_array()
            .unwrap()
            .contains(&json!("data")),
        "{output}"
    );

    // NOTE: the cap is the server's, so no tool takes it.
    for tool in tools["result"]["tools"].as_array().unwrap() {
        let properties = tool["inputSchema"]["properties"].as_object().unwrap();
        assert!(properties.get("max-attachment-size").is_none(), "{tool}");
    }
}

#[test]
fn an_attachment_is_read_as_an_embedded_resource_and_nothing_is_written() {
    let dir = TempDir::new().unwrap();
    let config = account_with_attachment(dir.path());
    let before = snapshot(dir.path());

    let responses = serve(
        &config,
        &[],
        &[call(
            1,
            "attachment_download",
            json!({ "message-id": "m2", "attachment-id": ["3"] }),
        )],
    );

    // NOTE: checked first, so it is what fails with the fixed `--stdout`
    // removed, the command then writing the part to `downloads-dir`.
    assert_eq!(
        snapshot(dir.path()),
        before,
        "a file was written or changed"
    );

    let result = &responses[0]["result"];
    assert!(result.get("isError").is_none(), "{result}");

    let part = &result["structuredContent"];
    assert_eq!(
        part,
        &json!({
            "id": "3",
            "mime": "application/octet-stream",
            "filename": "blob.bin",
            "size": 7,
        })
    );

    let content = result["content"].as_array().unwrap();
    assert_eq!(content.len(), 2, "{content:#?}");

    let text: Value = serde_json::from_str(content[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(content[0]["type"], "text");
    assert_eq!(&text, part, "the text block is the JSON without data");

    let embedded = &content[1];
    assert_eq!(embedded["type"], "resource");
    assert_eq!(embedded["resource"]["uri"], "himalaya://a/inbox/m2/3");
    assert_eq!(embedded["resource"]["mimeType"], "application/octet-stream");

    let blob = embedded["resource"]["blob"].as_str().unwrap();
    assert_eq!(BASE64_STANDARD.decode(blob).unwrap(), BLOB);

    // NOTE: the bytes travel once, never also as `data`.
    assert!(!result.to_string().contains("\"data\""), "{result}");
}

#[test]
fn the_part_uri_names_the_mailbox_as_the_call_does() {
    let dir = TempDir::new().unwrap();
    let config = account_with_attachment(dir.path());
    let responses = serve(
        &config,
        &[],
        &[call(
            1,
            "attachment_download",
            json!({ "mailbox": "inbox", "message-id": "m2", "attachment-id": ["2"] }),
        )],
    );

    let resource = &responses[0]["result"]["content"][1]["resource"];
    assert_eq!(resource["uri"], "himalaya://a/inbox/m2/2");
    assert_eq!(resource["mimeType"], "text/plain");
    assert_eq!(
        BASE64_STANDARD
            .decode(resource["blob"].as_str().unwrap())
            .unwrap(),
        b"body"
    );
}

#[test]
fn the_download_tool_takes_exactly_one_part_id() {
    let dir = TempDir::new().unwrap();
    let config = account_with_attachment(dir.path());
    let before = snapshot(dir.path());

    let responses = serve(
        &config,
        &[],
        &[
            call(
                1,
                "attachment_download",
                json!({ "message-id": "m2", "attachment-id": ["2", "3"] }),
            ),
            call(2, "attachment_download", json!({ "message-id": "m2" })),
            call(
                3,
                "attachment_download",
                json!({ "message-id": "m2", "attachment-id": ["3"], "stdout": false }),
            ),
        ],
    );

    for refused in &responses[..2] {
        let text = refused["result"]["content"][0]["text"].as_str().unwrap();
        assert_eq!(refused["result"]["isError"], true, "{refused}");
        assert!(text.contains("exactly one attachment id"), "{text}");
    }

    let text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert_eq!(responses[2]["result"]["isError"], true);
    assert!(text.contains("Unknown argument `stdout`"), "{text}");

    assert_eq!(
        snapshot(dir.path()),
        before,
        "a file was written or changed"
    );
}

#[test]
fn a_part_over_the_server_cap_is_refused() {
    let dir = TempDir::new().unwrap();
    let config = account_with_attachment(dir.path());
    let read = |id: u32| {
        call(
            id,
            "attachment_download",
            json!({ "message-id": "m2", "attachment-id": ["3"] }),
        )
    };

    let responses = serve(&config, &["--max-attachment-size", "6"], &[read(1)]);
    let refused = &responses[0]["result"];
    let report: Value =
        serde_json::from_str(refused["content"][0]["text"].as_str().unwrap()).unwrap();

    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(report["code"], "attachment-too-large", "{report}");
    assert!(refused.get("structuredContent").is_none(), "{refused}");
    assert_eq!(refused["content"].as_array().unwrap().len(), 1);

    // NOTE: the cap bounds the decoded size, a part of exactly that size
    // passing.
    let responses = serve(&config, &["--max-attachment-size=7"], &[read(1)]);
    assert_eq!(responses[0]["result"]["structuredContent"]["size"], 7);
}

#[test]
fn the_search_tool_says_what_the_backend_answers() {
    let dir = TempDir::new().unwrap();
    let config = account(dir.path());
    let tools = &serve(&config, &[], &[list(1)])[0];
    let description = tool(tools, "envelope_search").unwrap()["description"]
        .as_str()
        .unwrap()
        .to_string();

    assert!(
        description.contains("(maildir) answers fully: date, after, before"),
        "{description}"
    );
}

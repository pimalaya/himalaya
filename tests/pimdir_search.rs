//! # pimdir search
//!
//! `pimdir search` run as the built binary over a store io-pimdir filled
//! the way a sync engine does, its index refreshed as the store's owner
//! refreshes it: a page names the next by `nextPage`, which `--page-token`
//! takes back, and a message the store cannot read is an item error.

#![cfg(feature = "pimdir")]

use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

use io_pimdir::{
    change::PimdirWriteOp,
    client::{
        PimdirStore,
        search::{PimdirIndex, PimdirIndexRefreshOptions},
    },
    collection::PimdirCollectionId,
    hash,
    object::PimdirObject,
    placement::{
        PimdirBase, PimdirFlags, PimdirHandle, PimdirLevel, PimdirPlacement, PimdirStatus,
    },
    summary::{PimdirDerivation, mail},
};
use serde_json::Value;
use tempfile::TempDir;

const INBOX: &str = "imap/INBOX";

/// A store whose `imap/INBOX` holds three messages about a report, dated
/// the 1st, 2nd and 3rd of October 2026, their bodies stored and indexed.
fn store(root: &Path) {
    let mut store = PimdirStore::open(root)
        .unwrap()
        .for_account("work")
        .for_source("imap")
        .unwrap();
    store.ensure_collection(INBOX, "message/rfc822").unwrap();

    for day in 1..=3 {
        let body = format!(
            "Message-ID: <{day}@x.test>\r\nFrom: a@x.test\r\n\
             Subject: Report {day}\r\nDate: {day:02} Oct 2026 09:00:00 +0000\r\n\
             \r\nThe report.\r\n"
        )
        .into_bytes();
        let hash = hash::hash(&body);
        let PimdirDerivation {
            link_id,
            summary,
            sort_key,
        } = mail::derive(&body);

        store
            .write(vec![
                PimdirWriteOp::StoreObject {
                    object: PimdirObject {
                        hash: hash.clone(),
                        size: body.len(),
                    },
                    body: Some(body),
                },
                PimdirWriteOp::UpsertPlacement(PimdirPlacement {
                    collection: PimdirCollectionId(INBOX.into()),
                    handle: PimdirHandle(day.to_string()),
                    link_id: Some(link_id),
                    level: PimdirLevel::Full,
                    object: Some(hash.clone()),
                    summary,
                    server_object_id: None,
                    sort_key,
                    flags: PimdirFlags::default(),
                    status: PimdirStatus::Clean,
                    conflict_revision: None,
                    conflict_object: None,
                    base: Some(PimdirBase {
                        flags: PimdirFlags::default(),
                        revision: None,
                        object: Some(hash),
                    }),
                    origin: None,
                }),
            ])
            .unwrap();
    }
    drop(store);

    PimdirIndex::open(root)
        .unwrap()
        .refresh(&PimdirIndexRefreshOptions::default())
        .unwrap();
}

/// Runs `<args>` against one pimdir account over the store at `root`.
fn himalaya(home: &Path, root: &Path, args: &[&str]) -> Output {
    let config = home.join("config.toml");
    let account = format!(
        "[accounts.work]\ndefault = true\npimdir.root = {:?}\n",
        root.to_str().unwrap()
    );
    fs::write(&config, account).unwrap();

    Command::new(env!("CARGO_BIN_EXE_himalaya"))
        .arg("-c")
        .arg(&config)
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("XDG_CONFIG_DIRS")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// The JSON a run printed.
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// The subjects of a page's hits, in order.
fn subjects(page: &Value) -> Vec<&str> {
    page["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["subject"].as_str().unwrap())
        .collect()
}

#[test]
fn a_search_pages_by_next_page_and_page_token() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("store");
    fs::create_dir(&root).unwrap();
    store(&root);
    let run = |args: &[&str]| himalaya(home.path(), &root, args);

    let first = run(&["--json", "pimdir", "search", "-s", "2", "report"]);
    assert!(first.status.success());
    let first = json(&first);
    assert_eq!(subjects(&first), ["Report 3", "Report 2"], "{first}");
    let token = first["nextPage"].as_str().unwrap().to_owned();
    assert_eq!(first["errors"], Value::Array(Vec::new()), "{first}");
    assert!(first.get("more").is_none(), "{first}");

    let args = [
        "--json",
        "pimdir",
        "search",
        "-s",
        "2",
        "--page-token",
        &token,
    ];
    let last = run(&[&args[..], &["report"]].concat());
    assert!(last.status.success());
    let last = json(&last);
    assert_eq!(subjects(&last), ["Report 1"], "{last}");
    assert_eq!(last["nextPage"], Value::Null, "{last}");

    let full = run(&["--json", "pimdir", "search", "-s", "3", "report"]);
    assert_eq!(json(&full)["nextPage"], Value::Null);

    let threads = run(&[
        "--json",
        "pimdir",
        "search",
        "--threads",
        "-s",
        "2",
        "report",
    ]);
    assert!(threads.status.success());
    let threads = json(&threads);
    assert_eq!(threads["threads"].as_array().unwrap().len(), 2, "{threads}");
    assert!(threads["nextPage"].is_string(), "{threads}");
    assert!(threads.get("more").is_none(), "{threads}");

    let text = run(&["pimdir", "search", "-s", "2", "report"]);
    let text = String::from_utf8_lossy(&text.stdout);
    let next = |line: &str| line == format!("Next page: --page-token {token}");
    assert!(text.lines().any(next), "{text}");

    for token in ["0", "2", "two"] {
        let refused = run(&[
            "--json",
            "pimdir",
            "search",
            "--page-token",
            token,
            "report",
        ]);
        assert_eq!(refused.status.code(), Some(1), "{token}");
        let error = json(&refused)["error"].as_str().unwrap().to_owned();
        assert!(error.contains("Invalid page token"), "{error}");
    }
}

#[test]
fn a_message_the_store_cannot_read_is_an_item_error() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("store");
    fs::create_dir(&root).unwrap();
    store(&root);

    let conn = rusqlite::Connection::open(root.join("pimdir.db")).unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    let sql = "UPDATE summary SET size = 'x' \
               WHERE component = 'mail' AND title = 'Report 2'";
    assert_eq!(conn.execute(sql, []).unwrap(), 1);
    drop(conn);

    let output = himalaya(
        home.path(),
        &root,
        &["--json", "pimdir", "search", "report"],
    );
    let page = json(&output);
    assert_eq!(output.status.code(), Some(3), "{page}");
    assert_eq!(subjects(&page), ["Report 3", "Report 1"], "{page}");

    let errors = page["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1, "{page}");
    assert_eq!(errors[0]["code"], "message-unreadable", "{page}");
    assert!(errors[0]["id"].is_string(), "{page}");
}

/// Options after the query are read as options, `--json` included, where
/// they once became query words matching nothing.
#[test]
fn an_option_after_the_query_is_the_option() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("store");
    fs::create_dir(&root).unwrap();
    store(&root);
    let run = |args: &[&str]| himalaya(home.path(), &root, args);

    let page = run(&["pimdir", "search", "report", "-s", "2", "--json"]);
    assert!(page.status.success());
    let page = json(&page);
    assert_eq!(subjects(&page), ["Report 3", "Report 2"], "{page}");
    assert!(page["nextPage"].is_string(), "{page}");

    let count = json(&run(&["pimdir", "count", "report", "--json"]));
    assert_eq!(count["count"], 3, "{count}");

    let negated = run(&["--json", "pimdir", "count", "report", "--", "-subject:3"]);
    assert_eq!(json(&negated)["count"], 2);
}

/// A page token reads back under the query, order and listing that
/// printed it alone: under another it is refused, coded
/// `page-token-invalid`, where a page number paged any query.
#[test]
fn a_token_of_another_query_is_refused() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("store");
    fs::create_dir(&root).unwrap();
    store(&root);
    let run = |args: &[&str]| himalaya(home.path(), &root, args);

    let first = json(&run(&["--json", "pimdir", "search", "-s", "2", "report"]));
    let token = first["nextPage"].as_str().unwrap().to_owned();

    let others: [&[&str]; 3] = [
        &["from:a@x.test"],
        &["--sort", "title", "report"],
        &["--threads", "report"],
    ];

    for other in others {
        let args = [
            "--json",
            "pimdir",
            "search",
            "-s",
            "2",
            "--page-token",
            &token,
        ];
        let refused = run(&[&args[..], other].concat());
        assert_eq!(refused.status.code(), Some(1), "{other:?}");
        assert_eq!(json(&refused)["code"], "page-token-invalid", "{other:?}");
    }
}

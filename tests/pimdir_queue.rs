//! # pimdir queue
//!
//! The writes of a pimdir account run as the built binary over a store
//! io-pimdir fills the way a sync engine does: each answers a receipt
//! naming its item by `id`, a send only queued says it was not sent, and
//! `pimdir queue list` lists every pending row whatever its kind, which
//! `pimdir queue show` prints as the list does, then where it stands.

#![cfg(feature = "pimdir")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use io_pimdir::{
    change::PimdirWriteOp,
    client::PimdirStore,
    collection::{PimdirCheckpoint, PimdirCollectionId},
    hash,
    object::PimdirObject,
    placement::{
        PimdirBase, PimdirFlags, PimdirHandle, PimdirLevel, PimdirPlacement, PimdirStatus,
    },
    summary::mail,
};
use serde_json::Value;
use tempfile::TempDir;

const MAIL: &str = "message/rfc822";

const STORED: &[u8] = b"Message-ID: <stored@x.test>\r\nFrom: a@x.test\r\n\
                        Subject: Stored\r\n\r\nbody\r\n";

/// A store holding, for the account `work`, `imap/INBOX` with one message
/// whose body is stored, `imap/Archive` and `imap/Sent`, which a sync
/// engine source reached.
fn store(root: &Path) {
    let mut store = PimdirStore::open(root)
        .unwrap()
        .for_account("work")
        .for_source("imap");
    for collection in ["imap/INBOX", "imap/Archive", "imap/Sent"] {
        store.ensure_collection(collection, MAIL).unwrap();
    }

    let hash = hash::hash(STORED);
    let derived = mail::derive(STORED);
    let placement = PimdirPlacement {
        collection: PimdirCollectionId("imap/INBOX".into()),
        handle: PimdirHandle("1".into()),
        link_id: Some(derived.link_id),
        level: PimdirLevel::Full,
        object: Some(hash.clone()),
        summary: derived.summary,
        sort_key: derived.sort_key,
        flags: PimdirFlags::default(),
        status: PimdirStatus::Clean,
        conflict_revision: None,
        conflict_object: None,
        base: Some(PimdirBase {
            flags: PimdirFlags::default(),
            revision: None,
            object: Some(hash.clone()),
        }),
        origin: None,
    };

    store
        .write(vec![
            PimdirWriteOp::StoreObject {
                object: PimdirObject {
                    hash,
                    size: STORED.len(),
                },
                body: Some(STORED.to_vec()),
            },
            PimdirWriteOp::UpsertPlacement(placement),
            PimdirWriteOp::SetCheckpoint {
                collection: PimdirCollectionId("imap/Sent".into()),
                checkpoint: PimdirCheckpoint(Vec::new()),
            },
        ])
        .unwrap();
}

/// The binary, configured with one pimdir account over its store.
struct Himalaya {
    home: TempDir,
    config: PathBuf,
}

impl Himalaya {
    fn new() -> Self {
        let home = TempDir::new().unwrap();
        let root = home.path().join("store");
        fs::create_dir(&root).unwrap();
        store(&root);

        let config = home.path().join("config.toml");
        let lines = format!(
            "[accounts.work]\ndefault = true\nemail = \"a@x.test\"\n\
             mailbox.alias.sent = \"imap/Sent\"\npimdir.root = {:?}\n",
            root.to_str().unwrap()
        );
        fs::write(&config, lines).unwrap();

        Self { home, config }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_himalaya"))
            .arg("-c")
            .arg(&self.config)
            .arg("--json")
            .args(args)
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path())
            .env_remove("XDG_CONFIG_DIRS")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    /// What a run printed, once it exited 0.
    fn json(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {stdout}{stderr}");
        serde_json::from_str(&stdout).unwrap()
    }
}

fn message(subject: &str) -> String {
    format!("From: a@x.test\r\nTo: b@x.test\r\nSubject: {subject}\r\n\r\nhi\r\n")
}

/// Every write answers its receipt, `id` naming the item; the queue then
/// lists a row of each kind under `actions`, the parked rows and the
/// rows it could not read apart, and a row shows as it lists.
#[test]
fn every_pending_row_lists_and_shows_as_it_lists() {
    let himalaya = Himalaya::new();

    let list = himalaya.json(&["envelope", "list", "-m", "imap/INBOX"]);
    let stored = list["envelopes"][0]["id"].as_str().unwrap().to_owned();

    let added = himalaya.json(&["message", "add", "-m", "imap/INBOX", &message("Added")]);
    assert!(added["id"].is_string(), "{added}");
    assert_eq!(added["saved"], true, "{added}");
    assert_eq!(added["sent"], false, "{added}");
    assert_eq!(added["queued"], true, "{added}");

    let staged = himalaya.json(&[
        "pimdir",
        "message",
        "add",
        "-m",
        "imap/INBOX",
        &message("Staged"),
    ]);
    assert!(staged["id"].is_string(), "{staged}");
    assert!(staged.get("messageId").is_none(), "{staged}");

    let sent = himalaya.json(&["message", "send", "--no-save", &message("Sent")]);
    assert_eq!(sent["sent"], false, "{sent}");
    assert_eq!(sent["queued"], true, "{sent}");

    himalaya.json(&["flag", "add", "-m", "imap/INBOX", "-f", "flagged", &stored]);
    let args = ["message", "move", "-f", "imap/INBOX", "-t", "imap/Archive"];
    himalaya.json(&[&args[..], &[&stored]].concat());

    let queue = himalaya.json(&["pimdir", "queue", "list"]);
    assert_eq!(queue["parked"], Value::Array(Vec::new()), "{queue}");
    assert_eq!(queue["errors"], Value::Array(Vec::new()), "{queue}");

    let actions = queue["actions"].as_array().unwrap();
    let kinds: Vec<&str> = actions
        .iter()
        .map(|a| a["kind"].as_str().unwrap())
        .collect();
    for kind in ["add", "submit", "set-flags", "move"] {
        assert!(kinds.contains(&kind), "{kind}: {queue}");
    }

    let link_ids: Vec<&Value> = actions.iter().map(|a| &a["linkId"]).collect();
    assert!(link_ids.contains(&&added["id"]), "{queue}");
    assert!(link_ids.contains(&&staged["id"]), "{queue}");

    let flagged = actions.iter().find(|a| a["kind"] == "set-flags").unwrap();
    assert_eq!(flagged["itemId"], stored.as_str(), "{queue}");

    let handle = flagged["handle"].as_str().unwrap();
    let shown = himalaya.json(&["pimdir", "queue", "show", handle]);
    for (key, value) in flagged.as_object().unwrap() {
        assert_eq!(&shown[key], value, "{key}: {shown}");
    }
    assert_eq!(shown["state"], "pending", "{shown}");
}

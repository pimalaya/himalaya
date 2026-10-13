//! # pimdir attachments
//!
//! `pimdir attachment list` and `search` run as the built binary over a
//! store io-pimdir filled the way a sync engine does: each message body
//! entering it records its attachments' stand-ins. The ids printed are
//! checked against `envelope list`, and a file's bytes read back through
//! `message read --raw` and `message parse --section`.

#![cfg(feature = "pimdir")]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use io_pimdir::{
    change::PimdirWriteOp,
    client::PimdirStore,
    collection::PimdirCollectionId,
    hash::PimdirHashAlgo,
    object::{PimdirHash, PimdirObject},
    placement::{
        PimdirBase, PimdirFlags, PimdirHandle, PimdirLevel, PimdirPlacement, PimdirStatus,
    },
    summary::{PimdirDerivation, mail},
};
use serde_json::{Value, json};
use tempfile::TempDir;

const MAIL: &str = "message/rfc822";
const ACCOUNT: &str = "work";

/// A message from `from` dated `day` (of October 2026) attaching
/// `files`, each a name, a media type and base64 content.
fn message(id: &str, from: &str, day: u32, files: &[(&str, &str, &str)]) -> Vec<u8> {
    let mut raw = format!(
        "Message-ID: <{id}>\r\n\
         From: {from}\r\n\
         To: bob@example.org\r\n\
         Subject: {id}\r\n\
         Date: {day:02} Oct 2026 09:00:00 +0000\r\n\
         Content-Type: multipart/mixed; boundary=\"b\"\r\n\
         \r\n\
         --b\r\n\
         Content-Type: multipart/alternative; boundary=\"a\"\r\n\
         \r\n\
         --a\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         See attached.\r\n\
         --a\r\n\
         Content-Type: text/html\r\n\
         \r\n\
         <p>See attached.</p>\r\n\
         --a--\r\n"
    );
    for (name, media_type, data) in files {
        raw.push_str(&format!(
            "--b\r\n\
             Content-Type: {media_type}\r\n\
             Content-Disposition: attachment; filename=\"{name}\"\r\n\
             Content-Transfer-Encoding: base64\r\n\
             \r\n\
             {data}\r\n"
        ));
    }
    raw.push_str("--b--\r\n");
    raw.into_bytes()
}

/// A store holding, for the account `work`, the four messages below in
/// `imap/INBOX` and `imap/Archive`, their bodies stored as a sync engine
/// stores them, and one more listed with no body.
fn store(dir: &Path) {
    let mut store = PimdirStore::open(dir)
        .unwrap()
        .for_account(ACCOUNT)
        .for_source("imap");
    store.ensure_collection("imap/INBOX", MAIL).unwrap();
    store.ensure_collection("imap/Archive", MAIL).unwrap();

    let bodies = [
        (
            "imap/Archive",
            "1",
            message(
                "old@shop.test",
                "Shop <billing@shop.test>",
                1,
                &[("Facture-Septembre.pdf", "application/pdf", "JVBERi0xLjQK")],
            ),
        ),
        (
            "imap/INBOX",
            "2",
            message(
                "photos@x.test",
                "=?utf-8?Q?=C3=89lise_Martin?= <elise@x.test>",
                3,
                &[
                    ("plage.jpg", "image/jpeg", "/9j/4AAQ"),
                    ("dune.png", "image/png", "iVBORw0KGgo="),
                ],
            ),
        ),
        (
            "imap/INBOX",
            "3",
            message("plain@x.test", "Bob <bob@x.test>", 4, &[]),
        ),
        (
            "imap/INBOX",
            "4",
            message(
                "new@shop.test",
                "Shop <billing@shop.test>",
                5,
                &[("Facture-Octobre.pdf", "application/pdf", "JVBERi0xLjcK")],
            ),
        ),
    ];

    for (collection, handle, body) in &bodies {
        let hash = PimdirHashAlgo::default().hash(body);
        let derived = mail::derive(body);
        store
            .write(vec![
                PimdirWriteOp::StoreObject {
                    object: PimdirObject {
                        hash: hash.clone(),
                        size: body.len(),
                    },
                    body: Some(body.clone()),
                },
                PimdirWriteOp::UpsertPlacement(placement(collection, handle, derived, Some(hash))),
            ])
            .unwrap();
    }

    // NOTE: listed with no body: it attaches nothing until one is stored.
    let unfetched = message(
        "later@shop.test",
        "Shop <billing@shop.test>",
        6,
        &[("Facture-Novembre.pdf", "application/pdf", "JVBERi0xLjcK")],
    );
    let derived = mail::derive(&unfetched);
    store
        .write(vec![PimdirWriteOp::UpsertPlacement(placement(
            "imap/INBOX",
            "5",
            derived,
            None,
        ))])
        .unwrap();
}

/// A clean placement of a derived message, holding `object` as its body
/// when there is one.
fn placement(
    collection: &str,
    handle: &str,
    derived: PimdirDerivation,
    object: Option<PimdirHash>,
) -> PimdirPlacement {
    let PimdirDerivation {
        link_id,
        summary,
        sort_key,
    } = derived;
    PimdirPlacement {
        collection: PimdirCollectionId(collection.into()),
        handle: PimdirHandle(handle.into()),
        link_id: Some(link_id),
        level: match object {
            Some(_) => PimdirLevel::Full,
            None => PimdirLevel::Meta,
        },
        object: object.clone(),
        summary,
        sort_key,
        flags: PimdirFlags::default(),
        status: PimdirStatus::Clean,
        conflict_revision: None,
        conflict_object: None,
        base: Some(PimdirBase {
            flags: PimdirFlags::default(),
            revision: None,
            object,
        }),
        origin: None,
    }
}

/// The binary, configured with one pimdir account over `store`.
struct Himalaya {
    home: TempDir,
    config: PathBuf,
}

impl Himalaya {
    fn new(fill: impl FnOnce(&Path)) -> Self {
        let home = TempDir::new().unwrap();
        let root = home.path().join("store");
        fs::create_dir(&root).unwrap();
        fill(&root);
        let config = home.path().join("config.toml");
        fs::write(
            &config,
            format!(
                "[accounts.work]\ndefault = true\npimdir.root = {:?}\n",
                root.to_str().unwrap()
            ),
        )
        .unwrap();
        Self { home, config }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_himalaya"));
        cmd.arg("-c")
            .arg(&self.config)
            .args(args)
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path())
            .env_remove("HIMALAYA_CONFIG")
            .env_remove("XDG_CONFIG_DIRS")
            .stdin(Stdio::null());
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let mut all = vec!["--json"];
        all.extend_from_slice(args);
        serde_json::from_slice(&ok(self.run(&all))).unwrap()
    }
}

fn ok(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output.stdout
}

/// The names of a page's files, in order.
fn names(page: &Value) -> Vec<&str> {
    page["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["name"].as_str().unwrap())
        .collect()
}

#[test]
fn every_file_lists_newest_message_first_with_its_message() {
    let himalaya = Himalaya::new(store);

    let page = himalaya.json(&["pimdir", "attachment", "list"]);
    assert_eq!(
        names(&page),
        [
            "Facture-Octobre.pdf",
            "dune.png",
            "plage.jpg",
            "Facture-Septembre.pdf"
        ]
    );
    assert_eq!(page["nextPage"], Value::Null);

    let first = &page["attachments"][0];
    let id = first["id"].as_str().unwrap();
    let message_id = first["messageId"].as_str().unwrap();
    assert!(id.parse::<i64>().unwrap() > 0, "{id}");
    assert_eq!(
        *first,
        json!({
            "id": id,
            "name": "Facture-Octobre.pdf",
            "mediaType": "application/pdf",
            "size": 9,
            "section": "2",
            "messageId": message_id,
            "mailbox": "imap/INBOX",
            "sender": "billing@shop.test",
            "senderName": "Shop",
            "date": "2026-10-05T09:00:00Z",
        })
    );
    assert_eq!(page["attachments"][1]["senderName"], "\u{c9}lise Martin");
    assert_eq!(page["attachments"][1]["section"], "3");
    assert_eq!(page["attachments"][2]["section"], "2");
    assert_eq!(page["attachments"][3]["mailbox"], "imap/Archive");

    let table = String::from_utf8(ok(himalaya.run(&["pimdir", "attachment", "list"]))).unwrap();
    assert!(table.contains("Facture-Octobre.pdf"), "{table}");
    assert!(!table.contains("Next page"), "{table}");
}

#[test]
fn a_file_s_message_is_the_one_envelope_list_shows() {
    let himalaya = Himalaya::new(store);
    let page = himalaya.json(&["pimdir", "attachment", "list"]);

    for file in page["attachments"].as_array().unwrap() {
        let mailbox = file["mailbox"].as_str().unwrap();
        let envelopes = himalaya.json(&["envelope", "list", "-m", mailbox]);
        let envelope = envelopes["envelopes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|envelope| envelope["id"] == file["messageId"])
            .unwrap_or_else(|| panic!("{file} not in {envelopes}"));
        assert_eq!(envelope["from"][0]["email"], file["sender"], "{envelope}");
    }
}

#[test]
fn a_file_s_bytes_are_its_message_s_section() {
    let himalaya = Himalaya::new(store);
    let page = himalaya.json(&["pimdir", "attachment", "list"]);
    let file = &page["attachments"][1];
    assert_eq!(file["name"], "dune.png");

    let raw = ok(himalaya.run(&[
        "message",
        "read",
        "--raw",
        "-m",
        file["mailbox"].as_str().unwrap(),
        file["messageId"].as_str().unwrap(),
    ]));

    let mut parse = himalaya
        .command(&[
            "--json",
            "message",
            "parse",
            "--section",
            file["section"].as_str().unwrap(),
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    parse.stdin.take().unwrap().write_all(&raw).unwrap();
    let part: Value = serde_json::from_slice(&ok(parse.wait_with_output().unwrap())).unwrap();
    assert_eq!(part["filename"], "dune.png");
    assert_eq!(part["mime"], "image/png");
    assert_eq!(part["data"], "iVBORw0KGgo=");
    assert_eq!(part["size"], file["size"]);
}

#[test]
fn pages_follow_one_another_by_their_token() {
    let himalaya = Himalaya::new(store);

    let mut seen = Vec::new();
    let mut token: Option<String> = None;
    let mut pages = 0;
    loop {
        let mut args = vec!["pimdir", "attachment", "list", "-s", "3"];
        if let Some(token) = &token {
            args.extend(["--page-token", token.as_str()]);
        }
        let page = himalaya.json(&args);
        seen.extend(names(&page).into_iter().map(String::from));
        pages += 1;
        match page["nextPage"].as_str() {
            Some(next) => token = Some(next.to_owned()),
            None => break,
        }
    }

    assert_eq!(pages, 2);
    assert_eq!(
        seen,
        [
            "Facture-Octobre.pdf",
            "dune.png",
            "plage.jpg",
            "Facture-Septembre.pdf"
        ]
    );

    // NOTE: a page exactly full of the last files names no next page.
    let page = himalaya.json(&["pimdir", "attachment", "list", "-s", "4"]);
    assert_eq!(page["nextPage"], Value::Null);

    let table = String::from_utf8(ok(himalaya.run(&[
        "pimdir",
        "attachment",
        "list",
        "-s",
        "1",
    ])))
    .unwrap();
    assert!(table.contains("Next page: "), "{table}");

    let output = himalaya.run(&[
        "--json",
        "pimdir",
        "attachment",
        "list",
        "--page-token",
        "nope",
    ]);
    assert!(!output.status.success());
    let err: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        err["error"]
            .as_str()
            .unwrap()
            .contains("Invalid page token"),
        "{err}"
    );
}

#[test]
fn a_search_finds_files_by_name_and_by_sender() {
    let himalaya = Himalaya::new(store);
    let search = |words: &[&str]| {
        let mut args = vec!["pimdir", "attachment", "search"];
        args.extend_from_slice(words);
        himalaya.json(&args)
    };

    assert_eq!(
        names(&search(&["facture"])),
        ["Facture-Octobre.pdf", "Facture-Septembre.pdf"]
    );
    assert_eq!(
        names(&search(&["billing@shop"])),
        ["Facture-Octobre.pdf", "Facture-Septembre.pdf"]
    );
    assert_eq!(names(&search(&["\u{e9}lise"])), ["dune.png", "plage.jpg"]);
    assert_eq!(names(&search(&["MARTIN", "plage"])), ["plage.jpg"]);
    assert_eq!(names(&search(&["octobre", "elise"])), Vec::<&str>::new());

    let first = search(&["facture", "-s", "1"]);
    assert_eq!(names(&first), ["Facture-Octobre.pdf"]);
    let token = first["nextPage"].as_str().unwrap();
    let second = search(&["facture", "-s", "1", "--page-token", token]);
    assert_eq!(names(&second), ["Facture-Septembre.pdf"]);
    assert_eq!(second["nextPage"], Value::Null);
}

#[test]
fn an_account_holding_no_attachments_lists_none() {
    // NOTE: mail listed with no body: no attachments collection at all.
    let himalaya = Himalaya::new(|dir| {
        let mut store = PimdirStore::open(dir)
            .unwrap()
            .for_account(ACCOUNT)
            .for_source("imap");
        store.ensure_collection("imap/INBOX", MAIL).unwrap();
        let derived = mail::derive(&message(
            "a@x.test",
            "a@x.test",
            1,
            &[("a.pdf", "application/pdf", "JVBERi0xLjQK")],
        ));
        store
            .write(vec![PimdirWriteOp::UpsertPlacement(placement(
                "imap/INBOX",
                "1",
                derived,
                None,
            ))])
            .unwrap();
    });

    for args in [
        &["pimdir", "attachment", "list"][..],
        &["pimdir", "attachment", "search", "a"][..],
    ] {
        assert_eq!(
            himalaya.json(args),
            json!({ "attachments": [], "nextPage": null })
        );
    }
}

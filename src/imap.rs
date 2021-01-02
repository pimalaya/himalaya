use imap;
use native_tls::{TlsConnector, TlsStream};
use rfc2047_decoder;
use std::net::TcpStream;

use crate::config::{Config, ServerInfo};
use crate::table;

type ImapClient = imap::Client<TlsStream<TcpStream>>;
type ImapSession = imap::Session<TlsStream<TcpStream>>;

pub fn create_tls_connector() -> TlsConnector {
    match native_tls::TlsConnector::new() {
        Ok(connector) => connector,
        Err(err) => {
            println!("The TLS connector could not be created.");
            panic!(err);
        }
    }
}

pub fn create_imap_client(server: &ServerInfo, tls: &TlsConnector) -> ImapClient {
    match imap::connect(server.get_addr(), server.get_host(), &tls) {
        Ok(client) => client,
        Err(err) => {
            println!("The IMAP socket could not be opened.");
            panic!(err);
        }
    }
}

pub fn create_imap_sess(client: ImapClient, server: &ServerInfo) -> ImapSession {
    match client.login(server.get_login(), server.get_password()) {
        Ok(sess) => sess,
        Err((err, _)) => {
            println!("The IMAP connection could not be established.");
            panic!(err);
        }
    }
}

pub fn login(config: &Config) -> ImapSession {
    let tls = create_tls_connector();
    let client = create_imap_client(&config.imap, &tls);
    let imap_sess = create_imap_sess(client, &config.imap);
    imap_sess
}

fn subject_from_fetch(fetch: &imap::types::Fetch) -> String {
    let envelope = fetch.envelope().expect("envelope is missing");

    match &envelope.subject {
        None => String::new(),
        Some(bytes) => match rfc2047_decoder::decode(bytes) {
            Err(_) => String::new(),
            Ok(subject) => subject,
        },
    }
}

fn first_addr_from_fetch(fetch: &imap::types::Fetch) -> String {
    let envelope = fetch.envelope().expect("envelope is missing");

    match &envelope.from {
        None => String::new(),
        Some(addresses) => match addresses.first() {
            None => String::new(),
            Some(address) => {
                let mbox = String::from_utf8(address.mailbox.expect("invalid addr mbox").to_vec())
                    .expect("invalid addr mbox");
                let host = String::from_utf8(address.host.expect("invalid addr host").to_vec())
                    .expect("invalid addr host");
                let email = format!("{}@{}", mbox, host);

                match address.name {
                    None => email,
                    Some(name) => match rfc2047_decoder::decode(name) {
                        Err(_) => email,
                        Ok(name) => name,
                    },
                }
            }
        },
    }
}

fn date_from_fetch(fetch: &imap::types::Fetch) -> String {
    let envelope = fetch.envelope().expect("envelope is missing");

    match &envelope.date {
        None => String::new(),
        Some(date) => match String::from_utf8(date.to_vec()) {
            Err(_) => String::new(),
            Ok(date) => date,
        },
    }
}

pub fn read_emails(imap_sess: &mut ImapSession, mbox: &str, query: &str) -> imap::Result<()> {
    imap_sess.select(mbox)?;

    let seqs = imap_sess
        .search(query)?
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>();

    let table_head = vec![
        table::Cell::new(
            vec![table::BOLD, table::UNDERLINE, table::WHITE],
            String::from("FLAGS"),
        ),
        table::Cell::new(
            vec![table::BOLD, table::UNDERLINE, table::WHITE],
            String::from("FROM"),
        ),
        table::Cell::new(
            vec![table::BOLD, table::UNDERLINE, table::WHITE],
            String::from("SUBJECT"),
        ),
        table::Cell::new(
            vec![table::BOLD, table::UNDERLINE, table::WHITE],
            String::from("DATE"),
        ),
    ];

    let mut table_rows = imap_sess
        .fetch(
            seqs[..20.min(seqs.len())].join(","),
            "(INTERNALDATE ENVELOPE)",
        )?
        .iter()
        .map(|fetch| {
            vec![
                table::Cell::new(vec![table::WHITE], String::from("!@")),
                table::Cell::new(vec![table::BLUE], first_addr_from_fetch(fetch)),
                table::Cell::new(vec![table::GREEN], subject_from_fetch(fetch)),
                table::Cell::new(vec![table::YELLOW], date_from_fetch(fetch)),
            ]
        })
        .collect::<Vec<_>>();

    table_rows.insert(0, table_head);

    println!("{}", table::render(table_rows));

    Ok(())
}

// List mailboxes
// let mboxes = imap_sess.list(Some(""), Some("*"))?;

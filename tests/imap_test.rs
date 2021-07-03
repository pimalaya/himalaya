extern crate himalaya;

use std::convert::TryFrom;

use himalaya::{
    config::model::Account, imap::model::ImapConnector, mbox::model::Mboxes, msg::model::Msgs, smtp,
};

fn get_account(addr: &str) -> Account {
    Account {
        name: None,
        downloads_dir: None,
        signature: None,
        default_page_size: None,
        default: Some(true),
        email: addr.into(),
        watch_cmds: None,
        imap_host: String::from("localhost"),
        imap_port: 3993,
        imap_starttls: Some(false),
        imap_insecure: Some(true),
        imap_login: addr.into(),
        imap_passwd_cmd: String::from("echo 'password'"),
        smtp_host: String::from("localhost"),
        smtp_port: 3465,
        smtp_starttls: Some(false),
        smtp_insecure: Some(true),
        smtp_login: addr.into(),
        smtp_passwd_cmd: String::from("echo 'password'"),
    }
}

#[test]
fn mbox() {
    let account = get_account("inbox@localhost");
    let mut imap_conn = ImapConnector::new(&account).unwrap();
    let names = imap_conn.list_mboxes().unwrap();
    let mboxes: Vec<String> = Mboxes::from(&names)
        .0
        .into_iter()
        .map(|mbox| mbox.name)
        .collect();
    assert_eq!(mboxes, vec![String::from("INBOX")]);
    imap_conn.logout();
}

#[test]
fn msg() {
    // =================
    // Preparations
    // =================
    let account = get_account("inbox@localhost");

    // Login
    let mut imap_conn = ImapConnector::new(&account).unwrap();

    // remove all previous mails first
    let fetches = imap_conn.list_msgs("INBOX", &10, &0).unwrap();
    let msgs = if let Some(ref fetches) = fetches {
        Msgs::try_from(fetches).unwrap()
    } else {
        Msgs::new()
    };

    for msg in msgs.0.iter() {
        imap_conn
            .add_flags("INBOX", &msg.get_uid().unwrap().to_string(), "\\Deleted")
            .unwrap();
    }
    imap_conn.expunge("INBOX").unwrap();

    // ============
    // Testing
    // ============
    // Add messages
    smtp::send(
        &account,
        &lettre::Message::builder()
            .from("sender-a@localhost".parse().unwrap())
            .to("inbox@localhost".parse().unwrap())
            .subject("Subject A")
            .singlepart(lettre::message::SinglePart::builder().body("Body A".as_bytes().to_vec()))
            .unwrap(),
    )
    .unwrap();
    smtp::send(
        &account,
        &lettre::Message::builder()
            .from("\"Sender B\" <sender-b@localhost>".parse().unwrap())
            .to("inbox@localhost".parse().unwrap())
            .subject("Subject B")
            .singlepart(lettre::message::SinglePart::builder().body("Body B".as_bytes().to_vec()))
            .unwrap(),
    )
    .unwrap();

    // List messages
    // TODO: check non-existance of \Seen flag
    let msgs = imap_conn.list_msgs("INBOX", &10, &0).unwrap();
    let msgs = if let Some(ref fetches) = msgs {
        Msgs::try_from(fetches).unwrap()
    } else {
        Msgs::new()
    };

    // make sure that there are both mails which we sended
    assert_eq!(msgs.0.len(), 2);
    dbg!("{:?}", &msgs);

    let msg_a = msgs
        .0
        .iter()
        .find(|msg| msg.envelope.subject.clone().unwrap() == "Subject A")
        .unwrap();

    dbg!("{:?}", &msg_a);

    assert_eq!(msg_a.envelope.subject.clone().unwrap(), "Subject A");
    assert_eq!(msg_a.envelope.sender.clone().unwrap(), "sender-a@localhost");

    let msg_b = msgs
        .0
        .iter()
        .find(|msg| msg.envelope.subject.clone().unwrap() == "Subject B")
        .unwrap();
    assert_eq!(msg_b.envelope.subject.clone().unwrap(), "Subject B");
    assert_eq!(msg_b.envelope.sender.clone().unwrap(), "Sender B");

    // TODO: search messages
    // TODO: read message (+ \Seen flag)
    // TODO: list message attachments
    // TODO: add/set/remove flags
    assert!(imap_conn.list_msgs("INBOX", &10, &0).unwrap().is_none());

    // Logout
    imap_conn.logout();
}

extern crate himalaya;

use himalaya::{config::model::Account, imap::model::ImapConnector, msg::model::Msgs, smtp};

fn get_account(addr: &str) -> Account {
    Account {
        name: None,
        downloads_dir: None,
        signature: None,
        default_page_size: None,
        default: Some(true),
        email: addr.into(),
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
    let mboxes: Vec<String> = imap_conn
        .list_mboxes()
        .unwrap()
        .0
        .into_iter()
        .map(|mbox| mbox.name)
        .collect();
    assert_eq!(mboxes, vec![String::from("INBOX")]);
    imap_conn.logout();
}

#[test]
fn msg() {
    let account = get_account("inbox@localhost");

    // Let's add message
    smtp::send(
        &account,
        &lettre::Message::builder()
            .from("sender@localhost".parse().unwrap())
            .to("inbox@localhost".parse().unwrap())
            .subject("Very important message")
            .singlepart(
                lettre::message::SinglePart::builder().body("Hello, world!".as_bytes().to_vec()),
            )
            .unwrap(),
    )
    .unwrap();

    // We should see the message
    let mut imap_conn = ImapConnector::new(&account).unwrap();
    let msgs = imap_conn.list_msgs("INBOX", &10, &0).unwrap();
    let msgs = Msgs::from(&msgs);
    assert_eq!(msgs.0.len(), 1);
    let msg = msgs.0.first().unwrap();
    assert_eq!("Very important message", msg.subject.as_str());
    imap_conn.logout();
}

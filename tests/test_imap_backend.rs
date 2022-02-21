use himalaya::{
    backends::{Backend, ImapBackend, ImapEnvelopes},
    config::{AccountConfig, ImapBackendConfig},
};

#[test]
fn test_imap_backend() {
    // configure accounts
    let account_config = AccountConfig {
        inbox_folder: "INBOX".into(),
        draft_folder: "Sent".into(),
        smtp_host: "localhost".into(),
        smtp_port: 3465,
        smtp_starttls: false,
        smtp_insecure: true,
        smtp_login: "inbox@localhost".into(),
        smtp_passwd_cmd: "echo 'password'".into(),
        ..AccountConfig::default()
    };
    let imap_config = ImapBackendConfig {
        imap_host: "localhost".into(),
        imap_port: 3993,
        imap_starttls: false,
        imap_insecure: true,
        imap_login: "inbox@localhost".into(),
        imap_passwd_cmd: "echo 'password'".into(),
    };
    let mut imap = ImapBackend::new(&account_config, &imap_config);

    // check that a mailbox can be created
    assert!(imap.add_mbox("Mailbox").is_ok());

    // check that a message can be added
    let msg = include_bytes!("./emails/alice-to-patrick.eml");
    let id = imap.add_msg("INBOX", msg, "seen").unwrap().to_string();

    // check that the added message exists
    let msg = imap.get_msg("INBOX", &id).unwrap();
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    // check that the envelope of the added message exists
    let envelopes = imap
        .get_envelopes("INBOX", "arrival:desc", "ALL", 10, 0)
        .unwrap();
    let envelopes: &ImapEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.sender);
    assert_eq!("Plain message", envelope.subject);

    // check that the message can be copied
    imap.copy_msg("INBOX", "Mailbox", &envelope.id.to_string())
        .unwrap();
    let envelopes = imap
        .get_envelopes("INBOX", "arrival:desc", "ALL", 10, 0)
        .unwrap();
    let envelopes: &ImapEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    assert_eq!(1, envelopes.len());
    let envelopes = imap
        .get_envelopes("Mailbox", "arrival:desc", "ALL", 10, 0)
        .unwrap();
    let envelopes: &ImapEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    assert_eq!(1, envelopes.len());

    // check that the message can be moved
    imap.move_msg("INBOX", "Mailbox", &envelope.id.to_string())
        .unwrap();
    let envelopes = imap
        .get_envelopes("INBOX", "arrival:desc", "ALL", 10, 0)
        .unwrap();
    let envelopes: &ImapEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    assert_eq!(0, envelopes.len());
    let envelopes = imap
        .get_envelopes("Mailbox", "arrival:desc", "ALL", 10, 0)
        .unwrap();
    let envelopes: &ImapEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    assert_eq!(2, envelopes.len());
    let id = envelopes.first().unwrap().id.to_string();

    // check that the message can be deleted
    imap.del_msg("Mailbox", &id).unwrap();
    assert!(imap.get_msg("Mailbox", &id).is_err());
}

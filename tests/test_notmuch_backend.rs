use std::{collections::HashMap, env, fs, iter::FromIterator};

use himalaya::{
    backends::{Backend, MaildirBackend, NotmuchBackend, NotmuchEnvelopes},
    config::{AccountConfig, MaildirBackendConfig, NotmuchBackendConfig},
};

#[test]
fn test_notmuch_backend() {
    // set up maildir folders and notmuch database
    let mdir: maildir::Maildir = env::temp_dir().join("himalaya-test-notmuch").into();
    if let Err(_) = fs::remove_dir_all(mdir.path()) {}
    mdir.create_dirs().unwrap();
    notmuch::Database::create(mdir.path()).unwrap();

    // configure accounts
    let account_config = AccountConfig {
        mailboxes: HashMap::from_iter([("inbox".into(), "*".into())]),
        ..AccountConfig::default()
    };
    let mdir_config = MaildirBackendConfig {
        maildir_dir: mdir.path().to_owned(),
    };
    let notmuch_config = NotmuchBackendConfig {
        notmuch_database_dir: mdir.path().to_owned(),
    };
    let mut mdir = MaildirBackend::new(&account_config, &mdir_config);
    let mut notmuch = NotmuchBackend::new(&account_config, &notmuch_config, &mut mdir).unwrap();

    // check that a message can be added
    let msg = include_bytes!("./emails/alice-to-patrick.eml");
    let hash = notmuch.add_msg("", msg, "inbox seen").unwrap().to_string();

    // check that the added message exists
    let msg = notmuch.get_msg("", &hash).unwrap();
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    // check that the envelope of the added message exists
    let envelopes = notmuch.get_envelopes("inbox", 10, 0).unwrap();
    let envelopes: &NotmuchEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.sender);
    assert_eq!("Plain message", envelope.subject);

    // check that a flag can be added to the message
    notmuch
        .add_flags("", &envelope.hash, "flagged passed")
        .unwrap();
    let envelopes = notmuch.get_envelopes("inbox", 1, 0).unwrap();
    let envelopes: &NotmuchEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&"inbox".into()));
    assert!(envelope.flags.contains(&"seen".into()));
    assert!(envelope.flags.contains(&"flagged".into()));
    assert!(envelope.flags.contains(&"passed".into()));

    // check that the message flags can be changed
    notmuch
        .set_flags("", &envelope.hash, "inbox passed")
        .unwrap();
    let envelopes = notmuch.get_envelopes("inbox", 1, 0).unwrap();
    let envelopes: &NotmuchEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&"inbox".into()));
    assert!(!envelope.flags.contains(&"seen".into()));
    assert!(!envelope.flags.contains(&"flagged".into()));
    assert!(envelope.flags.contains(&"passed".into()));

    // check that a flag can be removed from the message
    notmuch.del_flags("", &envelope.hash, "passed").unwrap();
    let envelopes = notmuch.get_envelopes("inbox", 1, 0).unwrap();
    let envelopes: &NotmuchEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&"inbox".into()));
    assert!(!envelope.flags.contains(&"seen".into()));
    assert!(!envelope.flags.contains(&"flagged".into()));
    assert!(!envelope.flags.contains(&"passed".into()));
}

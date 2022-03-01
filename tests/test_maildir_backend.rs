use maildir::Maildir;
use std::{collections::HashMap, env, fs, iter::FromIterator};

use himalaya::{
    backends::{Backend, MaildirBackend, MaildirEnvelopes, MaildirFlag},
    config::{AccountConfig, MaildirBackendConfig},
};

#[test]
fn test_maildir_backend() {
    // set up maildir folders
    let mdir: Maildir = env::temp_dir().join("himalaya-test-mdir").into();
    if let Err(_) = fs::remove_dir_all(mdir.path()) {}
    mdir.create_dirs().unwrap();

    let mdir_sub: Maildir = mdir.path().join(".Subdir").into();
    if let Err(_) = fs::remove_dir_all(mdir_sub.path()) {}
    mdir_sub.create_dirs().unwrap();

    // configure accounts
    let account_config = AccountConfig {
        mailboxes: HashMap::from_iter([("inbox".into(), "INBOX".into())]),
        ..AccountConfig::default()
    };
    let mdir_config = MaildirBackendConfig {
        maildir_dir: mdir.path().to_owned(),
    };
    let mut mdir = MaildirBackend::new(&account_config, &mdir_config);
    let mdir_sub_config = MaildirBackendConfig {
        maildir_dir: mdir_sub.path().to_owned(),
    };
    let mut mdir_subdir = MaildirBackend::new(&account_config, &mdir_sub_config);

    // check that a message can be added
    let msg = include_bytes!("./emails/alice-to-patrick.eml");
    let hash = mdir.add_msg("INBOX", msg, "seen").unwrap().to_string();

    // check that the added message exists
    let msg = mdir.get_msg("INBOX", &hash).unwrap();
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    // check that the envelope of the added message exists
    let envelopes = mdir.get_envelopes("INBOX", 10, 0).unwrap();
    let envelopes: &MaildirEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.sender);
    assert_eq!("Plain message", envelope.subject);

    // check that a flag can be added to the message
    mdir.add_flags("INBOX", &envelope.hash, "flagged passed")
        .unwrap();
    let envelopes = mdir.get_envelopes("INBOX", 1, 0).unwrap();
    let envelopes: &MaildirEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&MaildirFlag::Seen));
    assert!(envelope.flags.contains(&MaildirFlag::Flagged));
    assert!(envelope.flags.contains(&MaildirFlag::Passed));

    // check that the message flags can be changed
    mdir.set_flags("INBOX", &envelope.hash, "passed").unwrap();
    let envelopes = mdir.get_envelopes("INBOX", 1, 0).unwrap();
    let envelopes: &MaildirEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&MaildirFlag::Seen));
    assert!(!envelope.flags.contains(&MaildirFlag::Flagged));
    assert!(envelope.flags.contains(&MaildirFlag::Passed));

    // check that a flag can be removed from the message
    mdir.del_flags("INBOX", &envelope.hash, "passed").unwrap();
    let envelopes = mdir.get_envelopes("INBOX", 1, 0).unwrap();
    let envelopes: &MaildirEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&MaildirFlag::Seen));
    assert!(!envelope.flags.contains(&MaildirFlag::Flagged));
    assert!(!envelope.flags.contains(&MaildirFlag::Passed));

    // check that the message can be copied
    mdir.copy_msg("INBOX", "Subdir", &envelope.hash).unwrap();
    assert!(mdir.get_msg("INBOX", &hash).is_ok());
    assert!(mdir.get_msg("Subdir", &hash).is_ok());
    assert!(mdir_subdir.get_msg("INBOX", &hash).is_ok());

    // check that the message can be moved
    mdir.move_msg("INBOX", "Subdir", &envelope.hash).unwrap();
    assert!(mdir.get_msg("INBOX", &hash).is_err());
    assert!(mdir.get_msg("Subdir", &hash).is_ok());
    assert!(mdir_subdir.get_msg("INBOX", &hash).is_ok());

    // check that the message can be deleted
    mdir.del_msg("Subdir", &hash).unwrap();
    assert!(mdir.get_msg("Subdir", &hash).is_err());
    assert!(mdir_subdir.get_msg("INBOX", &hash).is_err());
}

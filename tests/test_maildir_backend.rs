use maildir::Maildir;
use std::{env, fs};

use himalaya::{
    backends::{Backend, MaildirBackend, MaildirEnvelopes},
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
        inbox_folder: "INBOX".into(),
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
    let id = mdir.add_msg("INBOX", msg, "seen").unwrap().to_string();

    // check that the added message exists
    let msg = mdir.get_msg("INBOX", &id).unwrap();
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    // check that the envelope of the added message exists
    let envelopes = mdir.get_envelopes("INBOX", "", "cur", 10, 0).unwrap();
    let envelopes: &MaildirEnvelopes = envelopes.as_any().downcast_ref().unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.sender);
    assert_eq!("Plain message", envelope.subject);

    // check that the message can be copied
    mdir.copy_msg("INBOX", "Subdir", &envelope.id).unwrap();
    assert!(mdir.get_msg("INBOX", &id).is_ok());
    assert!(mdir.get_msg("Subdir", &id).is_ok());
    assert!(mdir_subdir.get_msg("INBOX", &id).is_ok());

    // check that the message can be moved
    mdir.move_msg("INBOX", "Subdir", &envelope.id).unwrap();
    assert!(mdir.get_msg("INBOX", &id).is_err());
    assert!(mdir.get_msg("Subdir", &id).is_ok());
    assert!(mdir_subdir.get_msg("INBOX", &id).is_ok());

    // check that the message can be deleted
    mdir.del_msg("Subdir", &id).unwrap();
    assert!(mdir.get_msg("Subdir", &id).is_err());
    assert!(mdir_subdir.get_msg("INBOX", &id).is_err());
}

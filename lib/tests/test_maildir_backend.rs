use maildir::Maildir;
use std::{collections::HashMap, env, fs, iter::FromIterator};

use himalaya_lib::{
    account::{Account, MaildirBackendConfig},
    backend::{Backend, MaildirBackend},
    msg::Flag,
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
    let account_config = Account {
        mailboxes: HashMap::from_iter([("subdir".into(), "Subdir".into())]),
        ..Account::default()
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
    let hash = mdir.add_msg("inbox", msg, "seen").unwrap();

    // check that the added message exists
    let msg = mdir.get_msg("inbox", &hash).unwrap();
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    // check that the envelope of the added message exists
    let envelopes = mdir.get_envelopes("inbox", 10, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.sender);
    assert_eq!("Plain message", envelope.subject);

    // check that a flag can be added to the message
    mdir.add_flags("inbox", &envelope.id, "flagged").unwrap();
    let envelopes = mdir.get_envelopes("inbox", 1, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));

    // check that the message flags can be changed
    mdir.set_flags("inbox", &envelope.id, "answered").unwrap();
    let envelopes = mdir.get_envelopes("inbox", 1, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that a flag can be removed from the message
    mdir.del_flags("inbox", &envelope.id, "answered").unwrap();
    let envelopes = mdir.get_envelopes("inbox", 1, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that the message can be copied
    mdir.copy_msg("inbox", "subdir", &envelope.id).unwrap();
    assert!(mdir.get_msg("inbox", &hash).is_ok());
    assert!(mdir.get_msg("subdir", &hash).is_ok());
    assert!(mdir_subdir.get_msg("inbox", &hash).is_ok());

    // check that the message can be moved
    mdir.move_msg("inbox", "subdir", &envelope.id).unwrap();
    assert!(mdir.get_msg("inbox", &hash).is_err());
    assert!(mdir.get_msg("subdir", &hash).is_ok());
    assert!(mdir_subdir.get_msg("inbox", &hash).is_ok());

    // check that the message can be deleted
    mdir.del_msg("subdir", &hash).unwrap();
    assert!(mdir.get_msg("subdir", &hash).is_err());
    assert!(mdir_subdir.get_msg("inbox", &hash).is_err());
}

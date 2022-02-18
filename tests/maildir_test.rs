use anyhow::Result;
use maildir::Maildir;
use std::env;

use himalaya::{
    backends::{Backend, MaildirBackend},
    config::MaildirBackendConfig,
};

#[test]
fn test_maildir() -> Result<()> {
    let dir = env::temp_dir().join("himalaya-test-maildir");
    let maildir: Maildir = dir.clone().into();
    maildir.create_dirs()?;

    let maildir_config = MaildirBackendConfig { maildir_dir: dir };
    let mut maildir = MaildirBackend::new(&maildir_config);
    let msg = include_bytes!("./emails/alice-to-patrick.eml");
    let id = maildir.add_msg("", msg, "seen")?;

    let msg = maildir.get_msg("", &id)?;
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    let envelopes = maildir.get_envelopes("", "", "cur", 10, 1)?;
    assert_eq!(1, envelopes.len());

    // TODO: test flags methods

    maildir.del_msg("", &id)?;
    let envelopes = maildir.get_envelopes("", "", "cur", 10, 1)?;
    assert_eq!(0, envelopes.len());

    Ok(())
}

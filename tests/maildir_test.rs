use anyhow::Result;
use maildir::Maildir;
use std::env;

use himalaya::{
    config::{AccountConfig, MaildirBackendConfig},
    domain::{BackendService, MaildirService, Mbox},
};

#[test]
fn test_maildir() -> Result<()> {
    let dir = env::temp_dir().join("himalaya-test-maildir");
    let maildir: Maildir = dir.clone().into();
    maildir.create_dirs()?;

    let account_config = AccountConfig {
        ..AccountConfig::default()
    };
    let maildir_config = MaildirBackendConfig { maildir_dir: dir };
    let mut maildir = MaildirService::new(&account_config, &maildir_config);
    let mbox = Mbox::new("INBOX");
    let msg = include_bytes!("./emails/alice-to-patrick.eml");
    let id = maildir.add_msg(&mbox, msg, "seen".into())?;

    let msg = maildir.get_msg(&id)?;
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    let msgs = maildir.get_envelopes(&[], "", &10, &1)?;
    assert_eq!(1, msgs.len());

    Ok(())
}

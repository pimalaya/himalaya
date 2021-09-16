use anyhow::Result;

use crate::domain::{imap::service::ImapServiceInterface, msg::flag::entity::Flags};

pub fn set<ImapService: ImapServiceInterface>(
    uid: &str,
    flags: &str,
    imap: &mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.set_flags(uid, flags)?;
    imap.logout()?;
    Ok(())
}

pub fn add<ImapService: ImapServiceInterface>(
    uid: &str,
    flags: &str,
    imap: &mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.add_flags(uid, flags)?;
    imap.logout()?;
    Ok(())
}

pub fn remove<ImapService: ImapServiceInterface>(
    uid: &str,
    flags: &str,
    imap: &mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.remove_flags(uid, flags)?;
    imap.logout()?;
    Ok(())
}

//! Module related to message flag handling.
//!
//! This module gathers all message flag commands.  

use anyhow::Result;

use crate::{
    domain::{imap::service::ImapServiceInterface, msg::flag::entity::Flags},
    output::service::OutputServiceInterface,
};

/// Add flags from the given message UID sequence.
/// Flags do not need to be prefixed with `\` and they are not case-sensitive.
///
/// ```ignore
/// add("21", "\\Seen", &output, &mut imap)?;
/// add("42", "recent", &output, &mut imap)?;
/// add("1:10", "Answered custom", &output, &mut imap)?;
/// ```
pub fn add<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &'a str,
    flags: Vec<&'a str>,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.add_flags(uid, &flags)?;
    output.print(format!(
        r#"Flag(s) "{}" successfully added to message {}"#,
        flags, uid
    ))?;
    Ok(())
}

/// Remove flags from the given message UID sequence.
/// Flags do not need to be prefixed with `\` and they are not case-sensitive.
///
/// ```ignore
/// remove("21", "\\Seen", &output, &mut imap)?;
/// remove("42", "recent", &output, &mut imap)?;
/// remove("1:10", "Answered custom", &output, &mut imap)?;
/// ```
pub fn remove<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &'a str,
    flags: Vec<&'a str>,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.remove_flags(uid, &flags)?;
    output.print(format!(
        r#"Flag(s) "{}" successfully removed from message {}"#,
        flags, uid
    ))?;
    Ok(())
}

/// Replace flags from the given message UID sequence.
/// Flags do not need to be prefixed with `\` and they are not case-sensitive.
///
/// ```ignore
/// set("21", "\\Seen", &output, &mut imap)?;
/// set("42", "recent", &output, &mut imap)?;
/// set("1:10", "Answered custom", &output, &mut imap)?;
/// ```
pub fn set<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    uid: &'a str,
    flags: Vec<&'a str>,
    output: &'a OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let flags = Flags::from(flags);
    imap.set_flags(uid, &flags)?;
    output.print(format!(
        r#"Flag(s) "{}" successfully set for message {}"#,
        flags, uid
    ))?;
    Ok(())
}

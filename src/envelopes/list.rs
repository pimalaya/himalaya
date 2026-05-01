#[cfg(feature = "maildir")]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(any(feature = "imap", feature = "jmap"))]
use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    account::Account,
    cli::BackendArg,
    config::{AccountConfig, Config},
    envelopes::table::EnvelopesTable,
};

#[cfg(any(feature = "imap", feature = "jmap"))]
const READ_BUFFER_SIZE: usize = 16 * 1024;

/// List envelopes for the active account, regardless of the underlying
/// backend (IMAP, JMAP or Maildir).
#[derive(Debug, Parser)]
pub struct EnvelopesListCommand {
    /// Path or name of the IMAP/Maildir mailbox.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "PATH",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    /// Page number, starting from 1. The most recent envelopes are on
    /// page 1.
    #[arg(long, short = 'p', value_name = "N", default_value = "1")]
    pub page: u32,

    /// Maximum number of envelopes per page.
    #[arg(
        long = "page-size",
        short = 's',
        value_name = "N",
        default_value = "25"
    )]
    pub page_size: u32,
}

impl EnvelopesListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        mut account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        #[cfg(feature = "imap")]
        if backend.allows_imap() {
            if let Some(imap_config) = account_config.imap.take() {
                use io_email::imap::envelope_list::{EnvelopeList, EnvelopeListResult};
                use io_imap::types::mailbox::Mailbox;
                use pimalaya_toolbox::stream::imap::ImapSession;

                let account = Account::new(config, account_config, imap_config)?;
                let mut session = ImapSession::new(
                    account.backend.url.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.starttls,
                    account.backend.sasl.clone().try_into()?,
                )?;

                let mailbox: Mailbox<'static> = self.mailbox.clone().try_into()?;
                let mut coroutine = EnvelopeList::new(
                    session.context,
                    mailbox,
                    Some(self.page),
                    Some(self.page_size),
                );
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                let envelopes = loop {
                    match coroutine.resume(arg.take()) {
                        EnvelopeListResult::Ok(envelopes) => break envelopes,
                        EnvelopeListResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        EnvelopeListResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        EnvelopeListResult::Err(err) => bail!("{err}"),
                    }
                };

                return printer.out(EnvelopesTable {
                    preset: account.table_preset,
                    arrangement: account.table_arrangement,
                    envelopes,
                });
            }
        }

        #[cfg(feature = "jmap")]
        if backend.allows_jmap() {
            if let Some(jmap_config) = account_config.jmap.take() {
                use io_email::jmap::envelope_list::{EnvelopeList, EnvelopeListResult};
                use pimalaya_toolbox::stream::jmap::JmapSession;

                let account = Account::new(config, account_config, jmap_config)?;
                let mut session = JmapSession::new(
                    account.backend.server.clone(),
                    account.backend.tls.clone().try_into()?,
                    account.backend.auth.clone().try_into()?,
                )?;
                let mut coroutine = EnvelopeList::new(
                    &session.session,
                    &session.http_auth,
                    Some(self.page),
                    Some(self.page_size),
                )?;
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut arg: Option<&[u8]> = None;

                let envelopes = loop {
                    match coroutine.resume(arg.take()) {
                        EnvelopeListResult::Ok(envelopes) => break envelopes,
                        EnvelopeListResult::WantsRead => {
                            let n = session.stream.read(&mut buf)?;
                            arg = Some(&buf[..n]);
                        }
                        EnvelopeListResult::WantsWrite(bytes) => {
                            session.stream.write_all(&bytes)?;
                        }
                        EnvelopeListResult::Err(err) => bail!("{err}"),
                    }
                };

                return printer.out(EnvelopesTable {
                    preset: account.table_preset,
                    arrangement: account.table_arrangement,
                    envelopes,
                });
            }
        }

        #[cfg(feature = "maildir")]
        if backend.allows_maildir() {
            if let Some(maildir_config) = account_config.maildir.take() {
                use io_email::maildir::envelope_list::{
                    EnvelopeList, EnvelopeListArg, EnvelopeListResult,
                };
                use io_maildir::maildir::Maildir;

                let account = Account::new(config, account_config, maildir_config)?;
                let path = account.backend.root.join(&self.mailbox);
                let maildir = Maildir::try_from(path)?;

                let mut coroutine =
                    EnvelopeList::new(maildir, Some(self.page), Some(self.page_size));
                let mut arg: Option<EnvelopeListArg> = None;

                let envelopes = loop {
                    match coroutine.resume(arg.take()) {
                        EnvelopeListResult::Ok(envelopes) => break envelopes,
                        EnvelopeListResult::WantsDirRead(paths) => {
                            arg = Some(EnvelopeListArg::DirRead(read_dirs(&paths)?));
                        }
                        EnvelopeListResult::WantsFileRead(paths) => {
                            arg = Some(EnvelopeListArg::FileRead(read_files(&paths)?));
                        }
                        EnvelopeListResult::Err(err) => bail!("{err}"),
                    }
                };

                return printer.out(EnvelopesTable {
                    preset: account.table_preset,
                    arrangement: account.table_arrangement,
                    envelopes,
                });
            }
        }

        bail!("no backend matching `{backend}` is configured for this account")
    }
}

#[cfg(feature = "maildir")]
fn read_dirs(paths: &BTreeSet<String>) -> Result<BTreeMap<String, BTreeSet<String>>> {
    use std::fs;

    let mut out = BTreeMap::new();

    for path in paths {
        let mut entries = BTreeSet::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;

            if let Some(s) = entry.path().to_str() {
                entries.insert(s.to_owned());
            }
        }

        out.insert(path.clone(), entries);
    }

    Ok(out)
}

#[cfg(feature = "maildir")]
fn read_files(paths: &BTreeSet<String>) -> Result<BTreeMap<String, Vec<u8>>> {
    use std::fs;

    let mut out = BTreeMap::new();

    for path in paths {
        out.insert(path.clone(), fs::read(path)?);
    }

    Ok(out)
}

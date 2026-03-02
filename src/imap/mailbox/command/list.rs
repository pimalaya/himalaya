use anyhow::{bail, Result};
use clap::Parser;
use io_imap::{
    coroutines::{
        authenticate::*, authenticate_anonymous::*, authenticate_plain::*, list::*, login::*,
    },
    types::response::Capability,
};
use io_stream::runtimes::std::handle;
use log::warn;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{account::Account, sasl::SaslMechanism, stream};

/// List all mailboxes.
///
/// This command allows you to list all exsting mailboxes from your
/// IMAP account.
#[derive(Debug, Parser)]
pub struct ListMailboxesCommand {
    // /// The maximum width the table should not exceed.
    // ///
    // /// This argument will force the table not to exceed the given
    // /// width, in pixels. Columns may shrink with ellipsis in order to
    // /// fit the width.
    // #[arg(long = "max-width", short = 'w')]
    // #[arg(name = "table_max_width", value_name = "PIXELS")]
    // pub table_max_width: Option<u16>,
}

impl ListMailboxesCommand {
    pub fn execute(self, printer: &mut impl Printer, account: Account) -> Result<()> {
        let imap = account.imap.unwrap();

        let (mut context, mut stream) = if imap.tls.disable {
            let port = imap.port.unwrap_or(143);
            stream::tcp(imap.host, port)?
        } else {
            let port = imap.port.unwrap_or(if imap.starttls { 143 } else { 993 });
            stream::rustls(imap.host, port, imap.starttls, imap.tls.cert)?
        };

        let ir = context.capability.contains(&Capability::SaslIr);

        let mut candidates = vec![];

        for mechanism in imap.sasl.mechanisms {
            match mechanism {
                SaslMechanism::Login => {
                    let Some(ref auth) = imap.sasl.login else {
                        warn!("missing SASL LOGIN configuration, skipping it");
                        continue;
                    };

                    let params = ImapLoginParams::new(&auth.username, auth.password.get()?)?;
                    candidates.push(ImapAuthenticateCandidate::Login(params));
                }
                SaslMechanism::Plain => {
                    let Some(ref auth) = imap.sasl.plain else {
                        warn!("missing SASL PLAIN configuration, skipping it");
                        continue;
                    };

                    let params = ImapAuthenticatePlainParams::new(
                        auth.authzid.as_ref(),
                        &auth.authcid,
                        auth.passwd.get()?,
                        ir,
                    );

                    candidates.push(ImapAuthenticateCandidate::Plain(params))
                }
                SaslMechanism::Anonymous => {
                    let msg = imap
                        .sasl
                        .anonymous
                        .as_ref()
                        .and_then(|auth| auth.message.as_ref());

                    let params = ImapAuthenticateAnonymousParams::new(msg, ir);

                    candidates.push(ImapAuthenticateCandidate::Anonymous(params))
                }
            }
        }

        let mut arg = None;
        let mut coroutine = ImapAuthenticate::new(context, candidates);

        loop {
            match coroutine.resume(arg.take()) {
                ImapAuthenticateResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                ImapAuthenticateResult::Ok { context: c } => break context = c,
                ImapAuthenticateResult::Err { err, .. } => bail!(err),
            }
        }

        let mut arg = None;
        let mut coroutine = ImapList::new(context, "".try_into().unwrap(), "*".try_into().unwrap());

        let mailboxes = loop {
            match coroutine.resume(arg.take()) {
                ImapListResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                ImapListResult::Ok(ok) => break ok.mailboxes,
                ImapListResult::Err(err) => bail!(err),
            }
        };

        println!("mailboxes: {mailboxes:#?}");

        // TODO: list folders

        // let folders = Folders::from(backend.list_folders().await?);
        // let table = FoldersTable::from(folders)
        //     .with_some_width(self.table_max_width)
        //     .with_some_preset(toml_account_config.folder_list_table_preset())
        //     .with_some_name_color(toml_account_config.folder_list_table_name_color())
        //     .with_some_desc_color(toml_account_config.folder_list_table_desc_color());

        // printer.out(table)?;
        Ok(())
    }
}

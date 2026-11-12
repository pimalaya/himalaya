//! # Vacation command
//!
//! The `vacation` command, dispatching onto its subcommands over the
//! client of the backend keeping the auto-reply.

// NOTE: a build with neither JMAP nor Gmail compiles the tree with no
// client to run it on, the resolver refusing the verb before any runs.
#![cfg_attr(
    not(any(feature = "jmap", feature = "gmail")),
    allow(unused_variables, unreachable_code)
)]

use anyhow::Result;
#[cfg(not(any(feature = "jmap", feature = "gmail")))]
use anyhow::bail;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

#[cfg(feature = "gmail")]
use crate::gmail::client::GmailClient;
#[cfg(feature = "jmap")]
use crate::jmap::client::JmapClient;
use crate::{
    account::context::Account,
    capability,
    email::vacation::{Vacation, VacationPatch},
    shared::vacation::{get::VacationGetCommand, set::VacationSetCommand},
};

/// Manage the auto-reply using the unified API.
///
/// The auto-reply answers incoming mail while it is on, within its window
/// when it has one. It is a feature of the account rather than of a
/// mailbox, so it is read from the first configured backend keeping one,
/// whatever backend stores the mail.
#[derive(Debug, Subcommand)]
pub enum VacationCommand {
    #[command(after_long_help = capability::help("vacation", "get"))]
    Get(VacationGetCommand),
    #[command(after_long_help = capability::help("vacation", "set"))]
    Set(VacationSetCommand),
}

impl VacationCommand {
    /// Runs the subcommand over the backend keeping the auto-reply.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: VacationClient<'_>,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Set(cmd) => cmd.execute(printer, account, client),
        }
    }
}

/// The client of the backend the auto-reply resolved to.
pub enum VacationClient<'a> {
    #[cfg(feature = "jmap")]
    Jmap(&'a mut JmapClient),
    #[cfg(feature = "gmail")]
    Gmail(&'a mut GmailClient),
    /// Holds the lifetime in a build with neither backend.
    #[cfg(not(any(feature = "jmap", feature = "gmail")))]
    None(std::marker::PhantomData<&'a ()>),
}

impl VacationClient<'_> {
    /// Reads the auto-reply.
    pub fn get(self) -> Result<Vacation> {
        match self {
            #[cfg(feature = "jmap")]
            Self::Jmap(client) => client.get_vacation(),
            #[cfg(feature = "gmail")]
            Self::Gmail(client) => client.get_vacation(),
            #[cfg(not(any(feature = "jmap", feature = "gmail")))]
            Self::None(_) => bail!("No backend keeping an auto-reply is compiled in"),
        }
    }

    /// Changes what `patch` names of the auto-reply, returning it as it
    /// then stands.
    pub fn set(self, patch: VacationPatch) -> Result<Vacation> {
        match self {
            #[cfg(feature = "jmap")]
            Self::Jmap(client) => client.set_vacation(patch),
            #[cfg(feature = "gmail")]
            Self::Gmail(client) => client.set_vacation(patch),
            #[cfg(not(any(feature = "jmap", feature = "gmail")))]
            Self::None(_) => bail!("No backend keeping an auto-reply is compiled in"),
        }
    }
}

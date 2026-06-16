pub mod autoforwarding;
pub mod convert;
pub mod delegates;
pub mod filters;
pub mod forwarding_addresses;
pub mod imap;
pub mod language;
pub mod pop;
pub mod sendas;
pub mod vacation;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::gmail::{
    client::GmailClient,
    settings::{
        autoforwarding::GmailSettingsAutoForwardingCommand,
        delegates::GmailSettingsDelegatesCommand, filters::GmailSettingsFiltersCommand,
        forwarding_addresses::GmailSettingsForwardingAddressesCommand,
        imap::GmailSettingsImapCommand, language::GmailSettingsLanguageCommand,
        pop::GmailSettingsPopCommand, sendas::GmailSettingsSendAsCommand,
        vacation::GmailSettingsVacationCommand,
    },
};

/// Manage Gmail settings (users.settings), organized by sub-resource.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsCommand {
    #[command(subcommand)]
    Vacation(GmailSettingsVacationCommand),
    #[command(subcommand)]
    Imap(GmailSettingsImapCommand),
    #[command(subcommand)]
    Pop(GmailSettingsPopCommand),
    #[command(subcommand)]
    Language(GmailSettingsLanguageCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["autoforwarding"])]
    AutoForwarding(GmailSettingsAutoForwardingCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["filter"])]
    Filters(GmailSettingsFiltersCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["forwarding-address"])]
    ForwardingAddresses(GmailSettingsForwardingAddressesCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["delegate"])]
    Delegates(GmailSettingsDelegatesCommand),
    #[command(subcommand)]
    #[command(visible_aliases = ["sendas"])]
    SendAs(GmailSettingsSendAsCommand),
}

impl GmailSettingsCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Vacation(cmd) => cmd.execute(printer, account, client),
            Self::Imap(cmd) => cmd.execute(printer, account, client),
            Self::Pop(cmd) => cmd.execute(printer, account, client),
            Self::Language(cmd) => cmd.execute(printer, account, client),
            Self::AutoForwarding(cmd) => cmd.execute(printer, account, client),
            Self::Filters(cmd) => cmd.execute(printer, account, client),
            Self::ForwardingAddresses(cmd) => cmd.execute(printer, account, client),
            Self::Delegates(cmd) => cmd.execute(printer, account, client),
            Self::SendAs(cmd) => cmd.execute(printer, account, client),
        }
    }
}

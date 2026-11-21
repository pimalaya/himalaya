//! # Gmail filter list
//!
//! The `gmail settings filters list` command,
//! `users.settings.filters.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::filters::GmailFilter;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::filters::summary::{action_summary, criteria_summary},
    },
    unified::backend::Backend,
};

/// List all Gmail filters (users.settings.filters.list).
///
/// JSON output: Gmail's answer as Gmail sent it, `{"filter": [<Gmail
/// filter>...]}`, `[]` when there is none.
#[derive(Debug, Parser)]
pub struct GmailSettingsFiltersListCommand;

impl GmailSettingsFiltersListCommand {
    /// Lists every filter and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let page = client.raw_list_at(&["settings", "filters"], "filter", Vec::new())?;

        printer.out(GmailSettingsFilterListOutput {
            account: account.name().to_string(),
            filter: page.items,
            rest: page.rest,
        })
    }
}

/// The `gmail settings filters list` output, the Gmail filters.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailSettingsFilterListOutput {
    #[serde(skip)]
    account: String,
    /// The filters, each as Gmail sent it.
    #[schemars(with = "Vec<Map<String, Value>>")]
    filter: Vec<Value>,
    /// The rest of Gmail's answer as Gmail sent it.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

impl fmt::Display for GmailSettingsFilterListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));

        for filter in &self.filter {
            let filter: GmailFilter = serde_json::from_value(filter.clone()).unwrap_or_default();
            let criteria = filter.criteria.as_ref().map(criteria_summary);
            let action = filter.action.as_ref().map(action_summary);

            listing.items.push([
                sanitize::text(&filter.id).into_owned(),
                criteria.unwrap_or_default(),
                action.unwrap_or_default(),
            ]);
        }

        write!(f, "{listing}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::{
        account::context::Account,
        gmail::{
            client::GmailClient,
            settings::{
                delegates::list::GmailSettingsDelegatesListCommand,
                filters::list::GmailSettingsFiltersListCommand,
                forwarding_addresses::list::GmailSettingsForwardingAddressesListCommand,
                sendas::list::GmailSettingsSendAsListCommand,
            },
        },
    };

    /// Each settings listing prints Gmail's answer as sent, under Gmail's
    /// key, a property io-gmail does not model kept, and `[]` when Gmail
    /// leaves the key out.
    #[test]
    fn the_settings_lists_print_as_gmail_sent_them() {
        let run = |answer: serde_json::Value, path: &str, command: u8| {
            let (mut client, requests) = GmailClient::scripted(&[answer]);
            let mut printer = CapturePrinter::default();
            let account = &mut Account::default();
            match command {
                0 => GmailSettingsSendAsListCommand.execute(&mut printer, account, &mut client),
                1 => GmailSettingsDelegatesListCommand.execute(&mut printer, account, &mut client),
                2 => GmailSettingsForwardingAddressesListCommand.execute(
                    &mut printer,
                    account,
                    &mut client,
                ),
                _ => GmailSettingsFiltersListCommand.execute(&mut printer, account, &mut client),
            }
            .unwrap();
            let request = requests.lock().unwrap()[0].clone();
            assert!(request.contains(path), "{request}");
            printer.into_single().unwrap()
        };

        for (command, key, path) in [
            (0, "sendAs", "/settings/sendAs "),
            (1, "delegates", "/settings/delegates "),
            (2, "forwardingAddresses", "/settings/forwardingAddresses "),
            (3, "filter", "/settings/filters "),
        ] {
            let answer = json!({key: [{"id": "f1", "x-vendor": 1}]});
            assert_eq!(run(answer.clone(), path, command), answer);
            assert_eq!(run(json!({}), path, command), json!({key: []}));
        }
    }
}

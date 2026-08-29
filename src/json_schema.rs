//! # JSON Schema registry
//!
//! Maps a CLI-invocation key, the command path joined with hyphens and
//! prefixed `himalaya-`, to the JSON Schema of that command's `--json`
//! payload. [`JsonSchemaCommand`] writes one file per entry.
//!
//! Protocol-specific entries are gated behind the same cargo features as
//! their command modules, so the registry stays coherent under any feature
//! combination, none included.
//!
//! [`JsonSchemaCommand`]: pimalaya_cli::clap::commands::JsonSchemaCommand

use std::collections::BTreeMap;

use schemars::schema_for;
use serde_json::Value;

/// Builds the command-to-schema map consumed by `json-schema <DIR>`.
///
/// Each value describes the type the command hands to the printer.
/// Paginated listings use the `Paginated<T>` wrapper, so the extra
/// `next_page` field is described too.
pub fn schemas() -> BTreeMap<String, Value> {
    let mut schemas = BTreeMap::new();

    macro_rules! insert {
        ($key:expr, $ty:ty) => {
            schemas.insert(
                $key.to_string(),
                serde_json::to_value(schema_for!($ty)).unwrap(),
            );
        };
    }

    #[cfg(backend)]
    {
        insert!(
            "himalaya-mailbox-list",
            crate::shared::mailbox::list::Mailboxes
        );
        insert!(
            "himalaya-envelope-list",
            crate::shared::envelope::list::Envelopes
        );
        // NOTE: `envelope search` renders the `Envelopes` table of `list`.
        insert!(
            "himalaya-envelope-search",
            crate::shared::envelope::list::Envelopes
        );
        insert!("himalaya-flag-add", crate::shared::flag::add::AddedFlags);
        insert!("himalaya-flag-set", crate::shared::flag::set::SetFlags);
        insert!(
            "himalaya-flag-remove",
            crate::shared::flag::remove::RemovedFlags
        );
        insert!(
            "himalaya-message-add",
            crate::shared::message::add::MessageAddOutput
        );
        insert!(
            "himalaya-message-read",
            crate::shared::message::read::MessageView
        );
        insert!(
            "himalaya-message-delete",
            crate::shared::message::delete::DeleteReport
        );
        insert!(
            "himalaya-attachment-list",
            crate::shared::attachment::list::Attachments
        );
        // NOTE: `attachment download` reports the `Attachments` table of
        // `list`.
        insert!(
            "himalaya-attachment-download",
            crate::shared::attachment::list::Attachments
        );
    }

    insert!(
        "himalaya-configure",
        crate::wizard::configure::GeneratedConfig
    );
    insert!("himalaya-account-list", crate::account::list::AccountsTable);
    insert!("himalaya-account-check", crate::account::check::CheckReport);

    #[cfg(feature = "imap")]
    {
        insert!("himalaya-imap-id", crate::imap::id::ServerIdTable);
        insert!(
            "himalaya-imap-status",
            crate::imap::mailbox::status::MailboxStatus
        );
        insert!(
            "himalaya-imap-list",
            crate::imap::mailbox::list::MailboxesTable
        );
        insert!(
            "himalaya-imap-search",
            crate::imap::envelope::search::SearchTable
        );
        insert!(
            "himalaya-imap-sort",
            crate::imap::envelope::sort::SortResultsTable
        );
        insert!(
            "himalaya-imap-thread",
            crate::imap::envelope::thread::ThreadResultsTable
        );
        insert!(
            "himalaya-imap-flag",
            crate::imap::flag::list::FlagsTable<'static>
        );
        insert!("himalaya-imap-fetch", crate::imap::fetch::FetchedMessages);
    }

    #[cfg(feature = "sieve")]
    {
        insert!(
            "himalaya-sieve-capability",
            crate::sieve::capability::SieveCapabilities
        );
        insert!("himalaya-sieve-list", crate::sieve::list::SieveScripts);
        insert!("himalaya-sieve-get", crate::sieve::get::SieveScriptOutput);
    }

    #[cfg(feature = "jmap")]
    {
        insert!("himalaya-jmap-query", crate::jmap::query::RawResponse);
        insert!(
            "himalaya-jmap-mailbox-get",
            crate::jmap::mailbox::query::MailboxesTable
        );
        insert!(
            "himalaya-jmap-mailbox-query",
            crate::jmap::mailbox::query::MailboxesTable
        );
        insert!(
            "himalaya-jmap-email-get",
            crate::jmap::email::query::EmailsTable
        );
        insert!(
            "himalaya-jmap-email-query",
            crate::jmap::email::query::EmailsTable
        );
        insert!(
            "himalaya-jmap-email-parse",
            crate::jmap::email::parse::ParsedBodies
        );
        insert!(
            "himalaya-jmap-thread-get",
            crate::jmap::thread::get::ThreadsTable
        );
        insert!(
            "himalaya-jmap-identity-get",
            crate::jmap::identity::get::IdentitiesTable
        );
        insert!(
            "himalaya-jmap-submission-get",
            crate::jmap::submission::query::SubmissionsTable
        );
        insert!(
            "himalaya-jmap-submission-query",
            crate::jmap::submission::query::SubmissionsTable
        );
        insert!(
            "himalaya-jmap-submission-create",
            crate::jmap::submission::query::SubmissionsTable
        );
        insert!(
            "himalaya-jmap-vacation-response-get",
            crate::jmap::vacation::get::VacationTable
        );
    }

    #[cfg(feature = "gmail")]
    {
        use crate::shared::output::Paginated;

        insert!(
            "himalaya-gmail-profile-get",
            crate::gmail::profile::get::GmailProfileOutput
        );
        insert!(
            "himalaya-gmail-labels-list",
            crate::gmail::labels::list::LabelsTable
        );
        insert!(
            "himalaya-gmail-labels-get",
            crate::gmail::labels::list::LabelsTable
        );
        insert!(
            "himalaya-gmail-messages-list",
            Paginated<crate::gmail::messages::list::MessageIdsTable>
        );
        insert!(
            "himalaya-gmail-messages-get",
            crate::gmail::messages::get::GmailMessageGetOutput
        );
        insert!(
            "himalaya-gmail-drafts-list",
            Paginated<crate::gmail::drafts::list::DraftsTable>
        );
        insert!(
            "himalaya-gmail-drafts-get",
            crate::gmail::drafts::get::GmailDraftGetOutput
        );
        insert!(
            "himalaya-gmail-threads-list",
            Paginated<crate::gmail::threads::list::ThreadsTable>
        );
        insert!(
            "himalaya-gmail-threads-get",
            crate::gmail::threads::get::GmailThreadGetOutput
        );
        insert!(
            "himalaya-gmail-history-list",
            Paginated<crate::gmail::history::list::GmailHistoryListOutput>
        );
        insert!(
            "himalaya-gmail-settings-filters-list",
            crate::gmail::settings::filters::list::FiltersTable
        );
        insert!(
            "himalaya-gmail-settings-filters-get",
            crate::gmail::settings::filters::get::GmailSettingsFilterGetOutput
        );
        insert!(
            "himalaya-gmail-settings-forwarding-addresses-list",
            crate::gmail::settings::forwarding_addresses::list::ForwardingAddressesTable
        );
        insert!(
            "himalaya-gmail-settings-forwarding-addresses-get",
            crate::gmail::settings::forwarding_addresses::get::GmailSettingsForwardingAddressGetOutput
        );
        insert!(
            "himalaya-gmail-settings-delegates-list",
            crate::gmail::settings::delegates::list::DelegatesTable
        );
        insert!(
            "himalaya-gmail-settings-delegates-get",
            crate::gmail::settings::delegates::get::GmailSettingsDelegateGetOutput
        );
        insert!(
            "himalaya-gmail-settings-send-as-list",
            crate::gmail::settings::sendas::list::SendAsTable
        );
        insert!(
            "himalaya-gmail-settings-send-as-get",
            crate::gmail::settings::sendas::get::GmailSettingsSendAsGetOutput
        );
        insert!(
            "himalaya-gmail-settings-vacation-get",
            crate::gmail::settings::vacation::get::GmailSettingsVacationGetOutput
        );
        insert!(
            "himalaya-gmail-settings-auto-forwarding-get",
            crate::gmail::settings::autoforwarding::get::GmailSettingsAutoForwardingGetOutput
        );
        insert!(
            "himalaya-gmail-settings-pop-get",
            crate::gmail::settings::pop::get::GmailSettingsPopGetOutput
        );
        insert!(
            "himalaya-gmail-settings-imap-get",
            crate::gmail::settings::imap::get::GmailSettingsImapGetOutput
        );
        insert!(
            "himalaya-gmail-settings-language-get",
            crate::gmail::settings::language::get::GmailSettingsLanguageGetOutput
        );
    }

    #[cfg(feature = "msgraph")]
    {
        use crate::shared::output::Paginated;

        insert!(
            "himalaya-msgraph-profile-get",
            crate::msgraph::profile::get::MsgraphProfileOutput
        );
        insert!(
            "himalaya-msgraph-mail-folders-list",
            crate::msgraph::mail_folders::list::MailFoldersTable
        );
        insert!(
            "himalaya-msgraph-mail-folders-child-folders",
            crate::msgraph::mail_folders::list::MailFoldersTable
        );
        insert!(
            "himalaya-msgraph-mail-folders-get",
            crate::msgraph::mail_folders::list::MailFoldersTable
        );
        insert!(
            "himalaya-msgraph-messages-list",
            Paginated<crate::msgraph::messages::list::MessagesTable>
        );
        insert!(
            "himalaya-msgraph-messages-get",
            crate::msgraph::messages::get::MsgraphMessageGetOutput
        );
        insert!(
            "himalaya-msgraph-attachments-list",
            crate::msgraph::attachments::list::AttachmentsTable
        );
    }

    #[cfg(feature = "maildir")]
    {
        insert!("himalaya-maildir-list", crate::maildir::list::MaildirsTable);
        insert!(
            "himalaya-maildir-flag-list",
            crate::maildir::flag::list::FlagsTable
        );
        insert!(
            "himalaya-maildir-message-save",
            crate::maildir::message::save::StoredMessage
        );
    }

    #[cfg(feature = "m2dir")]
    {
        insert!("himalaya-m2dir-list", crate::m2dir::list::M2dirsTable);
        insert!(
            "himalaya-m2dir-flag-list",
            crate::m2dir::flag::list::FlagsTable
        );
        insert!(
            "himalaya-m2dir-message-save",
            crate::m2dir::message::save::StoredMessage
        );
    }

    #[cfg(feature = "pimdir")]
    {
        insert!(
            "himalaya-pimdir-queue-list",
            crate::pimdir::queue::list::PimdirQueuedMessages
        );
        insert!(
            "himalaya-pimdir-queue-cancel",
            crate::pimdir::queue::cancel::PimdirQueueCancelled
        );
    }

    schemas
}

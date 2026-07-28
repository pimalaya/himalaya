//! JSON Schema registry for the `json-schema <DIR>` meta command.
//!
//! For every command that emits structured `--json` output, this module
//! maps a CLI-invocation key (the command path joined with hyphens and
//! prefixed `himalaya-`, mirroring how the man pages are named
//! `himalaya-<cmd>-<subcmd>.1`) to the JSON Schema describing that
//! command's `--json` payload. [`JsonSchemaCommand`] writes one
//! `<key>.json` file per entry.
//!
//! Protocol-specific entries are gated behind the same cargo features as
//! their command modules, so the registry compiles and stays coherent
//! under any feature combination (including none): only the schemas for
//! compiled-in backends are emitted.
//!
//! [`JsonSchemaCommand`]: pimalaya_cli::clap::commands::JsonSchemaCommand

use std::collections::BTreeMap;

use schemars::schema_for;
use serde_json::Value;

/// Builds the command-to-schema map consumed by `json-schema <DIR>`.
///
/// Each value is the JSON Schema of the concrete Rust type the command
/// hands to `printer.out(...)` (the same value serialized under
/// `--json`). Paginated listings use the `Paginated<T>` wrapper so the
/// extra `next_page` field is described.
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

    // --- Shared API (needs a storage backend, like the commands)

    #[cfg(any(
        feature = "imap",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "maildir",
        feature = "m2dir"
    ))]
    {
        insert!(
            "himalaya-mailbox-list",
            crate::shared::mailbox::list::Mailboxes
        );
        insert!(
            "himalaya-envelope-list",
            crate::shared::envelope::list::Envelopes
        );
        // `envelope search` renders the same `Envelopes` table as `list`.
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
            "himalaya-attachment-list",
            crate::shared::attachment::list::Attachments
        );
        // `attachment download` reports the same `Attachments` table.
        insert!(
            "himalaya-attachment-download",
            crate::shared::attachment::list::Attachments
        );
    }

    // --- Account meta commands (always present)

    insert!("himalaya-account-list", crate::account::list::AccountsTable);
    insert!("himalaya-account-check", crate::account::check::CheckReport);

    // --- IMAP

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
            "himalaya-imap-flags",
            crate::imap::flag::list::FlagsTable<'static>
        );
        insert!("himalaya-imap-fetch", crate::imap::fetch::FetchedMessages);
    }

    // --- JMAP

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

    // --- Gmail

    #[cfg(feature = "gmail")]
    {
        use crate::shared::output::Paginated;

        insert!(
            "himalaya-gmail-profile-get",
            crate::gmail::profile::GmailProfileOutput
        );
        insert!(
            "himalaya-gmail-labels-list",
            crate::gmail::labels::LabelsTable
        );
        insert!(
            "himalaya-gmail-labels-get",
            crate::gmail::labels::LabelsTable
        );
        insert!(
            "himalaya-gmail-messages-list",
            Paginated<crate::gmail::messages::MessageIdsTable>
        );
        insert!(
            "himalaya-gmail-drafts-list",
            Paginated<crate::gmail::drafts::DraftsTable>
        );
        insert!(
            "himalaya-gmail-threads-list",
            Paginated<crate::gmail::threads::ThreadsTable>
        );
        insert!(
            "himalaya-gmail-history-list",
            Paginated<crate::gmail::history::HistoryOutput>
        );
        insert!(
            "himalaya-gmail-settings-filters-list",
            crate::gmail::settings::filters::FiltersTable
        );
        insert!(
            "himalaya-gmail-settings-forwarding-addresses-list",
            crate::gmail::settings::forwarding_addresses::ForwardingAddressesTable
        );
        insert!(
            "himalaya-gmail-settings-delegates-list",
            crate::gmail::settings::delegates::DelegatesTable
        );
        insert!(
            "himalaya-gmail-settings-send-as-list",
            crate::gmail::settings::sendas::SendAsTable
        );
    }

    // --- Microsoft Graph

    #[cfg(feature = "msgraph")]
    {
        use crate::shared::output::Paginated;

        insert!(
            "himalaya-msgraph-profile-get",
            crate::msgraph::profile::get::MsgraphProfileOutput
        );
        insert!(
            "himalaya-msgraph-mail-folder-list",
            crate::msgraph::mail_folder::list::MailFoldersTable
        );
        insert!(
            "himalaya-msgraph-mail-folder-child-folders",
            crate::msgraph::mail_folder::list::MailFoldersTable
        );
        insert!(
            "himalaya-msgraph-mail-folder-get",
            crate::msgraph::mail_folder::list::MailFoldersTable
        );
        insert!(
            "himalaya-msgraph-message-list",
            Paginated<crate::msgraph::message::list::MessagesTable>
        );
        insert!(
            "himalaya-msgraph-attachment-list",
            crate::msgraph::attachment::list::AttachmentsTable
        );
    }

    // --- Maildir

    #[cfg(feature = "maildir")]
    {
        insert!("himalaya-maildir-list", crate::maildir::list::MaildirsTable);
        insert!(
            "himalaya-maildir-flags-list",
            crate::maildir::flag::list::FlagsTable
        );
        insert!(
            "himalaya-maildir-messages-save",
            crate::maildir::message::save::StoredMessage
        );
    }

    // --- m2dir

    #[cfg(feature = "m2dir")]
    {
        insert!("himalaya-m2dir-list", crate::m2dir::list::M2dirsTable);
        insert!(
            "himalaya-m2dir-flags-list",
            crate::m2dir::flag::list::FlagsTable
        );
        insert!(
            "himalaya-m2dir-messages-save",
            crate::m2dir::message::save::StoredMessage
        );
    }

    schemas
}

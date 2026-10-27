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

use pimalaya_cli::{clap::commands::JsonSchemaRegistry, error::ErrorReport};
use schemars::schema_for;

use crate::error::ErrorCode;

/// Builds the command-to-schema registry consumed by `json-schema <DIR>`.
///
/// Each value describes the type the command hands to the printer, and
/// `himalaya-error` the failure object every command prints instead.
/// Paginated listings use the `Paginated<T>` wrapper, so the extra
/// `nextPage` field is described too.
pub fn registry() -> JsonSchemaRegistry {
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
            crate::shared::mailbox::list::MailboxListOutput
        );
        insert!(
            "himalaya-envelope-list",
            crate::shared::envelope::list::EnvelopeListOutput
        );
        // NOTE: `envelope search` renders the `EnvelopeListOutput` of `list`.
        insert!(
            "himalaya-envelope-search",
            crate::shared::envelope::list::EnvelopeListOutput
        );
        insert!("himalaya-flag-add", crate::shared::flag::add::FlagAddOutput);
        insert!("himalaya-flag-set", crate::shared::flag::set::FlagSetOutput);
        insert!(
            "himalaya-flag-remove",
            crate::shared::flag::remove::FlagRemoveOutput
        );
        insert!(
            "himalaya-message-add",
            crate::shared::message::add::MessageAddOutput
        );
        // NOTE: `message send` prints the confirmation line only.
        insert!(
            "himalaya-message-send",
            crate::shared::message::handler::MessageRouteOutput
        );
        insert!(
            "himalaya-message-read",
            crate::shared::message::read::MessageReadOutput
        );
        // NOTE: `message parse` prints the `message read` view, or under
        // `--part` the part `attachment download --stdout` prints.
        insert!(
            "himalaya-message-parse",
            crate::shared::message::parse::MessageParseOutput
        );
        insert!(
            "himalaya-message-delete",
            crate::shared::message::delete::MessageDeleteOutput
        );
        // NOTE: the composers print a `MessageTemplateOutput` when neither
        // saving nor sending, and a confirmation line otherwise.
        insert!(
            "himalaya-message-compose",
            crate::shared::message::handler::MessageTemplateOutput
        );
        insert!(
            "himalaya-message-reply",
            crate::shared::message::handler::MessageTemplateOutput
        );
        insert!(
            "himalaya-message-forward",
            crate::shared::message::handler::MessageTemplateOutput
        );
        insert!(
            "himalaya-attachment-list",
            crate::shared::attachment::list::AttachmentListOutput
        );
        // NOTE: `attachment download` reports the `AttachmentListOutput` of
        // `list`, or the one part `--stdout --json` reads.
        insert!(
            "himalaya-attachment-download",
            crate::shared::attachment::download::AttachmentDownloadOutput
        );
    }

    #[cfg(feature = "wizard")]
    insert!(
        "himalaya-configure",
        crate::wizard::configure::ConfigureOutput
    );
    insert!(
        "himalaya-account-list",
        crate::account::list::AccountListOutput
    );
    insert!(
        "himalaya-account-check",
        crate::account::check::AccountCheckOutput
    );

    #[cfg(feature = "imap")]
    {
        insert!("himalaya-imap-id", crate::imap::id::ImapIdOutput);
        insert!(
            "himalaya-imap-status",
            crate::imap::mailbox::status::ImapMailboxStatusOutput
        );
        insert!(
            "himalaya-imap-list",
            crate::imap::mailbox::list::ImapMailboxListOutput
        );
        insert!(
            "himalaya-imap-search",
            crate::imap::envelope::search::ImapSearchOutput
        );
        insert!(
            "himalaya-imap-sort",
            crate::imap::envelope::sort::ImapSortOutput
        );
        insert!(
            "himalaya-imap-thread",
            crate::imap::envelope::thread::ImapThreadOutput
        );
        insert!(
            "himalaya-imap-flag",
            crate::imap::flag::list::ImapFlagListOutput<'static>
        );
        insert!("himalaya-imap-fetch", crate::imap::fetch::ImapFetchOutput);
    }

    #[cfg(feature = "sieve")]
    {
        insert!(
            "himalaya-sieve-capability",
            crate::sieve::capability::SieveCapabilityOutput
        );
        insert!("himalaya-sieve-list", crate::sieve::list::SieveListOutput);
        insert!("himalaya-sieve-get", crate::sieve::get::SieveScriptOutput);
    }

    #[cfg(feature = "jmap")]
    {
        insert!("himalaya-jmap-query", crate::jmap::query::JmapQueryOutput);
        insert!(
            "himalaya-jmap-mailbox-get",
            crate::jmap::mailbox::query::JmapMailboxQueryOutput
        );
        insert!(
            "himalaya-jmap-mailbox-query",
            crate::jmap::mailbox::query::JmapMailboxQueryOutput
        );
        insert!(
            "himalaya-jmap-email-get",
            crate::jmap::email::query::JmapEmailQueryOutput
        );
        insert!(
            "himalaya-jmap-email-query",
            crate::jmap::email::query::JmapEmailQueryOutput
        );
        insert!(
            "himalaya-jmap-email-parse",
            crate::jmap::email::parse::JmapEmailParseOutput
        );
        insert!(
            "himalaya-jmap-thread-get",
            crate::jmap::thread::get::JmapThreadGetOutput
        );
        insert!(
            "himalaya-jmap-identity-get",
            crate::jmap::identity::get::JmapIdentityGetOutput
        );
        insert!(
            "himalaya-jmap-submission-get",
            crate::jmap::submission::query::JmapSubmissionQueryOutput
        );
        insert!(
            "himalaya-jmap-submission-query",
            crate::jmap::submission::query::JmapSubmissionQueryOutput
        );
        insert!(
            "himalaya-jmap-submission-create",
            crate::jmap::submission::query::JmapSubmissionQueryOutput
        );
        insert!(
            "himalaya-jmap-vacation-response-get",
            crate::jmap::vacation::get::JmapVacationResponseGetOutput
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
            crate::gmail::labels::list::GmailLabelListOutput
        );
        insert!(
            "himalaya-gmail-labels-get",
            crate::gmail::labels::list::GmailLabelListOutput
        );
        insert!(
            "himalaya-gmail-messages-list",
            Paginated<crate::gmail::messages::list::GmailMessageListOutput>
        );
        insert!(
            "himalaya-gmail-messages-get",
            crate::gmail::messages::get::GmailMessageGetOutput
        );
        insert!(
            "himalaya-gmail-drafts-list",
            Paginated<crate::gmail::drafts::list::GmailDraftListOutput>
        );
        insert!(
            "himalaya-gmail-drafts-get",
            crate::gmail::drafts::get::GmailDraftGetOutput
        );
        insert!(
            "himalaya-gmail-drafts-create",
            crate::gmail::drafts::GmailDraftWriteOutput
        );
        insert!(
            "himalaya-gmail-drafts-update",
            crate::gmail::drafts::GmailDraftWriteOutput
        );
        insert!(
            "himalaya-gmail-threads-list",
            Paginated<crate::gmail::threads::list::GmailThreadListOutput>
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
            crate::gmail::settings::filters::list::GmailSettingsFilterListOutput
        );
        insert!(
            "himalaya-gmail-settings-filters-get",
            crate::gmail::settings::filters::get::GmailSettingsFilterGetOutput
        );
        insert!(
            "himalaya-gmail-settings-forwarding-addresses-list",
            crate::gmail::settings::forwarding_addresses::list::GmailSettingsForwardingAddressListOutput
        );
        insert!(
            "himalaya-gmail-settings-forwarding-addresses-get",
            crate::gmail::settings::forwarding_addresses::get::GmailSettingsForwardingAddressGetOutput
        );
        insert!(
            "himalaya-gmail-settings-delegates-list",
            crate::gmail::settings::delegates::list::GmailSettingsDelegateListOutput
        );
        insert!(
            "himalaya-gmail-settings-delegates-get",
            crate::gmail::settings::delegates::get::GmailSettingsDelegateGetOutput
        );
        insert!(
            "himalaya-gmail-settings-send-as-list",
            crate::gmail::settings::sendas::list::GmailSettingsSendAsListOutput
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
            crate::msgraph::mail_folders::list::MsgraphMailFolderListOutput
        );
        insert!(
            "himalaya-msgraph-mail-folders-child-folders",
            crate::msgraph::mail_folders::list::MsgraphMailFolderListOutput
        );
        insert!(
            "himalaya-msgraph-mail-folders-get",
            crate::msgraph::mail_folders::list::MsgraphMailFolderListOutput
        );
        insert!(
            "himalaya-msgraph-messages-list",
            Paginated<crate::msgraph::messages::list::MsgraphMessageListOutput>
        );
        insert!(
            "himalaya-msgraph-messages-get",
            crate::msgraph::messages::get::MsgraphMessageGetOutput
        );
        insert!(
            "himalaya-msgraph-attachments-list",
            crate::msgraph::attachments::list::MsgraphAttachmentListOutput
        );
    }

    #[cfg(feature = "maildir")]
    {
        insert!(
            "himalaya-maildir-list",
            crate::maildir::list::MaildirListOutput
        );
        insert!(
            "himalaya-maildir-flag-list",
            crate::maildir::flag::list::MaildirFlagListOutput
        );
        insert!(
            "himalaya-maildir-message-save",
            crate::maildir::message::save::MaildirMessageSaveOutput
        );
    }

    #[cfg(feature = "mbox")]
    {
        insert!("himalaya-mbox-list", crate::mbox::list::MboxListOutput);
        insert!(
            "himalaya-mbox-flag-list",
            crate::mbox::flag::list::MboxFlagListOutput
        );
        insert!(
            "himalaya-mbox-message-save",
            crate::mbox::message::save::MboxMessageSaveOutput
        );
    }

    #[cfg(feature = "m2dir")]
    {
        insert!("himalaya-m2dir-list", crate::m2dir::list::M2dirListOutput);
        insert!(
            "himalaya-m2dir-flag-list",
            crate::m2dir::flag::list::M2dirFlagListOutput
        );
        insert!(
            "himalaya-m2dir-message-save",
            crate::m2dir::message::save::M2dirMessageSaveOutput
        );
    }

    #[cfg(feature = "pimdir")]
    {
        insert!(
            "himalaya-pimdir-queue-list",
            crate::pimdir::queue::list::PimdirQueueListOutput
        );
        insert!(
            "himalaya-pimdir-queue-cancel",
            crate::pimdir::queue::cancel::PimdirQueueCancelOutput
        );
        insert!(
            "himalaya-pimdir-queue-show",
            crate::pimdir::queue::show::PimdirQueueShowOutput
        );
        insert!(
            "himalaya-pimdir-message-add",
            crate::pimdir::message::add::PimdirMessageAddOutput
        );
        insert!(
            "himalaya-pimdir-message-send",
            crate::pimdir::message::send::PimdirMessageSendOutput
        );
        insert!(
            "himalaya-pimdir-flag-list",
            crate::pimdir::flag::list::PimdirFlagListOutput
        );
        insert!(
            "himalaya-pimdir-flag-add",
            crate::pimdir::flag::PimdirFlagStageOutput
        );
        insert!(
            "himalaya-pimdir-flag-set",
            crate::pimdir::flag::PimdirFlagStageOutput
        );
        insert!(
            "himalaya-pimdir-flag-remove",
            crate::pimdir::flag::PimdirFlagStageOutput
        );
        insert!(
            "himalaya-pimdir-mailbox-create",
            crate::pimdir::mailbox::create::PimdirMailboxCreateOutput
        );
        insert!(
            "himalaya-pimdir-mailbox-list",
            crate::pimdir::mailbox::list::PimdirMailboxListOutput
        );
        insert!(
            "himalaya-pimdir-attachment-list",
            crate::pimdir::attachment::list::PimdirAttachmentListOutput
        );
        // NOTE: `pimdir attachment search` prints the page `list` prints.
        insert!(
            "himalaya-pimdir-attachment-search",
            crate::pimdir::attachment::list::PimdirAttachmentListOutput
        );
    }

    insert!("himalaya-error", ErrorReport<ErrorCode>);

    JsonSchemaRegistry {
        bin: "himalaya",
        version: env!("CARGO_PKG_VERSION"),
        schemas,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_payload_is_an_object() {
        assert_eq!(super::registry().non_object_roots(), Vec::<&str>::new());
    }
}

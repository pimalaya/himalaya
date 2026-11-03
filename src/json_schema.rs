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

use pimalaya_cli::{
    clap::commands::JsonSchemaRegistry,
    error::ErrorReport,
    output::{Bytes, Paginated},
};
use schemars::JsonSchema;

use crate::error::ErrorCode;
#[cfg(any(
    feature = "gmail",
    feature = "msgraph",
    feature = "jmap",
    feature = "maildir",
    feature = "mbox",
    feature = "m2dir",
    feature = "sieve"
))]
use crate::shared::receipt::{IdReceipt, IdsReceipt};

/// Builds the command-to-schema registry consumed by `json-schema <DIR>`.
///
/// Each value describes the type the command hands to the printer, and
/// `himalaya-error` the failure object every command prints instead.
/// Paginated listings use the `Paginated<T>` wrapper, so the extra
/// `nextPage` field is described too, and a command printing one of two
/// types is described by both, through `OneOf`.
pub fn registry() -> JsonSchemaRegistry {
    let mut registry = JsonSchemaRegistry::new("himalaya", env!("CARGO_PKG_VERSION"));

    macro_rules! insert {
        ($key:expr, $ty:ty) => {
            registry.insert::<$ty>($key);
        };
    }

    #[cfg(feature = "imap")]
    macro_rules! imap {
        ($key:expr, $ty:ty) => {
            registry.insert::<crate::imap::outcome::ImapOutcome<$ty>>($key);
        };
    }

    #[cfg(any(backend, feature = "smtp"))]
    {
        insert!(
            "himalaya-mailbox-list",
            crate::shared::mailbox::list::MailboxListOutput
        );
        insert!(
            "himalaya-mailbox-create",
            crate::shared::mailbox::create::MailboxCreateOutput
        );
        insert!(
            "himalaya-mailbox-rename",
            crate::shared::mailbox::rename::MailboxRenameOutput
        );
        insert!(
            "himalaya-mailbox-delete",
            crate::shared::mailbox::delete::MailboxDeleteOutput
        );
        insert!(
            "himalaya-envelope-list",
            Paginated<crate::shared::envelope::list::EnvelopeListOutput>
        );
        // NOTE: `envelope search` renders the page `list` renders.
        insert!(
            "himalaya-envelope-search",
            Paginated<crate::shared::envelope::list::EnvelopeListOutput>
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
        insert!(
            "himalaya-message-copy",
            crate::shared::message::copy::MessageCopyOutput
        );
        insert!(
            "himalaya-message-move",
            crate::shared::message::mv::MessageMoveOutput
        );
        insert!(
            "himalaya-message-send",
            crate::shared::message::handler::MessageRouteOutput
        );
        // NOTE: `message read --raw` prints the message as bytes.
        insert!(
            "himalaya-message-read",
            OneOf<crate::shared::message::read::MessageReadOutput, Bytes>
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
        // NOTE: the composers print the template when neither saving nor
        // sending, and the receipt of the save or the send otherwise.
        insert!(
            "himalaya-message-compose",
            OneOf<
                crate::shared::message::handler::MessageTemplateOutput,
                crate::shared::message::handler::MessageRouteOutput,
            >
        );
        insert!(
            "himalaya-message-reply",
            OneOf<
                crate::shared::message::handler::MessageTemplateOutput,
                crate::shared::message::handler::MessageRouteOutput,
            >
        );
        insert!(
            "himalaya-message-forward",
            OneOf<
                crate::shared::message::handler::MessageTemplateOutput,
                crate::shared::message::handler::MessageRouteOutput,
            >
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
        insert!(
            "himalaya-vacation-get",
            crate::shared::vacation::get::VacationGetOutput
        );
        insert!(
            "himalaya-vacation-set",
            crate::shared::vacation::set::VacationSetOutput
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
    insert!(
        "himalaya-account-capability",
        crate::account::capability::AccountCapabilityOutput
    );
    #[cfg(feature = "imap")]
    {
        imap!("himalaya-imap-id", crate::imap::id::ImapIdOutput);
        imap!(
            "himalaya-imap-status",
            crate::imap::mailbox::status::ImapMailboxStatusOutput
        );
        imap!(
            "himalaya-imap-list",
            crate::imap::mailbox::list::ImapMailboxListOutput
        );
        imap!(
            "himalaya-imap-search",
            crate::imap::envelope::search::ImapSearchOutput
        );
        imap!(
            "himalaya-imap-sort",
            crate::imap::envelope::sort::ImapSortOutput
        );
        imap!(
            "himalaya-imap-thread",
            crate::imap::envelope::thread::ImapThreadOutput
        );
        imap!(
            "himalaya-imap-flag",
            crate::imap::flag::list::ImapFlagListOutput<'static>
        );
        imap!("himalaya-imap-fetch", crate::imap::fetch::ImapFetchOutput);
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
            OneOf<crate::gmail::messages::get::GmailMessageGetOutput, Bytes>
        );
        insert!(
            "himalaya-gmail-drafts-list",
            Paginated<crate::gmail::drafts::list::GmailDraftListOutput>
        );
        insert!(
            "himalaya-gmail-drafts-get",
            OneOf<crate::gmail::drafts::get::GmailDraftGetOutput, Bytes>
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
            OneOf<crate::msgraph::messages::get::MsgraphMessageGetOutput, Bytes>
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
            "himalaya-pimdir-mailbox-rename",
            crate::pimdir::mailbox::rename::PimdirMailboxRenameOutput
        );
        insert!(
            "himalaya-pimdir-mailbox-delete",
            crate::pimdir::mailbox::delete::PimdirMailboxDeleteOutput
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
        // NOTE: `pimdir search --threads` prints the threads instead.
        insert!(
            "himalaya-pimdir-search",
            OneOf<
                crate::pimdir::search::PimdirSearchOutput,
                crate::pimdir::search::PimdirSearchThreadsOutput,
            >
        );
        // NOTE: `pimdir performer` lists the candidates, or with a source
        // or `--clear` prints the choice.
        insert!(
            "himalaya-pimdir-performer",
            OneOf<
                crate::pimdir::performer::PimdirPerformers,
                crate::pimdir::performer::PimdirPerformerChosen,
            >
        );
        insert!(
            "himalaya-pimdir-count",
            crate::pimdir::count::PimdirCountOutput
        );
        insert!(
            "himalaya-pimdir-thread",
            crate::pimdir::thread::PimdirThreadOutput
        );
        insert!("himalaya-pimdir-tag", crate::pimdir::tag::PimdirTagOutput);
    }

    // NOTE: the writes of the protocol namespaces with no payload of their
    // own print a receipt naming what they acted on.

    #[cfg(feature = "imap")]
    {
        use crate::imap::outcome::{ImapDoneOutput, ImapMailboxOutput};

        imap!(
            "himalaya-imap-select",
            crate::imap::mailbox::select::ImapMailboxSelectOutput
        );
        imap!("himalaya-imap-create", ImapMailboxOutput);
        imap!("himalaya-imap-delete", ImapMailboxOutput);
        imap!("himalaya-imap-rename", ImapMailboxOutput);
        imap!("himalaya-imap-subscribe", ImapMailboxOutput);
        imap!("himalaya-imap-unsubscribe", ImapMailboxOutput);
        imap!("himalaya-imap-close", ImapDoneOutput);
        imap!("himalaya-imap-unselect", ImapDoneOutput);
        imap!(
            "himalaya-imap-expunge",
            crate::imap::mailbox::expunge::ImapMailboxExpungeOutput
        );
        imap!("himalaya-imap-store", crate::imap::fetch::ImapFetchOutput);
        imap!(
            "himalaya-imap-append",
            crate::imap::message::save::ImapMessageAppendOutput
        );
        imap!(
            "himalaya-imap-copy",
            crate::imap::message::copy::ImapMessageCopyOutput
        );
        imap!(
            "himalaya-imap-move",
            crate::imap::message::copy::ImapMessageCopyOutput
        );
        insert!("himalaya-imap-raw", crate::imap::raw::ImapRawOutput);
        registry.insert_events::<crate::imap::raw::ImapRawEvent>("himalaya-imap-raw");
    }

    #[cfg(feature = "smtp")]
    {
        insert!("himalaya-smtp-send", crate::smtp::send::SmtpSendOutput);
        insert!("himalaya-smtp-raw", crate::smtp::raw::SmtpRawOutput);
    }

    #[cfg(feature = "sieve")]
    {
        insert!("himalaya-sieve-activate", IdReceipt);
        insert!("himalaya-sieve-delete", IdReceipt);
        insert!("himalaya-sieve-rename", IdReceipt);
        insert!(
            "himalaya-sieve-deactivate",
            crate::sieve::deactivate::SieveDeactivateOutput
        );
        insert!(
            "himalaya-sieve-check",
            crate::sieve::check::SieveScriptOutput
        );
        insert!("himalaya-sieve-put", crate::sieve::check::SieveScriptOutput);
        insert!("himalaya-sieve-raw", crate::sieve::raw::SieveRawOutput);
        registry.insert_events::<crate::sieve::raw::SieveRawEvent>("himalaya-sieve-raw");
    }

    #[cfg(feature = "jmap")]
    {
        insert!("himalaya-jmap-email-copy", IdsReceipt);
        insert!("himalaya-jmap-email-delete", IdsReceipt);
        insert!("himalaya-jmap-email-update", IdsReceipt);
        insert!("himalaya-jmap-email-export", Bytes);
        insert!(
            "himalaya-jmap-email-import",
            crate::jmap::email::import::JmapEmailImportOutput
        );
        insert!(
            "himalaya-jmap-email-read",
            crate::jmap::email::read::JmapEmailReadOutput
        );
        insert!("himalaya-jmap-identity-create", IdReceipt);
        insert!("himalaya-jmap-identity-update", IdReceipt);
        insert!("himalaya-jmap-identity-delete", IdsReceipt);
        insert!("himalaya-jmap-mailbox-create", IdReceipt);
        insert!("himalaya-jmap-mailbox-update", IdReceipt);
        insert!("himalaya-jmap-mailbox-destroy", IdsReceipt);
        insert!("himalaya-jmap-submission-cancel", IdsReceipt);
        insert!("himalaya-jmap-vacation-response-set", IdReceipt);
    }

    #[cfg(feature = "gmail")]
    {
        use crate::{gmail::settings, shared::output::SavedOutput};

        for key in [
            "himalaya-gmail-drafts-delete",
            "himalaya-gmail-drafts-send",
            "himalaya-gmail-labels-create",
            "himalaya-gmail-labels-delete",
            "himalaya-gmail-labels-update",
            "himalaya-gmail-messages-delete",
            "himalaya-gmail-messages-import",
            "himalaya-gmail-messages-insert",
            "himalaya-gmail-messages-modify",
            "himalaya-gmail-messages-send",
            "himalaya-gmail-messages-trash",
            "himalaya-gmail-messages-untrash",
            "himalaya-gmail-settings-delegates-create",
            "himalaya-gmail-settings-delegates-delete",
            "himalaya-gmail-settings-filters-create",
            "himalaya-gmail-settings-filters-delete",
            "himalaya-gmail-settings-forwarding-addresses-create",
            "himalaya-gmail-settings-forwarding-addresses-delete",
            "himalaya-gmail-settings-send-as-create",
            "himalaya-gmail-settings-send-as-delete",
            "himalaya-gmail-settings-send-as-update",
            "himalaya-gmail-settings-send-as-verify",
            "himalaya-gmail-threads-delete",
            "himalaya-gmail-threads-modify",
            "himalaya-gmail-threads-trash",
            "himalaya-gmail-threads-untrash",
        ] {
            insert!(key, IdReceipt);
        }

        insert!("himalaya-gmail-messages-batch-delete", IdsReceipt);
        insert!("himalaya-gmail-messages-batch-modify", IdsReceipt);
        insert!(
            "himalaya-gmail-attachments-get",
            OneOf<Bytes, SavedOutput>
        );
        // NOTE: a settings write prints the settings the server holds
        // after it, as the matching `get` does.
        insert!(
            "himalaya-gmail-settings-auto-forwarding-set",
            settings::autoforwarding::get::GmailSettingsAutoForwardingGetOutput
        );
        insert!(
            "himalaya-gmail-settings-imap-set",
            settings::imap::get::GmailSettingsImapGetOutput
        );
        insert!(
            "himalaya-gmail-settings-language-set",
            settings::language::get::GmailSettingsLanguageGetOutput
        );
        insert!(
            "himalaya-gmail-settings-pop-set",
            settings::pop::get::GmailSettingsPopGetOutput
        );
        insert!(
            "himalaya-gmail-settings-vacation-set",
            settings::vacation::get::GmailSettingsVacationGetOutput
        );
    }

    #[cfg(feature = "msgraph")]
    {
        use crate::shared::output::SavedOutput;

        for key in [
            "himalaya-msgraph-attachments-create",
            "himalaya-msgraph-attachments-delete",
            "himalaya-msgraph-mail-folders-copy",
            "himalaya-msgraph-mail-folders-create",
            "himalaya-msgraph-mail-folders-delete",
            "himalaya-msgraph-mail-folders-move",
            "himalaya-msgraph-mail-folders-rename",
            "himalaya-msgraph-messages-copy",
            "himalaya-msgraph-messages-create",
            "himalaya-msgraph-messages-delete",
            "himalaya-msgraph-messages-move",
            "himalaya-msgraph-messages-send-draft",
            "himalaya-msgraph-messages-update",
        ] {
            insert!(key, IdReceipt);
        }

        insert!(
            "himalaya-msgraph-messages-send",
            crate::msgraph::messages::send::MsgraphMessageSendOutput
        );
        insert!(
            "himalaya-msgraph-attachments-get",
            OneOf<Bytes, SavedOutput>
        );
    }

    #[cfg(feature = "maildir")]
    {
        for key in [
            "himalaya-maildir-create",
            "himalaya-maildir-delete",
            "himalaya-maildir-rename",
        ] {
            insert!(key, IdReceipt);
        }

        for key in [
            "himalaya-maildir-flag-add",
            "himalaya-maildir-flag-remove",
            "himalaya-maildir-flag-set",
            "himalaya-maildir-message-copy",
            "himalaya-maildir-message-move",
        ] {
            insert!(key, IdsReceipt);
        }
    }

    #[cfg(feature = "mbox")]
    {
        for key in [
            "himalaya-mbox-create",
            "himalaya-mbox-delete",
            "himalaya-mbox-rename",
        ] {
            insert!(key, IdReceipt);
        }

        for key in [
            "himalaya-mbox-flag-add",
            "himalaya-mbox-flag-remove",
            "himalaya-mbox-flag-set",
            "himalaya-mbox-message-copy",
            "himalaya-mbox-message-move",
        ] {
            insert!(key, IdsReceipt);
        }
    }

    #[cfg(feature = "m2dir")]
    {
        for key in ["himalaya-m2dir-create", "himalaya-m2dir-delete"] {
            insert!(key, IdReceipt);
        }

        for key in [
            "himalaya-m2dir-flag-add",
            "himalaya-m2dir-flag-remove",
            "himalaya-m2dir-flag-set",
        ] {
            insert!(key, IdsReceipt);
        }
    }

    insert!("himalaya-error", ErrorReport<ErrorCode>);

    registry
}

/// The schema of a command printing one of two types, told apart by their
/// keys.
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum OneOf<A, B> {
    A(A),
    B(B),
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::{cli::Cli, json_schema::registry};

    /// The commands printing nothing a schema could describe: the REPL
    /// and the MCP server speak their own protocols, and a bare
    /// `himalaya` shows the help, or runs the wizard `configure`
    /// describes.
    const SILENT: &[&str] = &["himalaya", "himalaya-repl", "himalaya-mcp"];

    #[test]
    fn every_payload_is_an_object() {
        assert_eq!(registry().non_object_roots(), Vec::<String>::new());
    }

    #[test]
    fn every_command_is_described() {
        let problems = registry().check(&Cli::command(), SILENT);
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}

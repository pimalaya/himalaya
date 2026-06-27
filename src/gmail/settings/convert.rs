//! CLI presentation and input helpers mapping Gmail settings enums to
//! and from their camelCase wire spellings. These converters live in
//! himalaya, not io-gmail, since they are pure CLI affordances. Input
//! enums derive [`ValueEnum`] with the Gmail wire spelling, so the
//! value a `set` command accepts matches what the paired `get` prints.

use clap::ValueEnum;
use io_gmail::v1::rest::settings::{
    GmailDisposition, GmailExpungeBehavior, GmailPopAccessWindow, GmailVerificationStatus,
};

/// Auto-forwarding / POP disposition accepted on the CLI.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum DispositionArg {
    LeaveInInbox,
    Archive,
    Trash,
    MarkRead,
}

impl From<DispositionArg> for GmailDisposition {
    fn from(arg: DispositionArg) -> Self {
        match arg {
            DispositionArg::LeaveInInbox => GmailDisposition::LeaveInInbox,
            DispositionArg::Archive => GmailDisposition::Archive,
            DispositionArg::Trash => GmailDisposition::Trash,
            DispositionArg::MarkRead => GmailDisposition::MarkRead,
        }
    }
}

/// IMAP expunge behavior accepted on the CLI.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum ExpungeBehaviorArg {
    Archive,
    Trash,
    DeleteForever,
}

impl From<ExpungeBehaviorArg> for GmailExpungeBehavior {
    fn from(arg: ExpungeBehaviorArg) -> Self {
        match arg {
            ExpungeBehaviorArg::Archive => GmailExpungeBehavior::Archive,
            ExpungeBehaviorArg::Trash => GmailExpungeBehavior::Trash,
            ExpungeBehaviorArg::DeleteForever => GmailExpungeBehavior::DeleteForever,
        }
    }
}

/// POP access window accepted on the CLI.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum PopAccessWindowArg {
    Disabled,
    FromNowOn,
    AllMail,
}

impl From<PopAccessWindowArg> for GmailPopAccessWindow {
    fn from(arg: PopAccessWindowArg) -> Self {
        match arg {
            PopAccessWindowArg::Disabled => GmailPopAccessWindow::Disabled,
            PopAccessWindowArg::FromNowOn => GmailPopAccessWindow::FromNowOn,
            PopAccessWindowArg::AllMail => GmailPopAccessWindow::AllMail,
        }
    }
}

/// Folds a `--enable` / `--disable` flag pair into a tri-state:
/// `Some(true)` to enable, `Some(false)` to disable, `None` to leave
/// the current value unchanged. The two flags are mutually exclusive
/// at the clap layer.
pub fn enabled_flag(enable: bool, disable: bool) -> Option<bool> {
    if enable {
        Some(true)
    } else if disable {
        Some(false)
    } else {
        None
    }
}

/// Map a disposition to its Gmail wire spelling for display.
pub fn disposition_wire(disposition: GmailDisposition) -> &'static str {
    match disposition {
        GmailDisposition::DispositionUnspecified => "dispositionUnspecified",
        GmailDisposition::LeaveInInbox => "leaveInInbox",
        GmailDisposition::Archive => "archive",
        GmailDisposition::Trash => "trash",
        GmailDisposition::MarkRead => "markRead",
    }
}

/// Map an expunge behavior to its Gmail wire spelling for display.
pub fn expunge_behavior_wire(behavior: GmailExpungeBehavior) -> &'static str {
    match behavior {
        GmailExpungeBehavior::ExpungeBehaviorUnspecified => "expungeBehaviorUnspecified",
        GmailExpungeBehavior::Archive => "archive",
        GmailExpungeBehavior::Trash => "trash",
        GmailExpungeBehavior::DeleteForever => "deleteForever",
    }
}

/// Map a POP access window to its Gmail wire spelling for display.
pub fn access_window_wire(access_window: GmailPopAccessWindow) -> &'static str {
    match access_window {
        GmailPopAccessWindow::AccessWindowUnspecified => "accessWindowUnspecified",
        GmailPopAccessWindow::Disabled => "disabled",
        GmailPopAccessWindow::FromNowOn => "fromNowOn",
        GmailPopAccessWindow::AllMail => "allMail",
    }
}

/// Map a verification status to its Gmail wire spelling for display.
pub fn verification_status_wire(status: GmailVerificationStatus) -> &'static str {
    match status {
        GmailVerificationStatus::VerificationStatusUnspecified => "verificationStatusUnspecified",
        GmailVerificationStatus::Accepted => "accepted",
        GmailVerificationStatus::Pending => "pending",
        GmailVerificationStatus::Rejected => "rejected",
        GmailVerificationStatus::Expired => "expired",
    }
}

//! # Gmail settings conversion
//!
//! Maps the Gmail settings enums to and from their camelCase wire
//! spellings.
//!
//! These are pure CLI affordances, so they live here rather than in
//! io-gmail. Each input enum keeps the wire spelling, so what a `set`
//! accepts is what the paired `get` prints.

use clap::ValueEnum;
use io_gmail::v1::rest::settings::{
    GmailDisposition, GmailExpungeBehavior, GmailPopAccessWindow, GmailVerificationStatus,
};

/// What becomes of a message once it has been forwarded or fetched.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum DispositionArg {
    /// Leave it in the inbox, unread.
    LeaveInInbox,
    /// Archive it.
    Archive,
    /// Move it to the trash.
    Trash,
    /// Leave it in the inbox, marked read.
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

/// What becomes of a message an IMAP client expunges.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum ExpungeBehaviorArg {
    /// Archive it.
    Archive,
    /// Move it to the trash.
    Trash,
    /// Delete it for good.
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

/// Which mail a POP client may fetch.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum PopAccessWindowArg {
    /// None: POP access is off.
    Disabled,
    /// Only the mail arriving from now on.
    FromNowOn,
    /// Every mail in the mailbox.
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

/// Spell a boolean the way the settings commands display it.
pub fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
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

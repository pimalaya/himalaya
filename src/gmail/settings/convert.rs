//! CLI presentation and parsing helpers mapping Gmail settings enums
//! to and from their camelCase wire spellings. These converters live in
//! himalaya, not io-gmail, since they are pure CLI affordances.

use anyhow::{Result, bail};
use io_gmail::v1::rest::settings::{
    GmailDisposition, GmailExpungeBehavior, GmailPopAccessWindow, GmailVerificationStatus,
};

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

/// Parse a CLI disposition into its Gmail enum value.
pub fn parse_disposition(value: &str) -> Result<GmailDisposition> {
    match value {
        "dispositionUnspecified" => Ok(GmailDisposition::DispositionUnspecified),
        "leaveInInbox" => Ok(GmailDisposition::LeaveInInbox),
        "archive" => Ok(GmailDisposition::Archive),
        "trash" => Ok(GmailDisposition::Trash),
        "markRead" => Ok(GmailDisposition::MarkRead),
        other => bail!(
            "Unknown disposition `{other}`, expected one of \
             dispositionUnspecified, leaveInInbox, archive, trash, markRead"
        ),
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

/// Parse a CLI expunge behavior into its Gmail enum value.
pub fn parse_expunge_behavior(value: &str) -> Result<GmailExpungeBehavior> {
    match value {
        "expungeBehaviorUnspecified" => Ok(GmailExpungeBehavior::ExpungeBehaviorUnspecified),
        "archive" => Ok(GmailExpungeBehavior::Archive),
        "trash" => Ok(GmailExpungeBehavior::Trash),
        "deleteForever" => Ok(GmailExpungeBehavior::DeleteForever),
        other => bail!(
            "Unknown expunge behavior `{other}`, expected one of \
             expungeBehaviorUnspecified, archive, trash, deleteForever"
        ),
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

/// Parse a CLI POP access window into its Gmail enum value.
pub fn parse_access_window(value: &str) -> Result<GmailPopAccessWindow> {
    match value {
        "accessWindowUnspecified" => Ok(GmailPopAccessWindow::AccessWindowUnspecified),
        "disabled" => Ok(GmailPopAccessWindow::Disabled),
        "fromNowOn" => Ok(GmailPopAccessWindow::FromNowOn),
        "allMail" => Ok(GmailPopAccessWindow::AllMail),
        other => bail!(
            "Unknown access window `{other}`, expected one of \
             accessWindowUnspecified, disabled, fromNowOn, allMail"
        ),
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

//! # Gmail filter summary
//!
//! Renders a filter's match criteria as the one line its listing
//! shows.

use io_gmail::v1::rest::settings::filters::{GmailFilterAction, GmailFilterCriteria};

/// Best-effort one-line summary of a filter's match criteria.
pub fn criteria_summary(criteria: &GmailFilterCriteria) -> String {
    let mut parts = Vec::new();
    if let Some(from) = &criteria.from {
        parts.push(format!("from={from}"));
    }
    if let Some(to) = &criteria.to {
        parts.push(format!("to={to}"));
    }
    if let Some(subject) = &criteria.subject {
        parts.push(format!("subject={subject}"));
    }
    if let Some(query) = &criteria.query {
        parts.push(format!("query={query}"));
    }
    if let Some(negated_query) = &criteria.negated_query {
        parts.push(format!("negated_query={negated_query}"));
    }
    if criteria.has_attachment == Some(true) {
        parts.push("has_attachment".to_string());
    }
    parts.join(" ")
}

/// Best-effort one-line summary of a filter's action.
pub fn action_summary(action: &GmailFilterAction) -> String {
    let mut parts = Vec::new();
    if let Some(add) = &action.add_label_ids {
        parts.push(format!("+labels={}", add.len()));
    }
    if let Some(remove) = &action.remove_label_ids {
        parts.push(format!("-labels={}", remove.len()));
    }
    if let Some(forward) = &action.forward {
        parts.push(format!("forward={forward}"));
    }
    parts.join(" ")
}

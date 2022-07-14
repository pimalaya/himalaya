//! Message sort criteria module.
//!
//! This module regroups everything related to deserialization of
//! message sort criteria.

use std::{convert::TryFrom, ops::Deref};

use crate::backend::imap::Error;

/// Represents the message sort criteria. It is just a wrapper around
/// the `imap::extensions::sort::SortCriterion`.
pub struct SortCriteria<'a>(Vec<imap::extensions::sort::SortCriterion<'a>>);

impl<'a> Deref for SortCriteria<'a> {
    type Target = Vec<imap::extensions::sort::SortCriterion<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> TryFrom<&'a str> for SortCriteria<'a> {
    type Error = Error;

    fn try_from(criteria_str: &'a str) -> Result<Self, Self::Error> {
        let mut criteria = vec![];
        for criterion_str in criteria_str.split(" ") {
            criteria.push(match criterion_str.trim() {
                "arrival:asc" | "arrival" => Ok(imap::extensions::sort::SortCriterion::Arrival),
                "arrival:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Arrival,
                )),
                "cc:asc" | "cc" => Ok(imap::extensions::sort::SortCriterion::Cc),
                "cc:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Cc,
                )),
                "date:asc" | "date" => Ok(imap::extensions::sort::SortCriterion::Date),
                "date:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Date,
                )),
                "from:asc" | "from" => Ok(imap::extensions::sort::SortCriterion::From),
                "from:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::From,
                )),
                "size:asc" | "size" => Ok(imap::extensions::sort::SortCriterion::Size),
                "size:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Size,
                )),
                "subject:asc" | "subject" => Ok(imap::extensions::sort::SortCriterion::Subject),
                "subject:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Subject,
                )),
                "to:asc" | "to" => Ok(imap::extensions::sort::SortCriterion::To),
                "to:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::To,
                )),
                _ => Err(Error::ParseSortCriterionError(criterion_str.to_owned())),
            }?);
        }
        Ok(Self(criteria))
    }
}

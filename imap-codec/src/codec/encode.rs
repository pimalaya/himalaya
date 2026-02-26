//! # Encoding of messages.
//!
//! To facilitates handling of literals, [Encoder::encode] returns an instance of [`Encoded`].
//! The idea is that the encoder not only "dumps" the final serialization of a message but can be iterated over.
//!
//! # Example
//!
//! ```rust
//! use imap_codec::{
//!     CommandCodec,
//!     encode::{Encoder, Fragment},
//!     imap_types::{
//!         command::{Command, CommandBody},
//!         core::LiteralMode,
//!     },
//! };
//!
//! let command = Command::new("A1", CommandBody::login("Alice", "Pa²²W0rD").unwrap()).unwrap();
//!
//! for fragment in CommandCodec::default().encode(&command) {
//!     match fragment {
//!         Fragment::Line { data } => {
//!             // A line that is ready to be send.
//!             println!("C: {}", String::from_utf8(data).unwrap());
//!         }
//!         Fragment::Literal { data, mode } => match mode {
//!             LiteralMode::Sync => {
//!                 // Wait for a continuation request.
//!                 println!("S: + ...")
//!             }
//!             LiteralMode::NonSync => {
//!                 // We don't need to wait for a continuation request
//!                 // as the server will also not send it.
//!             }
//!         },
//!     }
//! }
//! ```
//!
//! Output of example:
//!
//! ```imap
//! C: A1 LOGIN alice {10}
//! S: + ...
//! C: Pa²²W0rD
//! ```

#[cfg(feature = "ext_condstore_qresync")]
use std::num::NonZeroU64;
use std::{borrow::Borrow, collections::VecDeque, io::Write, num::NonZeroU32};

use base64::{Engine, engine::general_purpose::STANDARD as base64};
use chrono::{DateTime as ChronoDateTime, FixedOffset};
#[cfg(feature = "ext_condstore_qresync")]
use imap_types::command::{FetchModifier, SelectParameter, StoreModifier};
use imap_types::{
    auth::{AuthMechanism, AuthenticateData},
    body::{
        BasicFields, Body, BodyExtension, BodyStructure, Disposition, Language, Location,
        MultiPartExtensionData, SinglePartExtensionData, SpecificFields,
    },
    command::{Command, CommandBody},
    core::{
        AString, Atom, AtomExt, Charset, IString, Literal, LiteralMode, NString, NString8, Quoted,
        QuotedChar, Tag, Text,
    },
    datetime::{DateTime, NaiveDate},
    envelope::{Address, Envelope},
    extensions::idle::IdleDone,
    fetch::{
        Macro, MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName, Part, Section,
    },
    flag::{Flag, FlagFetch, FlagNameAttribute, FlagPerm, StoreResponse, StoreType},
    mailbox::{ListCharString, ListMailbox, Mailbox, MailboxOther},
    response::{
        Bye, Capability, Code, CodeOther, CommandContinuationRequest, Data, Greeting, GreetingKind,
        Response, Status, StatusBody, StatusKind, Tagged,
    },
    search::SearchKey,
    sequence::{SeqOrUid, Sequence, SequenceSet},
    status::{StatusDataItem, StatusDataItemName},
    utils::escape_quoted,
};
use utils::{List1AttributeValueOrNil, List1OrNil, join_serializable};

#[cfg(feature = "ext_namespace")]
use crate::extensions::namespace::encode_namespaces;
use crate::{AuthenticateDataCodec, CommandCodec, GreetingCodec, IdleDoneCodec, ResponseCodec};

/// Encoder.
///
/// Implemented for types that know how to encode a specific IMAP message. See [implementors](trait.Encoder.html#implementors).
pub trait Encoder {
    type Message<'a>;

    /// Encode this message.
    ///
    /// This will return an [`Encoded`] message.
    fn encode(&self, message: &Self::Message<'_>) -> Encoded;
}

/// An encoded message.
///
/// This struct facilitates the implementation of IMAP client- and server implementations by
/// yielding the encoding of a message through [`Fragment`]s. This is required, because the usage of
/// literals (and some other types) may change the IMAP message flow. Thus, in many cases, it is an
/// error to just "dump" a message and send it over the network.
///
/// # Example
///
/// ```rust
/// use imap_codec::{
///     CommandCodec,
///     encode::{Encoder, Fragment},
///     imap_types::command::{Command, CommandBody},
/// };
///
/// let cmd = Command::new("A", CommandBody::login("alice", "pass").unwrap()).unwrap();
///
/// for fragment in CommandCodec::default().encode(&cmd) {
///     match fragment {
///         Fragment::Line { data } => {}
///         Fragment::Literal { data, mode } => {}
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Encoded {
    items: VecDeque<Fragment>,
}

impl Encoded {
    /// Dump the (remaining) encoded data without being guided by [`Fragment`]s.
    pub fn dump(self) -> Vec<u8> {
        let mut out = Vec::new();

        for fragment in self.items {
            match fragment {
                Fragment::Line { mut data } => out.append(&mut data),
                Fragment::Literal { mut data, .. } => out.append(&mut data),
            }
        }

        out
    }
}

impl Iterator for Encoded {
    type Item = Fragment;

    fn next(&mut self) -> Option<Self::Item> {
        self.items.pop_front()
    }
}

/// The intended action of a client or server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fragment {
    /// A line that is ready to be send.
    Line { data: Vec<u8> },

    /// A literal that may require an action before it should be send.
    Literal { data: Vec<u8>, mode: LiteralMode },
}

//--------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EncodeContext {
    accumulator: Vec<u8>,
    items: VecDeque<Fragment>,
}

impl EncodeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_line(&mut self) {
        self.items.push_back(Fragment::Line {
            data: std::mem::take(&mut self.accumulator),
        })
    }

    pub fn push_literal(&mut self, mode: LiteralMode) {
        self.items.push_back(Fragment::Literal {
            data: std::mem::take(&mut self.accumulator),
            mode,
        })
    }

    pub fn into_items(self) -> VecDeque<Fragment> {
        let Self {
            accumulator,
            mut items,
        } = self;

        if !accumulator.is_empty() {
            items.push_back(Fragment::Line { data: accumulator });
        }

        items
    }

    #[cfg(test)]
    pub(crate) fn dump(self) -> Vec<u8> {
        let mut out = Vec::new();

        for item in self.into_items() {
            match item {
                Fragment::Line { data } | Fragment::Literal { data, .. } => {
                    out.extend_from_slice(&data)
                }
            }
        }

        out
    }
}

impl Write for EncodeContext {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.accumulator.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

macro_rules! impl_encoder_for_codec {
    ($codec:ty, $message:ty) => {
        impl Encoder for $codec {
            type Message<'a> = $message;

            fn encode(&self, message: &Self::Message<'_>) -> Encoded {
                let mut encode_context = EncodeContext::new();
                EncodeIntoContext::encode_ctx(message.borrow(), &mut encode_context).unwrap();

                Encoded {
                    items: encode_context.into_items(),
                }
            }
        }
    };
}

impl_encoder_for_codec!(GreetingCodec, Greeting<'a>);
impl_encoder_for_codec!(CommandCodec, Command<'a>);
impl_encoder_for_codec!(AuthenticateDataCodec, AuthenticateData<'a>);
impl_encoder_for_codec!(ResponseCodec, Response<'a>);
impl_encoder_for_codec!(IdleDoneCodec, IdleDone);

// -------------------------------------------------------------------------------------------------

pub(crate) trait EncodeIntoContext {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()>;
}

// ----- Primitive ---------------------------------------------------------------------------------

impl EncodeIntoContext for u32 {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.to_string().as_bytes())
    }
}

impl EncodeIntoContext for u64 {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.to_string().as_bytes())
    }
}

// ----- Command -----------------------------------------------------------------------------------

impl EncodeIntoContext for Command<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.tag.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.body.encode_ctx(ctx)?;
        ctx.write_all(b"\r\n")
    }
}

impl EncodeIntoContext for Tag<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.inner().as_bytes())
    }
}

impl EncodeIntoContext for CommandBody<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            CommandBody::Capability => ctx.write_all(b"CAPABILITY"),
            CommandBody::Noop => ctx.write_all(b"NOOP"),
            CommandBody::Logout => ctx.write_all(b"LOGOUT"),
            #[cfg(feature = "starttls")]
            CommandBody::StartTLS => ctx.write_all(b"STARTTLS"),
            CommandBody::Authenticate {
                mechanism,
                initial_response,
            } => {
                ctx.write_all(b"AUTHENTICATE ")?;
                mechanism.encode_ctx(ctx)?;

                if let Some(ir) = initial_response {
                    ctx.write_all(b" ")?;

                    // RFC 4959 (https://datatracker.ietf.org/doc/html/rfc4959#section-3)
                    // "To send a zero-length initial response, the client MUST send a single pad character ("=").
                    // This indicates that the response is present, but is a zero-length string."
                    if ir.declassify().is_empty() {
                        ctx.write_all(b"=")?;
                    } else {
                        ctx.write_all(base64.encode(ir.declassify()).as_bytes())?;
                    };
                };

                Ok(())
            }
            CommandBody::Login { username, password } => {
                ctx.write_all(b"LOGIN ")?;
                username.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                password.declassify().encode_ctx(ctx)
            }
            CommandBody::Select {
                mailbox,
                #[cfg(feature = "ext_condstore_qresync")]
                parameters,
            } => {
                ctx.write_all(b"SELECT ")?;
                mailbox.encode_ctx(ctx)?;

                #[cfg(feature = "ext_condstore_qresync")]
                if !parameters.is_empty() {
                    ctx.write_all(b" (")?;
                    join_serializable(parameters, b" ", ctx)?;
                    ctx.write_all(b")")?;
                }

                Ok(())
            }
            CommandBody::Unselect => ctx.write_all(b"UNSELECT"),
            CommandBody::Examine {
                mailbox,
                #[cfg(feature = "ext_condstore_qresync")]
                parameters,
            } => {
                ctx.write_all(b"EXAMINE ")?;
                mailbox.encode_ctx(ctx)?;

                #[cfg(feature = "ext_condstore_qresync")]
                if !parameters.is_empty() {
                    ctx.write_all(b" (")?;
                    join_serializable(parameters, b" ", ctx)?;
                    ctx.write_all(b")")?;
                }

                Ok(())
            }
            CommandBody::Create { mailbox } => {
                ctx.write_all(b"CREATE ")?;
                mailbox.encode_ctx(ctx)
            }
            CommandBody::Delete { mailbox } => {
                ctx.write_all(b"DELETE ")?;
                mailbox.encode_ctx(ctx)
            }
            CommandBody::Rename {
                from: mailbox,
                to: new_mailbox,
            } => {
                ctx.write_all(b"RENAME ")?;
                mailbox.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                new_mailbox.encode_ctx(ctx)
            }
            CommandBody::Subscribe { mailbox } => {
                ctx.write_all(b"SUBSCRIBE ")?;
                mailbox.encode_ctx(ctx)
            }
            CommandBody::Unsubscribe { mailbox } => {
                ctx.write_all(b"UNSUBSCRIBE ")?;
                mailbox.encode_ctx(ctx)
            }
            CommandBody::List {
                reference,
                mailbox_wildcard,
            } => {
                ctx.write_all(b"LIST ")?;
                reference.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                mailbox_wildcard.encode_ctx(ctx)
            }
            CommandBody::Lsub {
                reference,
                mailbox_wildcard,
            } => {
                ctx.write_all(b"LSUB ")?;
                reference.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                mailbox_wildcard.encode_ctx(ctx)
            }
            CommandBody::Status {
                mailbox,
                item_names,
            } => {
                ctx.write_all(b"STATUS ")?;
                mailbox.encode_ctx(ctx)?;
                ctx.write_all(b" (")?;
                join_serializable(item_names, b" ", ctx)?;
                ctx.write_all(b")")
            }
            CommandBody::Append {
                mailbox,
                flags,
                date,
                message,
            } => {
                ctx.write_all(b"APPEND ")?;
                mailbox.encode_ctx(ctx)?;

                if !flags.is_empty() {
                    ctx.write_all(b" (")?;
                    join_serializable(flags, b" ", ctx)?;
                    ctx.write_all(b")")?;
                }

                if let Some(date) = date {
                    ctx.write_all(b" ")?;
                    date.encode_ctx(ctx)?;
                }

                ctx.write_all(b" ")?;
                message.encode_ctx(ctx)
            }
            CommandBody::Check => ctx.write_all(b"CHECK"),
            CommandBody::Close => ctx.write_all(b"CLOSE"),
            CommandBody::Expunge => ctx.write_all(b"EXPUNGE"),
            CommandBody::ExpungeUid { sequence_set } => {
                ctx.write_all(b"UID EXPUNGE ")?;
                sequence_set.encode_ctx(ctx)
            }
            CommandBody::Search {
                charset,
                criteria,
                uid,
            } => {
                if *uid {
                    ctx.write_all(b"UID SEARCH")?;
                } else {
                    ctx.write_all(b"SEARCH")?;
                }
                if let Some(charset) = charset {
                    ctx.write_all(b" CHARSET ")?;
                    charset.encode_ctx(ctx)?;
                }
                ctx.write_all(b" ")?;
                join_serializable(criteria.as_ref(), b" ", ctx)
            }
            CommandBody::Sort {
                sort_criteria,
                charset,
                search_criteria,
                uid,
            } => {
                if *uid {
                    ctx.write_all(b"UID SORT (")?;
                } else {
                    ctx.write_all(b"SORT (")?;
                }
                join_serializable(sort_criteria.as_ref(), b" ", ctx)?;
                ctx.write_all(b") ")?;
                charset.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                join_serializable(search_criteria.as_ref(), b" ", ctx)
            }
            CommandBody::Thread {
                algorithm,
                charset,
                search_criteria,
                uid,
            } => {
                if *uid {
                    ctx.write_all(b"UID THREAD ")?;
                } else {
                    ctx.write_all(b"THREAD ")?;
                }
                algorithm.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                charset.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                join_serializable(search_criteria.as_ref(), b" ", ctx)
            }
            CommandBody::Fetch {
                sequence_set,
                macro_or_item_names,
                uid,
                #[cfg(feature = "ext_condstore_qresync")]
                modifiers,
            } => {
                if *uid {
                    ctx.write_all(b"UID FETCH ")?;
                } else {
                    ctx.write_all(b"FETCH ")?;
                }

                sequence_set.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                macro_or_item_names.encode_ctx(ctx)?;

                #[cfg(feature = "ext_condstore_qresync")]
                if !modifiers.is_empty() {
                    ctx.write_all(b" (")?;
                    join_serializable(modifiers, b" ", ctx)?;
                    ctx.write_all(b")")?;
                }

                Ok(())
            }
            CommandBody::Store {
                sequence_set,
                kind,
                response,
                flags,
                uid,
                #[cfg(feature = "ext_condstore_qresync")]
                modifiers,
            } => {
                if *uid {
                    ctx.write_all(b"UID STORE ")?;
                } else {
                    ctx.write_all(b"STORE ")?;
                }

                sequence_set.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;

                #[cfg(feature = "ext_condstore_qresync")]
                if !modifiers.is_empty() {
                    ctx.write_all(b"(")?;
                    join_serializable(modifiers, b" ", ctx)?;
                    ctx.write_all(b") ")?;
                }

                match kind {
                    StoreType::Add => ctx.write_all(b"+")?,
                    StoreType::Remove => ctx.write_all(b"-")?,
                    StoreType::Replace => {}
                }

                ctx.write_all(b"FLAGS")?;

                match response {
                    StoreResponse::Answer => {}
                    StoreResponse::Silent => ctx.write_all(b".SILENT")?,
                }

                ctx.write_all(b" (")?;
                join_serializable(flags, b" ", ctx)?;
                ctx.write_all(b")")
            }
            CommandBody::Copy {
                sequence_set,
                mailbox,
                uid,
            } => {
                if *uid {
                    ctx.write_all(b"UID COPY ")?;
                } else {
                    ctx.write_all(b"COPY ")?;
                }
                sequence_set.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                mailbox.encode_ctx(ctx)
            }
            CommandBody::Idle => ctx.write_all(b"IDLE"),
            CommandBody::Enable { capabilities } => {
                ctx.write_all(b"ENABLE ")?;
                join_serializable(capabilities.as_ref(), b" ", ctx)
            }
            CommandBody::Compress { algorithm } => {
                ctx.write_all(b"COMPRESS ")?;
                algorithm.encode_ctx(ctx)
            }
            CommandBody::GetQuota { root } => {
                ctx.write_all(b"GETQUOTA ")?;
                root.encode_ctx(ctx)
            }
            CommandBody::GetQuotaRoot { mailbox } => {
                ctx.write_all(b"GETQUOTAROOT ")?;
                mailbox.encode_ctx(ctx)
            }
            CommandBody::SetQuota { root, quotas } => {
                ctx.write_all(b"SETQUOTA ")?;
                root.encode_ctx(ctx)?;
                ctx.write_all(b" (")?;
                join_serializable(quotas.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")
            }
            CommandBody::Move {
                sequence_set,
                mailbox,
                uid,
            } => {
                if *uid {
                    ctx.write_all(b"UID MOVE ")?;
                } else {
                    ctx.write_all(b"MOVE ")?;
                }
                sequence_set.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                mailbox.encode_ctx(ctx)
            }
            #[cfg(feature = "ext_id")]
            CommandBody::Id { parameters } => {
                ctx.write_all(b"ID ")?;

                match parameters {
                    Some(parameters) => {
                        if let Some((first, tail)) = parameters.split_first() {
                            ctx.write_all(b"(")?;

                            first.0.encode_ctx(ctx)?;
                            ctx.write_all(b" ")?;
                            first.1.encode_ctx(ctx)?;

                            for parameter in tail {
                                ctx.write_all(b" ")?;
                                parameter.0.encode_ctx(ctx)?;
                                ctx.write_all(b" ")?;
                                parameter.1.encode_ctx(ctx)?;
                            }

                            ctx.write_all(b")")
                        } else {
                            #[cfg(not(feature = "quirk_id_empty_to_nil"))]
                            {
                                ctx.write_all(b"()")
                            }
                            #[cfg(feature = "quirk_id_empty_to_nil")]
                            {
                                ctx.write_all(b"NIL")
                            }
                        }
                    }
                    None => ctx.write_all(b"NIL"),
                }
            }
            #[cfg(feature = "ext_metadata")]
            CommandBody::SetMetadata {
                mailbox,
                entry_values,
            } => {
                ctx.write_all(b"SETMETADATA ")?;
                mailbox.encode_ctx(ctx)?;
                ctx.write_all(b" (")?;
                join_serializable(entry_values.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")
            }
            #[cfg(feature = "ext_metadata")]
            CommandBody::GetMetadata {
                options,
                mailbox,
                entries,
            } => {
                ctx.write_all(b"GETMETADATA")?;

                if !options.is_empty() {
                    ctx.write_all(b" (")?;
                    join_serializable(options, b" ", ctx)?;
                    ctx.write_all(b")")?;
                }

                ctx.write_all(b" ")?;
                mailbox.encode_ctx(ctx)?;

                ctx.write_all(b" ")?;

                if entries.as_ref().len() == 1 {
                    entries.as_ref()[0].encode_ctx(ctx)
                } else {
                    ctx.write_all(b"(")?;
                    join_serializable(entries.as_ref(), b" ", ctx)?;
                    ctx.write_all(b")")
                }
            }
            #[cfg(feature = "ext_namespace")]
            &CommandBody::Namespace => ctx.write_all(b"NAMESPACE"),
        }
    }
}

#[cfg(feature = "ext_condstore_qresync")]
impl EncodeIntoContext for FetchModifier {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            FetchModifier::ChangedSince(since) => write!(ctx, "CHANGEDSINCE {since}"),
            FetchModifier::Vanished => write!(ctx, "VANISHED"),
        }
    }
}

#[cfg(feature = "ext_condstore_qresync")]
impl EncodeIntoContext for StoreModifier {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            StoreModifier::UnchangedSince(since) => write!(ctx, "UNCHANGEDSINCE {since}"),
        }
    }
}

#[cfg(feature = "ext_condstore_qresync")]
impl EncodeIntoContext for SelectParameter {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            SelectParameter::CondStore => write!(ctx, "CONDSTORE"),
            SelectParameter::QResync {
                uid_validity,
                mod_sequence_value,
                known_uids,
                seq_match_data,
            } => {
                write!(ctx, "QRESYNC (")?;
                uid_validity.encode_ctx(ctx)?;
                write!(ctx, " ")?;
                mod_sequence_value.encode_ctx(ctx)?;

                if let Some(known_uids) = known_uids {
                    write!(ctx, " ")?;
                    known_uids.encode_ctx(ctx)?;
                }

                if let Some((known_sequence_set, known_uid_set)) = seq_match_data {
                    write!(ctx, " (")?;
                    known_sequence_set.encode_ctx(ctx)?;
                    write!(ctx, " ")?;
                    known_uid_set.encode_ctx(ctx)?;
                    write!(ctx, ")")?;
                }

                write!(ctx, ")")
            }
        }
    }
}

impl EncodeIntoContext for AuthMechanism<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

impl EncodeIntoContext for AuthenticateData<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Continue(data) => {
                let encoded = base64.encode(data.declassify());
                ctx.write_all(encoded.as_bytes())?;
                ctx.write_all(b"\r\n")
            }
            Self::Cancel => ctx.write_all(b"*\r\n"),
        }
    }
}

impl EncodeIntoContext for AString<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            AString::Atom(atom) => atom.encode_ctx(ctx),
            AString::String(imap_str) => imap_str.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for Atom<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.inner().as_bytes())
    }
}

impl EncodeIntoContext for AtomExt<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.inner().as_bytes())
    }
}

impl EncodeIntoContext for IString<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Literal(val) => val.encode_ctx(ctx),
            Self::Quoted(val) => val.encode_ctx(ctx),
            #[cfg(feature = "ext_utf8")]
            Self::QuotedUtf8(val) => val.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for Literal<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self.mode() {
            LiteralMode::Sync => write!(ctx, "{{{}}}\r\n", self.as_ref().len())?,
            LiteralMode::NonSync => write!(ctx, "{{{}+}}\r\n", self.as_ref().len())?,
        }

        ctx.push_line();
        ctx.write_all(self.as_ref())?;
        ctx.push_literal(self.mode());

        Ok(())
    }
}

impl EncodeIntoContext for Quoted<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "\"{}\"", escape_quoted(self.inner()))
    }
}

impl EncodeIntoContext for Mailbox<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Mailbox::Inbox => ctx.write_all(b"INBOX"),
            Mailbox::Other(other) => other.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for MailboxOther<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.inner().encode_ctx(ctx)
    }
}

impl EncodeIntoContext for ListMailbox<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            ListMailbox::Token(lcs) => lcs.encode_ctx(ctx),
            ListMailbox::String(istr) => istr.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for ListCharString<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.as_ref())
    }
}

impl EncodeIntoContext for StatusDataItemName {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Messages => ctx.write_all(b"MESSAGES"),
            Self::Recent => ctx.write_all(b"RECENT"),
            Self::UidNext => ctx.write_all(b"UIDNEXT"),
            Self::UidValidity => ctx.write_all(b"UIDVALIDITY"),
            Self::Unseen => ctx.write_all(b"UNSEEN"),
            Self::Deleted => ctx.write_all(b"DELETED"),
            Self::DeletedStorage => ctx.write_all(b"DELETED-STORAGE"),
            #[cfg(feature = "ext_condstore_qresync")]
            Self::HighestModSeq => ctx.write_all(b"HIGHESTMODSEQ"),
        }
    }
}

impl EncodeIntoContext for Flag<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

impl EncodeIntoContext for FlagFetch<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Flag(flag) => flag.encode_ctx(ctx),
            Self::Recent => ctx.write_all(b"\\Recent"),
        }
    }
}

impl EncodeIntoContext for FlagPerm<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Flag(flag) => flag.encode_ctx(ctx),
            Self::Asterisk => ctx.write_all(b"\\*"),
        }
    }
}

impl EncodeIntoContext for DateTime {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.as_ref().encode_ctx(ctx)
    }
}

impl EncodeIntoContext for Charset<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Charset::Atom(atom) => atom.encode_ctx(ctx),
            Charset::Quoted(quoted) => quoted.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for SearchKey<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            SearchKey::All => ctx.write_all(b"ALL"),
            SearchKey::Answered => ctx.write_all(b"ANSWERED"),
            SearchKey::Bcc(astring) => {
                ctx.write_all(b"BCC ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::Before(date) => {
                ctx.write_all(b"BEFORE ")?;
                date.encode_ctx(ctx)
            }
            SearchKey::Body(astring) => {
                ctx.write_all(b"BODY ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::Cc(astring) => {
                ctx.write_all(b"CC ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::Deleted => ctx.write_all(b"DELETED"),
            SearchKey::Flagged => ctx.write_all(b"FLAGGED"),
            SearchKey::From(astring) => {
                ctx.write_all(b"FROM ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::Keyword(flag_keyword) => {
                ctx.write_all(b"KEYWORD ")?;
                flag_keyword.encode_ctx(ctx)
            }
            SearchKey::New => ctx.write_all(b"NEW"),
            SearchKey::Old => ctx.write_all(b"OLD"),
            SearchKey::On(date) => {
                ctx.write_all(b"ON ")?;
                date.encode_ctx(ctx)
            }
            SearchKey::Recent => ctx.write_all(b"RECENT"),
            SearchKey::Seen => ctx.write_all(b"SEEN"),
            SearchKey::Since(date) => {
                ctx.write_all(b"SINCE ")?;
                date.encode_ctx(ctx)
            }
            SearchKey::Subject(astring) => {
                ctx.write_all(b"SUBJECT ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::Text(astring) => {
                ctx.write_all(b"TEXT ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::To(astring) => {
                ctx.write_all(b"TO ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::Unanswered => ctx.write_all(b"UNANSWERED"),
            SearchKey::Undeleted => ctx.write_all(b"UNDELETED"),
            SearchKey::Unflagged => ctx.write_all(b"UNFLAGGED"),
            SearchKey::Unkeyword(flag_keyword) => {
                ctx.write_all(b"UNKEYWORD ")?;
                flag_keyword.encode_ctx(ctx)
            }
            SearchKey::Unseen => ctx.write_all(b"UNSEEN"),
            SearchKey::Draft => ctx.write_all(b"DRAFT"),
            SearchKey::Header(header_fld_name, astring) => {
                ctx.write_all(b"HEADER ")?;
                header_fld_name.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                astring.encode_ctx(ctx)
            }
            SearchKey::Larger(number) => write!(ctx, "LARGER {number}"),
            SearchKey::Not(search_key) => {
                ctx.write_all(b"NOT ")?;
                search_key.encode_ctx(ctx)
            }
            SearchKey::Or(search_key_a, search_key_b) => {
                ctx.write_all(b"OR ")?;
                search_key_a.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                search_key_b.encode_ctx(ctx)
            }
            SearchKey::SentBefore(date) => {
                ctx.write_all(b"SENTBEFORE ")?;
                date.encode_ctx(ctx)
            }
            SearchKey::SentOn(date) => {
                ctx.write_all(b"SENTON ")?;
                date.encode_ctx(ctx)
            }
            SearchKey::SentSince(date) => {
                ctx.write_all(b"SENTSINCE ")?;
                date.encode_ctx(ctx)
            }
            SearchKey::Smaller(number) => write!(ctx, "SMALLER {number}"),
            SearchKey::Uid(sequence_set) => {
                ctx.write_all(b"UID ")?;
                sequence_set.encode_ctx(ctx)
            }
            SearchKey::Undraft => ctx.write_all(b"UNDRAFT"),
            #[cfg(feature = "ext_condstore_qresync")]
            SearchKey::ModSequence { entry, modseq } => {
                ctx.write_all(b"MODSEQ")?;
                if let Some((attribute_flag, entry_type_req)) = entry {
                    write!(ctx, " \"/flags/{attribute_flag}\"")?;
                    write!(ctx, " {entry_type_req}")?;
                }
                ctx.write_all(b" ")?;
                modseq.encode_ctx(ctx)
            }
            SearchKey::SequenceSet(sequence_set) => sequence_set.encode_ctx(ctx),
            SearchKey::And(search_keys) => {
                ctx.write_all(b"(")?;
                join_serializable(search_keys.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")
            }
        }
    }
}

impl EncodeIntoContext for SequenceSet {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        #[cfg(feature = "quirk_always_normalize_sequence_sets")]
        let mut set = self.clone();
        #[cfg(feature = "quirk_always_normalize_sequence_sets")]
        set.normalize();

        #[cfg(not(feature = "quirk_always_normalize_sequence_sets"))]
        let set = self;

        join_serializable(set.0.as_ref(), b",", ctx)
    }
}

impl EncodeIntoContext for Sequence {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        #[cfg(feature = "quirk_always_normalize_sequence_sets")]
        let mut seq = self.clone();
        #[cfg(feature = "quirk_always_normalize_sequence_sets")]
        seq.normalize();

        #[cfg(not(feature = "quirk_always_normalize_sequence_sets"))]
        let seq = self;

        match seq {
            Sequence::Single(seq_no) => seq_no.encode_ctx(ctx),
            Sequence::Range(from, to) => {
                from.encode_ctx(ctx)?;
                ctx.write_all(b":")?;
                to.encode_ctx(ctx)
            }
        }
    }
}

impl EncodeIntoContext for SeqOrUid {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            SeqOrUid::Value(number) => write!(ctx, "{number}"),
            SeqOrUid::Asterisk => ctx.write_all(b"*"),
        }
    }
}

impl EncodeIntoContext for NaiveDate {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "\"{}\"", self.as_ref().format("%d-%b-%Y"))
    }
}

impl EncodeIntoContext for MacroOrMessageDataItemNames<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Macro(m) => m.encode_ctx(ctx),
            Self::MessageDataItemNames(item_names) => {
                if item_names.len() == 1 {
                    item_names[0].encode_ctx(ctx)
                } else {
                    ctx.write_all(b"(")?;
                    join_serializable(item_names.as_slice(), b" ", ctx)?;
                    ctx.write_all(b")")
                }
            }
        }
    }
}

impl EncodeIntoContext for Macro {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

impl EncodeIntoContext for MessageDataItemName<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Body => ctx.write_all(b"BODY"),
            Self::BodyExt {
                section,
                partial,
                peek,
            } => {
                if *peek {
                    ctx.write_all(b"BODY.PEEK[")?;
                } else {
                    ctx.write_all(b"BODY[")?;
                }
                if let Some(section) = section {
                    section.encode_ctx(ctx)?;
                }
                ctx.write_all(b"]")?;
                if let Some((a, b)) = partial {
                    write!(ctx, "<{a}.{b}>")?;
                }

                Ok(())
            }
            Self::BodyStructure => ctx.write_all(b"BODYSTRUCTURE"),
            Self::Envelope => ctx.write_all(b"ENVELOPE"),
            Self::Flags => ctx.write_all(b"FLAGS"),
            Self::InternalDate => ctx.write_all(b"INTERNALDATE"),
            Self::Rfc822 => ctx.write_all(b"RFC822"),
            Self::Rfc822Header => ctx.write_all(b"RFC822.HEADER"),
            Self::Rfc822Size => ctx.write_all(b"RFC822.SIZE"),
            Self::Rfc822Text => ctx.write_all(b"RFC822.TEXT"),
            Self::Uid => ctx.write_all(b"UID"),
            MessageDataItemName::Binary {
                section,
                partial,
                peek,
            } => {
                ctx.write_all(b"BINARY")?;
                if *peek {
                    ctx.write_all(b".PEEK")?;
                }

                ctx.write_all(b"[")?;
                join_serializable(section, b".", ctx)?;
                ctx.write_all(b"]")?;

                if let Some((a, b)) = partial {
                    ctx.write_all(b"<")?;
                    a.encode_ctx(ctx)?;
                    ctx.write_all(b".")?;
                    b.encode_ctx(ctx)?;
                    ctx.write_all(b">")?;
                }

                Ok(())
            }
            MessageDataItemName::BinarySize { section } => {
                ctx.write_all(b"BINARY.SIZE")?;

                ctx.write_all(b"[")?;
                join_serializable(section, b".", ctx)?;
                ctx.write_all(b"]")
            }
            #[cfg(feature = "ext_condstore_qresync")]
            MessageDataItemName::ModSeq => ctx.write_all(b"MODSEQ"),
        }
    }
}

impl EncodeIntoContext for Section<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Section::Part(part) => part.encode_ctx(ctx),
            Section::Header(maybe_part) => match maybe_part {
                Some(part) => {
                    part.encode_ctx(ctx)?;
                    ctx.write_all(b".HEADER")
                }
                None => ctx.write_all(b"HEADER"),
            },
            Section::HeaderFields(maybe_part, header_list) => {
                match maybe_part {
                    Some(part) => {
                        part.encode_ctx(ctx)?;
                        ctx.write_all(b".HEADER.FIELDS (")?;
                    }
                    None => ctx.write_all(b"HEADER.FIELDS (")?,
                };
                join_serializable(header_list.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")
            }
            Section::HeaderFieldsNot(maybe_part, header_list) => {
                match maybe_part {
                    Some(part) => {
                        part.encode_ctx(ctx)?;
                        ctx.write_all(b".HEADER.FIELDS.NOT (")?;
                    }
                    None => ctx.write_all(b"HEADER.FIELDS.NOT (")?,
                };
                join_serializable(header_list.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")
            }
            Section::Text(maybe_part) => match maybe_part {
                Some(part) => {
                    part.encode_ctx(ctx)?;
                    ctx.write_all(b".TEXT")
                }
                None => ctx.write_all(b"TEXT"),
            },
            Section::Mime(part) => {
                part.encode_ctx(ctx)?;
                ctx.write_all(b".MIME")
            }
        }
    }
}

impl EncodeIntoContext for Part {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        join_serializable(self.0.as_ref(), b".", ctx)
    }
}

impl EncodeIntoContext for NonZeroU32 {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

#[cfg(feature = "ext_condstore_qresync")]
impl EncodeIntoContext for NonZeroU64 {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

impl EncodeIntoContext for Capability<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

// ----- Responses ---------------------------------------------------------------------------------

impl EncodeIntoContext for Response<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Response::Status(status) => status.encode_ctx(ctx),
            Response::Data(data) => data.encode_ctx(ctx),
            Response::CommandContinuationRequest(continue_request) => {
                continue_request.encode_ctx(ctx)
            }
        }
    }
}

impl EncodeIntoContext for Greeting<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(b"* ")?;
        self.kind.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;

        if let Some(ref code) = self.code {
            ctx.write_all(b"[")?;
            code.encode_ctx(ctx)?;
            ctx.write_all(b"] ")?;
        }

        self.text.encode_ctx(ctx)?;
        ctx.write_all(b"\r\n")
    }
}

impl EncodeIntoContext for GreetingKind {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            GreetingKind::Ok => ctx.write_all(b"OK"),
            GreetingKind::PreAuth => ctx.write_all(b"PREAUTH"),
            GreetingKind::Bye => ctx.write_all(b"BYE"),
        }
    }
}

impl EncodeIntoContext for Status<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        fn format_status(
            tag: Option<&Tag>,
            status: &str,
            code: &Option<Code>,
            comment: &Text,
            ctx: &mut EncodeContext,
        ) -> std::io::Result<()> {
            match tag {
                Some(tag) => tag.encode_ctx(ctx)?,
                None => ctx.write_all(b"*")?,
            }
            ctx.write_all(b" ")?;
            ctx.write_all(status.as_bytes())?;
            ctx.write_all(b" ")?;
            if let Some(code) = code {
                ctx.write_all(b"[")?;
                code.encode_ctx(ctx)?;
                ctx.write_all(b"] ")?;
            }
            comment.encode_ctx(ctx)?;
            ctx.write_all(b"\r\n")
        }

        match self {
            Self::Untagged(StatusBody { kind, code, text }) => match kind {
                StatusKind::Ok => format_status(None, "OK", code, text, ctx),
                StatusKind::No => format_status(None, "NO", code, text, ctx),
                StatusKind::Bad => format_status(None, "BAD", code, text, ctx),
            },
            Self::Tagged(Tagged {
                tag,
                body: StatusBody { kind, code, text },
            }) => match kind {
                StatusKind::Ok => format_status(Some(tag), "OK", code, text, ctx),
                StatusKind::No => format_status(Some(tag), "NO", code, text, ctx),
                StatusKind::Bad => format_status(Some(tag), "BAD", code, text, ctx),
            },
            Self::Bye(Bye { code, text }) => format_status(None, "BYE", code, text, ctx),
        }
    }
}

impl EncodeIntoContext for Code<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Code::Alert => ctx.write_all(b"ALERT"),
            Code::BadCharset { allowed } => {
                if allowed.is_empty() {
                    ctx.write_all(b"BADCHARSET")
                } else {
                    ctx.write_all(b"BADCHARSET (")?;
                    join_serializable(allowed, b" ", ctx)?;
                    ctx.write_all(b")")
                }
            }
            Code::Capability(caps) => {
                ctx.write_all(b"CAPABILITY ")?;
                join_serializable(caps.as_ref(), b" ", ctx)
            }
            Code::Parse => ctx.write_all(b"PARSE"),
            Code::PermanentFlags(flags) => {
                ctx.write_all(b"PERMANENTFLAGS (")?;
                join_serializable(flags, b" ", ctx)?;
                ctx.write_all(b")")
            }
            Code::ReadOnly => ctx.write_all(b"READ-ONLY"),
            Code::ReadWrite => ctx.write_all(b"READ-WRITE"),
            Code::TryCreate => ctx.write_all(b"TRYCREATE"),
            Code::UidNext(next) => {
                ctx.write_all(b"UIDNEXT ")?;
                next.encode_ctx(ctx)
            }
            Code::UidValidity(validity) => {
                ctx.write_all(b"UIDVALIDITY ")?;
                validity.encode_ctx(ctx)
            }
            Code::Unseen(seq) => {
                ctx.write_all(b"UNSEEN ")?;
                seq.encode_ctx(ctx)
            }
            // RFC 2221
            #[cfg(any(feature = "ext_login_referrals", feature = "ext_mailbox_referrals"))]
            Code::Referral(url) => {
                ctx.write_all(b"REFERRAL ")?;
                ctx.write_all(url.as_bytes())
            }
            // RFC 4551
            #[cfg(feature = "ext_condstore_qresync")]
            Code::HighestModSeq(modseq) => {
                ctx.write_all(b"HIGHESTMODSEQ ")?;
                modseq.encode_ctx(ctx)
            }
            #[cfg(feature = "ext_condstore_qresync")]
            Code::NoModSeq => ctx.write_all(b"NOMODSEQ"),
            #[cfg(feature = "ext_condstore_qresync")]
            Code::Modified(sequence_set) => {
                ctx.write_all(b"MODIFIED ")?;
                sequence_set.encode_ctx(ctx)
            }
            #[cfg(feature = "ext_condstore_qresync")]
            Code::Closed => ctx.write_all(b"CLOSED"),
            Code::CompressionActive => ctx.write_all(b"COMPRESSIONACTIVE"),
            Code::OverQuota => ctx.write_all(b"OVERQUOTA"),
            Code::TooBig => ctx.write_all(b"TOOBIG"),
            #[cfg(feature = "ext_metadata")]
            Code::Metadata(code) => {
                ctx.write_all(b"METADATA ")?;
                code.encode_ctx(ctx)
            }
            Code::UnknownCte => ctx.write_all(b"UNKNOWN-CTE"),
            Code::AppendUid { uid_validity, uid } => {
                ctx.write_all(b"APPENDUID ")?;
                uid_validity.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                uid.encode_ctx(ctx)
            }
            Code::CopyUid {
                uid_validity,
                source,
                destination,
            } => {
                ctx.write_all(b"COPYUID ")?;
                uid_validity.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                source.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                destination.encode_ctx(ctx)
            }
            Code::UidNotSticky => ctx.write_all(b"UIDNOTSTICKY"),
            Code::Other(unknown) => unknown.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for CodeOther<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.inner())
    }
}

impl EncodeIntoContext for Text<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.inner().as_bytes())
    }
}

impl EncodeIntoContext for Data<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Data::Capability(caps) => {
                ctx.write_all(b"* CAPABILITY ")?;
                join_serializable(caps.as_ref(), b" ", ctx)?;
            }
            Data::List {
                items,
                delimiter,
                mailbox,
            } => {
                ctx.write_all(b"* LIST (")?;
                join_serializable(items, b" ", ctx)?;
                ctx.write_all(b") ")?;

                if let Some(delimiter) = delimiter {
                    ctx.write_all(b"\"")?;
                    delimiter.encode_ctx(ctx)?;
                    ctx.write_all(b"\"")?;
                } else {
                    ctx.write_all(b"NIL")?;
                }
                ctx.write_all(b" ")?;
                mailbox.encode_ctx(ctx)?;
            }
            Data::Lsub {
                items,
                delimiter,
                mailbox,
            } => {
                ctx.write_all(b"* LSUB (")?;
                join_serializable(items, b" ", ctx)?;
                ctx.write_all(b") ")?;

                if let Some(delimiter) = delimiter {
                    ctx.write_all(b"\"")?;
                    delimiter.encode_ctx(ctx)?;
                    ctx.write_all(b"\"")?;
                } else {
                    ctx.write_all(b"NIL")?;
                }
                ctx.write_all(b" ")?;
                mailbox.encode_ctx(ctx)?;
            }
            Data::Status { mailbox, items } => {
                ctx.write_all(b"* STATUS ")?;
                mailbox.encode_ctx(ctx)?;
                ctx.write_all(b" (")?;
                join_serializable(items, b" ", ctx)?;
                ctx.write_all(b")")?;
            }
            // TODO: Exclude pattern via cfg?
            #[cfg(not(feature = "ext_condstore_qresync"))]
            Data::Search(seqs) => {
                if seqs.is_empty() {
                    ctx.write_all(b"* SEARCH")?;
                } else {
                    ctx.write_all(b"* SEARCH ")?;
                    join_serializable(seqs, b" ", ctx)?;
                }
            }
            // TODO: Exclude pattern via cfg?
            #[cfg(feature = "ext_condstore_qresync")]
            Data::Search(seqs, modseq) => {
                if seqs.is_empty() {
                    ctx.write_all(b"* SEARCH")?;
                } else {
                    ctx.write_all(b"* SEARCH ")?;
                    join_serializable(seqs, b" ", ctx)?;
                }

                if let Some(modseq) = modseq {
                    ctx.write_all(b" (MODSEQ ")?;
                    modseq.encode_ctx(ctx)?;
                    ctx.write_all(b")")?;
                }
            }
            // TODO: Exclude pattern via cfg?
            #[cfg(not(feature = "ext_condstore_qresync"))]
            Data::Sort(seqs) => {
                if seqs.is_empty() {
                    ctx.write_all(b"* SORT")?;
                } else {
                    ctx.write_all(b"* SORT ")?;
                    join_serializable(seqs, b" ", ctx)?;
                }
            }
            // TODO: Exclude pattern via cfg?
            #[cfg(feature = "ext_condstore_qresync")]
            Data::Sort(seqs, modseq) => {
                if seqs.is_empty() {
                    ctx.write_all(b"* SORT")?;
                } else {
                    ctx.write_all(b"* SORT ")?;
                    join_serializable(seqs, b" ", ctx)?;
                }

                if let Some(modseq) = modseq {
                    ctx.write_all(b" (MODSEQ ")?;
                    modseq.encode_ctx(ctx)?;
                    ctx.write_all(b")")?;
                }
            }
            Data::Thread(threads) => {
                if threads.is_empty() {
                    ctx.write_all(b"* THREAD")?;
                } else {
                    ctx.write_all(b"* THREAD ")?;
                    for thread in threads {
                        thread.encode_ctx(ctx)?;
                    }
                }
            }
            Data::Flags(flags) => {
                ctx.write_all(b"* FLAGS (")?;
                join_serializable(flags, b" ", ctx)?;
                ctx.write_all(b")")?;
            }
            Data::Exists(count) => write!(ctx, "* {count} EXISTS")?,
            Data::Recent(count) => write!(ctx, "* {count} RECENT")?,
            Data::Expunge(msg) => write!(ctx, "* {msg} EXPUNGE")?,
            Data::Fetch { seq, items } => {
                write!(ctx, "* {seq} FETCH (")?;
                join_serializable(items.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")?;
            }
            Data::Enabled { capabilities } => {
                write!(ctx, "* ENABLED")?;

                for cap in capabilities {
                    ctx.write_all(b" ")?;
                    cap.encode_ctx(ctx)?;
                }
            }
            Data::Quota { root, quotas } => {
                ctx.write_all(b"* QUOTA ")?;
                root.encode_ctx(ctx)?;
                ctx.write_all(b" (")?;
                join_serializable(quotas.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")?;
            }
            Data::QuotaRoot { mailbox, roots } => {
                ctx.write_all(b"* QUOTAROOT ")?;
                mailbox.encode_ctx(ctx)?;
                for root in roots {
                    ctx.write_all(b" ")?;
                    root.encode_ctx(ctx)?;
                }
            }
            #[cfg(feature = "ext_id")]
            Data::Id { parameters } => {
                ctx.write_all(b"* ID ")?;

                match parameters {
                    Some(parameters) => {
                        if let Some((first, tail)) = parameters.split_first() {
                            ctx.write_all(b"(")?;

                            first.0.encode_ctx(ctx)?;
                            ctx.write_all(b" ")?;
                            first.1.encode_ctx(ctx)?;

                            for parameter in tail {
                                ctx.write_all(b" ")?;
                                parameter.0.encode_ctx(ctx)?;
                                ctx.write_all(b" ")?;
                                parameter.1.encode_ctx(ctx)?;
                            }

                            ctx.write_all(b")")?;
                        } else {
                            #[cfg(not(feature = "quirk_id_empty_to_nil"))]
                            {
                                ctx.write_all(b"()")?;
                            }
                            #[cfg(feature = "quirk_id_empty_to_nil")]
                            {
                                ctx.write_all(b"NIL")?;
                            }
                        }
                    }
                    None => {
                        ctx.write_all(b"NIL")?;
                    }
                }
            }
            #[cfg(feature = "ext_metadata")]
            Data::Metadata { mailbox, items } => {
                ctx.write_all(b"* METADATA ")?;
                mailbox.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                items.encode_ctx(ctx)?;
            }
            #[cfg(feature = "ext_condstore_qresync")]
            Data::Vanished {
                earlier,
                known_uids,
            } => {
                ctx.write_all(b"* VANISHED")?;
                if *earlier {
                    ctx.write_all(b" (EARLIER)")?;
                }
                ctx.write_all(b" ")?;
                known_uids.encode_ctx(ctx)?;
            }
            #[cfg(feature = "ext_namespace")]
            Data::Namespace {
                personal,
                other,
                shared,
            } => {
                ctx.write_all(b"* NAMESPACE ")?;
                encode_namespaces(ctx, personal)?;
                ctx.write_all(b" ")?;
                encode_namespaces(ctx, other)?;
                ctx.write_all(b" ")?;
                encode_namespaces(ctx, shared)?;
            }
        }

        ctx.write_all(b"\r\n")
    }
}

impl EncodeIntoContext for FlagNameAttribute<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "{self}")
    }
}

impl EncodeIntoContext for QuotedChar {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self.inner() {
            '\\' => ctx.write_all(b"\\\\"),
            '"' => ctx.write_all(b"\\\""),
            other => ctx.write_all(&[other as u8]),
        }
    }
}

impl EncodeIntoContext for StatusDataItem {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Messages(count) => {
                ctx.write_all(b"MESSAGES ")?;
                count.encode_ctx(ctx)
            }
            Self::Recent(count) => {
                ctx.write_all(b"RECENT ")?;
                count.encode_ctx(ctx)
            }
            Self::UidNext(next) => {
                ctx.write_all(b"UIDNEXT ")?;
                next.encode_ctx(ctx)
            }
            Self::UidValidity(identifier) => {
                ctx.write_all(b"UIDVALIDITY ")?;
                identifier.encode_ctx(ctx)
            }
            Self::Unseen(count) => {
                ctx.write_all(b"UNSEEN ")?;
                count.encode_ctx(ctx)
            }
            Self::Deleted(count) => {
                ctx.write_all(b"DELETED ")?;
                count.encode_ctx(ctx)
            }
            Self::DeletedStorage(count) => {
                ctx.write_all(b"DELETED-STORAGE ")?;
                count.encode_ctx(ctx)
            }
            #[cfg(feature = "ext_condstore_qresync")]
            Self::HighestModSeq(value) => {
                ctx.write_all(b"HIGHESTMODSEQ ")?;
                value.encode_ctx(ctx)
            }
        }
    }
}

impl EncodeIntoContext for MessageDataItem<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::BodyExt {
                section,
                origin,
                data,
            } => {
                ctx.write_all(b"BODY[")?;
                if let Some(section) = section {
                    section.encode_ctx(ctx)?;
                }
                ctx.write_all(b"]")?;
                if let Some(origin) = origin {
                    write!(ctx, "<{origin}>")?;
                }
                ctx.write_all(b" ")?;
                data.encode_ctx(ctx)
            }
            // FIXME: do not return body-ext-1part and body-ext-mpart here
            Self::Body(body) => {
                ctx.write_all(b"BODY ")?;
                body.encode_ctx(ctx)
            }
            Self::BodyStructure(body) => {
                ctx.write_all(b"BODYSTRUCTURE ")?;
                body.encode_ctx(ctx)
            }
            Self::Envelope(envelope) => {
                ctx.write_all(b"ENVELOPE ")?;
                envelope.encode_ctx(ctx)
            }
            Self::Flags(flags) => {
                ctx.write_all(b"FLAGS (")?;
                join_serializable(flags, b" ", ctx)?;
                ctx.write_all(b")")
            }
            Self::InternalDate(datetime) => {
                ctx.write_all(b"INTERNALDATE ")?;
                datetime.encode_ctx(ctx)
            }
            Self::Rfc822(nstring) => {
                ctx.write_all(b"RFC822 ")?;
                nstring.encode_ctx(ctx)
            }
            Self::Rfc822Header(nstring) => {
                ctx.write_all(b"RFC822.HEADER ")?;
                nstring.encode_ctx(ctx)
            }
            Self::Rfc822Size(size) => write!(ctx, "RFC822.SIZE {size}"),
            Self::Rfc822Text(nstring) => {
                ctx.write_all(b"RFC822.TEXT ")?;
                nstring.encode_ctx(ctx)
            }
            Self::Uid(uid) => write!(ctx, "UID {uid}"),
            Self::Binary { section, value } => {
                ctx.write_all(b"BINARY[")?;
                join_serializable(section, b".", ctx)?;
                ctx.write_all(b"] ")?;
                value.encode_ctx(ctx)
            }
            Self::BinarySize { section, size } => {
                ctx.write_all(b"BINARY.SIZE[")?;
                join_serializable(section, b".", ctx)?;
                ctx.write_all(b"] ")?;
                size.encode_ctx(ctx)
            }
            #[cfg(feature = "ext_condstore_qresync")]
            Self::ModSeq(value) => write!(ctx, "MODSEQ {value}"),
        }
    }
}

impl EncodeIntoContext for NString<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match &self.0 {
            Some(imap_str) => imap_str.encode_ctx(ctx),
            None => ctx.write_all(b"NIL"),
        }
    }
}

impl EncodeIntoContext for NString8<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            NString8::NString(nstring) => nstring.encode_ctx(ctx),
            NString8::Literal8(literal8) => literal8.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for BodyStructure<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(b"(")?;
        match self {
            BodyStructure::Single {
                body,
                extension_data: extension,
            } => {
                body.encode_ctx(ctx)?;
                if let Some(extension) = extension {
                    ctx.write_all(b" ")?;
                    extension.encode_ctx(ctx)?;
                }
            }
            BodyStructure::Multi {
                bodies,
                subtype,
                extension_data,
            } => {
                for body in bodies.as_ref() {
                    body.encode_ctx(ctx)?;
                }
                ctx.write_all(b" ")?;
                subtype.encode_ctx(ctx)?;

                if let Some(extension) = extension_data {
                    ctx.write_all(b" ")?;
                    extension.encode_ctx(ctx)?;
                }
            }
        }
        ctx.write_all(b")")
    }
}

impl EncodeIntoContext for Body<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self.specific {
            SpecificFields::Basic {
                r#type: ref type_,
                ref subtype,
            } => {
                type_.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                subtype.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                self.basic.encode_ctx(ctx)
            }
            SpecificFields::Message {
                ref envelope,
                ref body_structure,
                number_of_lines,
            } => {
                ctx.write_all(b"\"MESSAGE\" \"RFC822\" ")?;
                self.basic.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                envelope.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                body_structure.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                write!(ctx, "{number_of_lines}")
            }
            SpecificFields::Text {
                ref subtype,
                number_of_lines,
            } => {
                ctx.write_all(b"\"TEXT\" ")?;
                subtype.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                self.basic.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                write!(ctx, "{number_of_lines}")
            }
        }
    }
}

impl EncodeIntoContext for BasicFields<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        List1AttributeValueOrNil(&self.parameter_list).encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.id.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.description.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.content_transfer_encoding.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        write!(ctx, "{}", self.size)
    }
}

impl EncodeIntoContext for Envelope<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(b"(")?;
        self.date.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.subject.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        List1OrNil(&self.from, b"").encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        List1OrNil(&self.sender, b"").encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        List1OrNil(&self.reply_to, b"").encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        List1OrNil(&self.to, b"").encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        List1OrNil(&self.cc, b"").encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        List1OrNil(&self.bcc, b"").encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.in_reply_to.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.message_id.encode_ctx(ctx)?;
        ctx.write_all(b")")
    }
}

impl EncodeIntoContext for Address<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(b"(")?;
        self.name.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.adl.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.mailbox.encode_ctx(ctx)?;
        ctx.write_all(b" ")?;
        self.host.encode_ctx(ctx)?;
        ctx.write_all(b")")?;

        Ok(())
    }
}

impl EncodeIntoContext for SinglePartExtensionData<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.md5.encode_ctx(ctx)?;

        if let Some(disposition) = &self.tail {
            ctx.write_all(b" ")?;
            disposition.encode_ctx(ctx)?;
        }

        Ok(())
    }
}

impl EncodeIntoContext for MultiPartExtensionData<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        List1AttributeValueOrNil(&self.parameter_list).encode_ctx(ctx)?;

        if let Some(disposition) = &self.tail {
            ctx.write_all(b" ")?;
            disposition.encode_ctx(ctx)?;
        }

        Ok(())
    }
}

impl EncodeIntoContext for Disposition<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match &self.disposition {
            Some((s, param)) => {
                ctx.write_all(b"(")?;
                s.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                List1AttributeValueOrNil(param).encode_ctx(ctx)?;
                ctx.write_all(b")")?;
            }
            None => ctx.write_all(b"NIL")?,
        }

        if let Some(language) = &self.tail {
            ctx.write_all(b" ")?;
            language.encode_ctx(ctx)?;
        }

        Ok(())
    }
}

impl EncodeIntoContext for Language<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        List1OrNil(&self.language, b" ").encode_ctx(ctx)?;

        if let Some(location) = &self.tail {
            ctx.write_all(b" ")?;
            location.encode_ctx(ctx)?;
        }

        Ok(())
    }
}

impl EncodeIntoContext for Location<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        self.location.encode_ctx(ctx)?;

        for body_extension in &self.extensions {
            ctx.write_all(b" ")?;
            body_extension.encode_ctx(ctx)?;
        }

        Ok(())
    }
}

impl EncodeIntoContext for BodyExtension<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            BodyExtension::NString(nstring) => nstring.encode_ctx(ctx),
            BodyExtension::Number(number) => number.encode_ctx(ctx),
            BodyExtension::List(list) => {
                ctx.write_all(b"(")?;
                join_serializable(list.as_ref(), b" ", ctx)?;
                ctx.write_all(b")")
            }
        }
    }
}

impl EncodeIntoContext for ChronoDateTime<FixedOffset> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        write!(ctx, "\"{}\"", self.format("%d-%b-%Y %H:%M:%S %z"))
    }
}

impl EncodeIntoContext for CommandContinuationRequest<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            Self::Basic(continue_basic) => match continue_basic.code() {
                Some(code) => {
                    ctx.write_all(b"+ [")?;
                    code.encode_ctx(ctx)?;
                    ctx.write_all(b"] ")?;
                    continue_basic.text().encode_ctx(ctx)?;
                    ctx.write_all(b"\r\n")
                }
                None => {
                    ctx.write_all(b"+ ")?;
                    continue_basic.text().encode_ctx(ctx)?;
                    ctx.write_all(b"\r\n")
                }
            },
            Self::Base64(data) => {
                ctx.write_all(b"+ ")?;
                ctx.write_all(base64.encode(data).as_bytes())?;
                ctx.write_all(b"\r\n")
            }
        }
    }
}

pub(crate) mod utils {
    use std::io::Write;

    use super::{EncodeContext, EncodeIntoContext};

    pub struct List1OrNil<'a, T>(pub &'a Vec<T>, pub &'a [u8]);

    pub struct List1AttributeValueOrNil<'a, T>(pub &'a Vec<(T, T)>);

    pub(crate) fn join_serializable<I: EncodeIntoContext>(
        elements: &[I],
        sep: &[u8],
        ctx: &mut EncodeContext,
    ) -> std::io::Result<()> {
        if let Some((last, head)) = elements.split_last() {
            for item in head {
                item.encode_ctx(ctx)?;
                ctx.write_all(sep)?;
            }

            last.encode_ctx(ctx)
        } else {
            Ok(())
        }
    }

    impl<T> EncodeIntoContext for List1OrNil<'_, T>
    where
        T: EncodeIntoContext,
    {
        fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
            if let Some((last, head)) = self.0.split_last() {
                ctx.write_all(b"(")?;

                for item in head {
                    item.encode_ctx(ctx)?;
                    ctx.write_all(self.1)?;
                }

                last.encode_ctx(ctx)?;

                ctx.write_all(b")")
            } else {
                ctx.write_all(b"NIL")
            }
        }
    }

    impl<T> EncodeIntoContext for List1AttributeValueOrNil<'_, T>
    where
        T: EncodeIntoContext,
    {
        fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
            if let Some((last, head)) = self.0.split_last() {
                ctx.write_all(b"(")?;

                for (attribute, value) in head {
                    attribute.encode_ctx(ctx)?;
                    ctx.write_all(b" ")?;
                    value.encode_ctx(ctx)?;
                    ctx.write_all(b" ")?;
                }

                let (attribute, value) = last;
                attribute.encode_ctx(ctx)?;
                ctx.write_all(b" ")?;
                value.encode_ctx(ctx)?;

                ctx.write_all(b")")
            } else {
                ctx.write_all(b"NIL")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use imap_types::{
        auth::AuthMechanism,
        command::{Command, CommandBody},
        core::{AString, Literal, NString, Vec1},
        fetch::MessageDataItem,
        response::{Data, Response},
        utils::escape_byte_string,
    };

    use super::*;

    #[test]
    fn test_api_encoder_usage() {
        let cmd = Command::new(
            "A",
            CommandBody::login(
                AString::from(Literal::unvalidated_non_sync(b"alice".as_ref())),
                "password",
            )
            .unwrap(),
        )
        .unwrap();

        // Dump.
        let got_encoded = CommandCodec::default().encode(&cmd).dump();

        // Encoded.
        let encoded = CommandCodec::default().encode(&cmd);

        let mut out = Vec::new();

        for x in encoded {
            match x {
                Fragment::Line { data } => {
                    println!("C: {}", escape_byte_string(&data));
                    out.extend_from_slice(&data);
                }
                Fragment::Literal { data, mode } => {
                    match mode {
                        LiteralMode::Sync => println!("C: <Waiting for continuation request>"),
                        LiteralMode::NonSync => println!("C: <Skipped continuation request>"),
                    }

                    println!("C: {}", escape_byte_string(&data));
                    out.extend_from_slice(&data);
                }
            }
        }

        assert_eq!(got_encoded, out);
    }

    #[test]
    fn test_encode_command() {
        kat_encoder::<CommandCodec, Command<'_>, &[Fragment]>(&[
            (
                Command::new("A", CommandBody::login("alice", "pass").unwrap()).unwrap(),
                [Fragment::Line {
                    data: b"A LOGIN alice pass\r\n".to_vec(),
                }]
                .as_ref(),
            ),
            (
                Command::new(
                    "A",
                    CommandBody::login("alice", b"\xCA\xFE".as_ref()).unwrap(),
                )
                .unwrap(),
                [
                    Fragment::Line {
                        data: b"A LOGIN alice {2}\r\n".to_vec(),
                    },
                    Fragment::Literal {
                        data: b"\xCA\xFE".to_vec(),
                        mode: LiteralMode::Sync,
                    },
                    Fragment::Line {
                        data: b"\r\n".to_vec(),
                    },
                ]
                .as_ref(),
            ),
            (
                Command::new("A", CommandBody::authenticate(AuthMechanism::Login)).unwrap(),
                [Fragment::Line {
                    data: b"A AUTHENTICATE LOGIN\r\n".to_vec(),
                }]
                .as_ref(),
            ),
            (
                Command::new(
                    "A",
                    CommandBody::authenticate_with_ir(AuthMechanism::Login, b"alice".as_ref()),
                )
                .unwrap(),
                [Fragment::Line {
                    data: b"A AUTHENTICATE LOGIN YWxpY2U=\r\n".to_vec(),
                }]
                .as_ref(),
            ),
            (
                Command::new("A", CommandBody::authenticate(AuthMechanism::Plain)).unwrap(),
                [Fragment::Line {
                    data: b"A AUTHENTICATE PLAIN\r\n".to_vec(),
                }]
                .as_ref(),
            ),
            (
                Command::new(
                    "A",
                    CommandBody::authenticate_with_ir(
                        AuthMechanism::Plain,
                        b"\x00alice\x00pass".as_ref(),
                    ),
                )
                .unwrap(),
                [Fragment::Line {
                    data: b"A AUTHENTICATE PLAIN AGFsaWNlAHBhc3M=\r\n".to_vec(),
                }]
                .as_ref(),
            ),
        ]);
    }

    #[test]
    fn test_encode_response() {
        kat_encoder::<ResponseCodec, Response<'_>, &[Fragment]>(&[
            (
                Response::Data(Data::Fetch {
                    seq: NonZeroU32::new(12345).unwrap(),
                    items: Vec1::from(MessageDataItem::BodyExt {
                        section: None,
                        origin: None,
                        data: NString::from(Literal::unvalidated(b"ABCDE".as_ref())),
                    }),
                }),
                [
                    Fragment::Line {
                        data: b"* 12345 FETCH (BODY[] {5}\r\n".to_vec(),
                    },
                    Fragment::Literal {
                        data: b"ABCDE".to_vec(),
                        mode: LiteralMode::Sync,
                    },
                    Fragment::Line {
                        data: b")\r\n".to_vec(),
                    },
                ]
                .as_ref(),
            ),
            (
                Response::Data(Data::Fetch {
                    seq: NonZeroU32::new(12345).unwrap(),
                    items: Vec1::from(MessageDataItem::BodyExt {
                        section: None,
                        origin: None,
                        data: NString::from(Literal::unvalidated_non_sync(b"ABCDE".as_ref())),
                    }),
                }),
                [
                    Fragment::Line {
                        data: b"* 12345 FETCH (BODY[] {5+}\r\n".to_vec(),
                    },
                    Fragment::Literal {
                        data: b"ABCDE".to_vec(),
                        mode: LiteralMode::NonSync,
                    },
                    Fragment::Line {
                        data: b")\r\n".to_vec(),
                    },
                ]
                .as_ref(),
            ),
        ])
    }

    fn kat_encoder<'a, E, M, F>(tests: &'a [(M, F)])
    where
        E: Encoder<Message<'a> = M> + Default,
        F: AsRef<[Fragment]>,
    {
        for (i, (obj, actions)) in tests.iter().enumerate() {
            println!("# Testing {i}");

            let encoder = E::default().encode(obj);
            let actions = actions.as_ref();

            assert_eq!(encoder.collect::<Vec<_>>(), actions);
        }
    }
}

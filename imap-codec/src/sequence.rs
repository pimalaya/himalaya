use imap_types::{
    core::Vec1,
    sequence::{SeqOrUid, Sequence, SequenceSet},
};
use nom::{
    branch::alt,
    bytes::streaming::tag,
    combinator::{map, value},
    multi::separated_list1,
    sequence::tuple,
};

use crate::{core::nz_number, decode::IMAPResult};

/// `sequence-set = (seq-number / seq-range) ["," sequence-set]`
///
/// Note: See errata id: 261 TODO: Why the errata?
///
/// Set of seq-number values, regardless of order.
/// Servers MAY coalesce overlaps and/or execute the sequence in any order.
///
/// Example: a message sequence number set of
/// 2,4:7,9,12:* for a mailbox with 15 messages is
/// equivalent to 2,4,5,6,7,9,12,13,14,15
///
/// Example: a message sequence number set of *:4,5:7
/// for a mailbox with 10 messages is equivalent to
/// 10,9,8,7,6,5,4,5,6,7 and MAY be reordered and
/// overlap coalesced to be 4,5,6,7,8,9,10.
///
/// Simplified:
///
/// `sequence-set = (seq-number / seq-range) *("," (seq-number / seq-range))`
pub(crate) fn sequence_set(input: &[u8]) -> IMAPResult<&[u8], SequenceSet> {
    map(
        separated_list1(
            tag(b","),
            alt((
                // Ordering is important!
                map(seq_range, |(from, to)| Sequence::Range(from, to)),
                map(seq_number, Sequence::Single),
            )),
        ),
        |set| SequenceSet(Vec1::unvalidated(set)),
    )(input)
}

/// `seq-range = seq-number ":" seq-number`
///
/// Two seq-number values and all values between these two regardless of order.
///
/// Example: 2:4 and 4:2 are equivalent and indicate values 2, 3, and 4.
///
/// Example: a unique identifier sequence range of 3291:* includes the UID
///          of the last message in the mailbox, even if that value is less than 3291.
pub(crate) fn seq_range(input: &[u8]) -> IMAPResult<&[u8], (SeqOrUid, SeqOrUid)> {
    let mut parser = tuple((seq_number, tag(b":"), seq_number));

    let (remaining, (from, _, to)) = parser(input)?;

    Ok((remaining, (from, to)))
}

/// `seq-number = nz-number / "*"`
///
/// Message sequence number (COPY, FETCH, STORE commands) or unique
/// identifier (UID COPY, UID FETCH, UID STORE commands).
///
/// "*" represents the largest number in use.
/// In the case of message sequence numbers, it is the number of messages in a non-empty mailbox.
/// In the case of unique identifiers, it is the unique identifier of the last message in the mailbox or,
/// if the mailbox is empty, the mailbox's current UIDNEXT value.
///
/// The server should respond with a tagged BAD response to a command that uses a message
/// sequence number greater than the number of messages in the selected mailbox.
/// This includes "*" if the selected mailbox is empty.
pub(crate) fn seq_number(input: &[u8]) -> IMAPResult<&[u8], SeqOrUid> {
    alt((
        map(nz_number, SeqOrUid::Value),
        value(SeqOrUid::Asterisk, tag(b"*")),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{EncodeContext, EncodeIntoContext};

    #[test]
    fn test_encode_of_some_sequence_sets() {
        let tests = [
            (
                Sequence::Single(SeqOrUid::Value(1.try_into().unwrap())),
                b"1".as_ref(),
            ),
            (Sequence::Single(SeqOrUid::Asterisk), b"*".as_ref()),
            (
                Sequence::Range(SeqOrUid::Value(1.try_into().unwrap()), SeqOrUid::Asterisk),
                b"1:*".as_ref(),
            ),
        ];

        for (test, expected) in tests {
            let mut ctx = EncodeContext::new();
            test.encode_ctx(&mut ctx).unwrap();

            let out = ctx.dump();
            assert_eq!(*expected, out);
        }
    }

    #[test]
    fn test_parse_sequence_set() {
        let (rem, val) = sequence_set(b"1:*?").unwrap();
        println!("{rem:?}, {val:?}");

        let (rem, val) = sequence_set(b"1:*,5?").unwrap();
        println!("{rem:?}, {val:?}");
    }

    #[test]
    fn test_parse_seq_number() {
        // Must not be 0.
        assert!(seq_number(b"0?").is_err());

        let (rem, val) = seq_number(b"1?").unwrap();
        println!("{rem:?}, {val:?}");

        let (rem, val) = seq_number(b"*?").unwrap();
        println!("{rem:?}, {val:?}");
    }

    #[test]
    fn test_parse_seq_range() {
        // Must not be 0.
        assert!(seq_range(b"0:1?").is_err());

        assert_eq!(
            (
                SeqOrUid::Value(1.try_into().unwrap()),
                SeqOrUid::Value(2.try_into().unwrap())
            ),
            seq_range(b"1:2?").unwrap().1
        );
        assert_eq!(
            (SeqOrUid::Value(1.try_into().unwrap()), SeqOrUid::Asterisk),
            seq_range(b"1:*?").unwrap().1
        );
        assert_eq!(
            (SeqOrUid::Asterisk, SeqOrUid::Value(10.try_into().unwrap())),
            seq_range(b"*:10?").unwrap().1
        );
    }
}

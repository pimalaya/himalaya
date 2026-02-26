use std::io::Write;

use abnf_core::streaming::sp;
use imap_types::{
    command::CommandBody,
    core::{Vec1, Vec2},
    extensions::thread::{Thread, ThreadingAlgorithm, ThreadingAlgorithmOther},
    response::Data,
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, opt},
    multi::{many_m_n, many1, separated_list1},
    sequence::{delimited, preceded, tuple},
};

use crate::{
    core::{atom, nz_number},
    decode::{IMAPErrorKind, IMAPParseError, IMAPResult},
    encode::{EncodeContext, EncodeIntoContext},
    search::search_criteria,
};

impl EncodeIntoContext for Thread {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.to_string().as_bytes())
    }
}

impl EncodeIntoContext for ThreadingAlgorithm<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        match self {
            ThreadingAlgorithm::OrderedSubject => ctx.write_all(b"ORDEREDSUBJECT"),
            ThreadingAlgorithm::References => ctx.write_all(b"REFERENCES"),
            ThreadingAlgorithm::Other(other) => other.encode_ctx(ctx),
        }
    }
}

impl EncodeIntoContext for ThreadingAlgorithmOther<'_> {
    fn encode_ctx(&self, ctx: &mut EncodeContext) -> std::io::Result<()> {
        ctx.write_all(self.as_ref().as_bytes())
    }
}

/// ```abnf
/// thread = ["UID" SP] "THREAD" SP thread-alg SP search-criteria
/// ```
pub(crate) fn thread(input: &[u8]) -> IMAPResult<&[u8], CommandBody> {
    let mut parser = tuple((
        map(opt(tag_no_case("UID ")), |thing| thing.is_some()),
        tag_no_case("THREAD "),
        thread_alg,
        sp,
        search_criteria,
    ));

    let (remaining, (uid, _, algorithm, _, (charset, search_key))) = parser(input)?;

    Ok((
        remaining,
        CommandBody::Thread {
            algorithm,
            charset,
            search_criteria: search_key,
            uid,
        },
    ))
}

/// ```abnf
/// thread-alg = "ORDEREDSUBJECT" / "REFERENCES" / thread-alg-ext
///
/// thread-alg-ext = atom
/// ```
pub(crate) fn thread_alg(input: &[u8]) -> IMAPResult<&[u8], ThreadingAlgorithm> {
    map(atom, ThreadingAlgorithm::from)(input)
}

/// ```abnf
/// thread-data = "THREAD" [SP 1*thread-list]
/// ```
pub(crate) fn thread_data(input: &[u8]) -> IMAPResult<&[u8], Data> {
    let mut parser = preceded(
        tag_no_case("THREAD"),
        opt(preceded(sp, many1(thread_list(8)))),
    );

    let (remaining, thread_list) = parser(input)?;

    Ok((remaining, Data::Thread(thread_list.unwrap_or_default())))
}

pub(crate) fn thread_list(
    remaining_recursions: usize,
) -> impl Fn(&[u8]) -> IMAPResult<&[u8], Thread> {
    move |input: &[u8]| thread_list_limited(input, remaining_recursions)
}

/// ```abnf
/// thread-list = "(" (thread-members / thread-nested) ")"
///
/// thread-members = nz-number *(SP nz-number) [SP thread-nested]
///
/// thread-nested = 2*thread-list
/// ```
///
/// ```abnf
/// // Simplified
/// thread-list = "("
///                 (
///                  nz-number *(SP nz-number) [SP 2*thread-list] /
///                                                2*thread-list
///                 )
///               ")"
/// ```
pub(crate) fn thread_list_limited(
    input: &[u8],
    remaining_recursion: usize,
) -> IMAPResult<&[u8], Thread> {
    if remaining_recursion == 0 {
        return Err(nom::Err::Failure(IMAPParseError {
            input,
            kind: IMAPErrorKind::RecursionLimitExceeded,
        }));
    }

    let thread_list = |input| thread_list_limited(input, remaining_recursion.saturating_sub(1));

    let mut parser = delimited(
        tag("("),
        alt((
            map(
                tuple((
                    separated_list1(sp, nz_number),
                    opt(preceded(sp, many_m_n(2, usize::MAX, thread_list))),
                )),
                |(prefix, answers)| Thread::Members {
                    prefix: Vec1::unvalidated(prefix),
                    answers: answers.map(Vec2::unvalidated),
                },
            ),
            map(many_m_n(2, usize::MAX, thread_list), |vec| Thread::Nested {
                answers: Vec2::unvalidated(vec),
            }),
        )),
        tag(")"),
    );

    let (rem, out) = parser(input)?;

    Ok((rem, out))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use imap_types::core::{Vec1, Vec2};

    use super::{Thread, thread_list};

    #[test]
    fn test_thread_list() {
        let tests: &[(&str, Thread)] = &[
            (
                "(1)",
                Thread::Members {
                    prefix: Vec1::from(NonZeroU32::new(1).unwrap()),
                    answers: None,
                },
            ),
            (
                "(1 2)",
                Thread::Members {
                    prefix: Vec1::try_from(vec![
                        NonZeroU32::new(1).unwrap(),
                        NonZeroU32::new(2).unwrap(),
                    ])
                    .unwrap(),
                    answers: None,
                },
            ),
            (
                "((1)(2))",
                Thread::Nested {
                    answers: Vec2::try_from(vec![
                        Thread::Members {
                            prefix: Vec1::from(NonZeroU32::new(1).unwrap()),
                            answers: None,
                        },
                        Thread::Members {
                            prefix: Vec1::from(NonZeroU32::new(2).unwrap()),
                            answers: None,
                        },
                    ])
                    .unwrap(),
                },
            ),
            (
                "(1 (2)(3))",
                Thread::Members {
                    prefix: Vec1::try_from(vec![NonZeroU32::new(1).unwrap()]).unwrap(),
                    answers: Some(
                        Vec2::try_from(vec![
                            Thread::Members {
                                prefix: Vec1::from(NonZeroU32::new(2).unwrap()),
                                answers: None,
                            },
                            Thread::Members {
                                prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                                answers: None,
                            },
                        ])
                        .unwrap(),
                    ),
                },
            ),
            (
                "(1 (2 4)(3))",
                Thread::Members {
                    prefix: Vec1::try_from(vec![NonZeroU32::new(1).unwrap()]).unwrap(),
                    answers: Some(
                        Vec2::try_from(vec![
                            Thread::Members {
                                prefix: Vec1::try_from(vec![
                                    NonZeroU32::new(2).unwrap(),
                                    NonZeroU32::new(4).unwrap(),
                                ])
                                .unwrap(),
                                answers: None,
                            },
                            Thread::Members {
                                prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                                answers: None,
                            },
                        ])
                        .unwrap(),
                    ),
                },
            ),
            (
                "(1 (2 4 (5)(6))(3))",
                Thread::Members {
                    prefix: Vec1::try_from(vec![NonZeroU32::new(1).unwrap()]).unwrap(),
                    answers: Some(
                        Vec2::try_from(vec![
                            Thread::Members {
                                prefix: Vec1::try_from(vec![
                                    NonZeroU32::new(2).unwrap(),
                                    NonZeroU32::new(4).unwrap(),
                                ])
                                .unwrap(),
                                answers: Some(
                                    Vec2::try_from(vec![
                                        Thread::Members {
                                            prefix: Vec1::from(NonZeroU32::new(5).unwrap()),
                                            answers: None,
                                        },
                                        Thread::Members {
                                            prefix: Vec1::from(NonZeroU32::new(6).unwrap()),
                                            answers: None,
                                        },
                                    ])
                                    .unwrap(),
                                ),
                            },
                            Thread::Members {
                                prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                                answers: None,
                            },
                        ])
                        .unwrap(),
                    ),
                },
            ),
            (
                "(1 (2)(3)(((4)(5))(6)))",
                Thread::Members {
                    prefix: Vec1::from(NonZeroU32::new(1).unwrap()),
                    answers: Some(
                        Vec2::try_from(vec![
                            Thread::Members {
                                prefix: Vec1::from(NonZeroU32::new(2).unwrap()),
                                answers: None,
                            },
                            Thread::Members {
                                prefix: Vec1::from(NonZeroU32::new(3).unwrap()),
                                answers: None,
                            },
                            Thread::Nested {
                                answers: Vec2::try_from(vec![
                                    Thread::Nested {
                                        answers: Vec2::try_from(vec![
                                            Thread::Members {
                                                prefix: Vec1::from(NonZeroU32::new(4).unwrap()),
                                                answers: None,
                                            },
                                            Thread::Members {
                                                prefix: Vec1::from(NonZeroU32::new(5).unwrap()),
                                                answers: None,
                                            },
                                        ])
                                        .unwrap(),
                                    },
                                    Thread::Members {
                                        prefix: Vec1::from(NonZeroU32::new(6).unwrap()),
                                        answers: None,
                                    },
                                ])
                                .unwrap(),
                            },
                        ])
                        .unwrap(),
                    ),
                },
            ),
        ];

        for (test, expected) in tests {
            println!("test:     {test}");
            println!("expected: {expected}\n");
            assert_eq!(*test, expected.to_string().as_str());

            let (rem, _data) = thread_list(8)(test.as_bytes()).unwrap();
            assert!(rem.is_empty());
        }
    }
}

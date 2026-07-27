---
cairn: log
change: raw-command-batch
landed: 2026-07-27
---

# Byte-verbatim raw command passthrough

Made `imap raw` and `smtp raw` forward their command bytes to the server verbatim, and taught `imap raw` to pipeline a whole batch. A raw command typed on the shell arrives with backslash escapes left literal (`a1 NOOP\r\n` reaches the process as the bytes `\` `r` `\` `n`, not a real CRLF), so both commands now resolve their argument through a shared `RawCommandArg` (in shared/raw.rs, mirroring `MessageArg`) that strips `\r` literals and turns `\n` literals into real CRLF, accepting the argument positionally or from stdin.

The underlying io-imap `ImapRaw` was reworked upstream to be a byte-verbatim batch passthrough: it no longer injects a tag or trims/appends CRLF, it parses the input to collect every command's tag, and it reads until all of them are acknowledged, tolerating out-of-order tagged completions (RFC 3501 §5.5). `imap raw` therefore sends a batch of caller-tagged commands separated by CRLF and appends a trailing CRLF when the last one omits it. `smtp raw` keeps io-smtp's single-command/single-reply model: it strips the trailing CRLF io-smtp adds itself and rejects a multi-line batch that would desync the reply parser.

Touched the commands capability (new "Raw passthrough is byte-verbatim" requirement). The io-imap change is a breaking rework of `ImapRaw` / `ImapClientStd::raw` (now `impl AsRef<[u8]>`, fallible `new`, new `ImapRawError` variants), pending an io-imap release before Himalaya can drop the published-crate path and adopt it.

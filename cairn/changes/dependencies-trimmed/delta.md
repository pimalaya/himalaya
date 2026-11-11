---
cairn: delta
change: dependencies-trimmed
---

# Delta

## ADDED Requirements

### Requirement: Hosts are ASCII
(packaging) Himalaya SHALL build url's IDNA without a Unicode back end, `idna_adapter` being pinned to 1.0.0 in Cargo.toml. A host SHALL be written in ASCII, an internationalised one in its Punycode form (`xn--…`), and an `http` or `https` URL with a non-ASCII host (a JMAP server, an HTTP proxy) SHALL be refused when parsed. IMAP, SMTP and ManageSieve servers are URLs of schemes url keeps opaque, which never reach IDNA: their host is written in ASCII as well.

#### Scenario: An internationalised JMAP server
- GIVEN `jmap.server = "https://bücher.example/jmap"`
- WHEN the account connects
- THEN the server is refused before any connection, and the same account connects with `jmap.server = "https://xn--bcher-kva.example/jmap"`

### Requirement: Dependencies name their features
(packaging) Every dependency, build and dev ones included, SHALL be declared with `default-features = false` and the features Himalaya uses, so a default turned on upstream never reaches the binary. A feature only one backend needs SHALL be turned on by that backend's cargo feature: pimalaya-cli's `prompt`, which `pimdir queue cancel` alone asks through, by `pimdir`. A dependency nothing uses SHALL be removed, and a crate the tests alone use SHALL be a dev-dependency.

### Requirement: Dependency checks pass clean
(packaging) `cargo deny check` SHALL pass over the targets deny.toml lists, which are the release targets and the ones people build from source, allowing only the licences the tree carries. A duplicate the tree cannot avoid stays a visible warning rather than a skip: today getrandom 0.2 (ring 0.17), syn 2 (bounded-static-derive under io-imap and io-smtp, windows-core under chrono on Windows), and hashbrown 0.15 with foldhash 0.1 (chumsky 0.13).

### Requirement: An IMAP envelope reads its strings as a header
(backends) The subject and display names of an IMAP `ENVELOPE`, as `envelope list`, `envelope search`, `imap fetch` and `imap thread` print them, SHALL be read as `message read` reads a header (commands: "Malformed mail reads as its sender meant"): raw 8-bit bytes as UTF-8 when they are valid UTF-8 and as Windows-1252 otherwise, line breaks dropped, adjacent RFC 2047 encoded words of one charset joined as bytes before their charset is decoded, and UTF-7 read as written.

#### Scenario: An emoji split across two encoded words
- GIVEN an IMAP message whose `ENVELOPE` subject is `=?UTF-8?Q?=F0=9F?= =?UTF-8?Q?=98=82?=`
- WHEN `envelope list` prints it
- THEN its subject is 😂, as `message read` shows it

## MODIFIED Requirements

None.

## REMOVED Requirements

None.

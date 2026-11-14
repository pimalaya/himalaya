---
cairn: delta
change: the-sample-lists-values-in-brackets
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: One sample configuration, with a section per backend
(config) The repository SHALL ship config.sample.toml holding one `[accounts.example]` table with dotted keys and no other table header, every line of code ending in one tab-aligned comment, every password or token command a failing `exit 127` placeholder, and no OAuth broker nor sync engine named. Every line SHALL be commented but `default = true`: a key with a default showing it, a key without one an example, the blocks of every backend (IMAP, SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph, Maildir, mbox, pimdir) included, each headed by its name. The other values of a key that is neither a boolean nor free text SHALL end its comment in brackets (`[bottom, none]`). A structural variant SHALL be one more commented line setting the same field (`password.raw` after `password.command`), and a paragraph setting keys under the same two-segment table as the paragraph above SHALL replace it (XOAUTH2 for PLAIN). No line SHALL be commented twice, and the TLS, proxy and ALPN keys SHALL be written once. The test suite SHALL parse config.sample.toml through the loader `-c` uses as written, then once per rank with every line uncommented, each field set to its value of that rank (the lines setting it in their order, each followed by its bracketed values, the last when it has fewer), a replacing paragraph left out at rank 1 and in place of the one above from rank 2, resolving `example` as the default account each time, so a configuration change breaking one fails the build.

#### Scenario: A field is renamed
- GIVEN config.sample.toml, whose commented `#imap.starttls = false` line names a field
- WHEN the field is renamed in the configuration types and the sample is not updated
- THEN the test suite fails parsing the first variant

## REMOVED Requirements

None.

---
cairn: delta
change: the-sample-is-tested-and-shipped
---

# Delta

## ADDED Requirements

### Requirement: One sample configuration, with a section per backend
(config) The repository SHALL ship config.sample.toml holding one `[accounts.example]` table with dotted keys and no other table header. It SHALL list the options every account shares and each of their values, the default active and the others commented as `#key = value`, every line ending in one tab-aligned comment, and every password command a failing `exit 127` placeholder. It SHALL end with one section per backend or provider (IMAP and SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph, Microsoft IMAP and SMTP, Fastmail, Maildir, m2dir, mbox, pimdir), each a paragraph of commented code headed by its name, its own alternatives commented twice, an OAuth 2.0 token read through `ortie token show -a example`, so the file holds a working account once one section is uncommented. The test suite SHALL parse config.sample.toml through the loader `-c` uses with each section uncommented whole, then on top of it each line commented twice inside it and each line of commented code outside the sections, in place of the active lines of its field, and SHALL resolve `example` as the default account each time, so a configuration change breaking one fails the build.

#### Scenario: A field is renamed
- GIVEN config.sample.toml, whose IMAP and SMTP section sets `imap.sort.fallback`
- WHEN the field is renamed in the configuration types and the sample is not updated
- THEN the test suite fails naming the sample and the line uncommented

### Requirement: The sample ships beside the binary
(packaging) The Nix package SHALL install config.sample.toml as share/doc/himalaya/config.sample.toml, the release archives SHALL carry it with the share/ of the build they come from, and install.sh SHALL copy it to the same path under its prefix when the downloaded archive holds it, so the first-run welcome copies it rather than downloading it.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
